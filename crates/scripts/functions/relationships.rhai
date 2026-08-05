// Relationship candidates (fixture 12): the high-recall half of the
// candidate -> verified -> declared arc. Runs at dataset grain
// (`SELECT detect_relationships() FROM fin`) and measures every
// plausible join pair across the landed tables — generous by design
// (ruled 2026-08-05): the statistical pass optimizes recall, and the
// judge reading this measurement removes the false positives against
// the data; this script never does. Core fields (from, to,
// cardinality, overlap) answer the aspect schema; the rest of the
// evidence rides its open remainder. Cost: one LIMIT 0 and one
// aggregate scan per table, one distinct-join per overlapping pair,
// a handful of queries per attempted composite rescue.

let tables = [];
let names = db.query("SELECT DISTINCT table_name FROM imports ORDER BY table_name")
    .col("table_name");
for i in 0..names.count() {
    tables.push(names.value_at(i));
}

// Per-column shape: filled and distinct counts decide which columns
// look like keys (the join's `to` side) — near-unique, not strictly
// unique, so dirty keys stay in the running.
let cols = [];
for t in tables {
    let head = db.query(`SELECT * FROM "${t}" LIMIT 0`);
    let select = "";
    for c in head.columns() {
        if select != "" { select += ", "; }
        select += `count("${c}") AS "f_${c}", count(DISTINCT "${c}") AS "d_${c}"`;
    }
    if select == "" { continue; }
    let agg = db.query(`SELECT ${select} FROM "${t}"`);
    for c in head.columns() {
        // value_at serves display text; counts read back as ints.
        let filled = agg.col(`f_${c}`).value_at(0).parse_int();
        let distinct = agg.col(`d_${c}`).value_at(0).parse_int();
        cols.push(#{
            table: t, column: c, dtype: head.col(c).dtype(),
            filled: filled, distinct: distinct,
            unique: filled > 0 && distinct == filled,
            key_like: filled > 0 && distinct >= 2
                && distinct.to_float() / filled.to_float() >= 0.9,
        });
    }
}

// Every type-compatible pair whose values overlap, keys or not — the
// non-key pairs are the raw material for composite rescue. Same-table
// pairs stay in: a parent column pointing at its own table's key is a
// hierarchy candidate.
let pairs = [];
for k in cols {
    if k.distinct < 2 { continue; }
    for f in cols {
        if f.table == k.table && f.column == k.column { continue; }
        if f.dtype != k.dtype { continue; }
        if f.distinct == 0 { continue; }
        let m = db.query(
            `SELECT count(*) AS m FROM
             (SELECT DISTINCT "${f.column}" AS v FROM "${f.table}"
              WHERE "${f.column}" IS NOT NULL) a
             JOIN (SELECT DISTINCT "${k.column}" AS v FROM "${k.table}"
              WHERE "${k.column}" IS NOT NULL) b ON a.v = b.v`,
        ).col("m").value_at(0).parse_int();
        let overlap = m.to_float() / f.distinct.to_float();
        if overlap < 0.5 { continue; }
        pairs.push(#{ f: f, k: k, overlap: overlap, matched: m });
    }
}

// Single-column candidates: the pairs whose to side stands as a key
// on its own.
let candidates = [];
for p in pairs {
    if !p.k.key_like { continue; }
    candidates.push(#{
        from: `${p.f.table}.${p.f.column}`,
        to: `${p.k.table}.${p.k.column}`,
        cardinality: if p.f.unique { "one-to-one" } else { "many-to-one" },
        overlap: p.overlap,
        matched: p.matched,
        orphans: p.f.distinct - p.matched,
        from_distinct: p.f.distinct,
        to_distinct: p.k.distinct,
    });
}

// Composite rescue (the v0.3 reality, ported 2026-08-05): a to side
// that is no key alone can be one inside a scope — the multi-tenant
// shape, (businessID, name). For each overlapping pair whose to side
// is not key-like, try the co-present pairs between the same two
// tables as the scoping leg; the first whose combination makes the to
// side near-unique and whose two-leg join resolves rescues the
// anchor. Width 2 only — anchor plus one scope — wider composites
// stay future work. Data decides, not names.
let seen = [];
for p in pairs {
    if p.k.key_like { continue; }
    let scopes = [];
    for s in pairs {
        if s.f.table != p.f.table || s.k.table != p.k.table { continue; }
        if s.f.column == p.f.column || s.k.column == p.k.column { continue; }
        scopes.push(s);
    }
    scopes.sort(|a, b| if a.overlap < b.overlap { 1 } else if a.overlap > b.overlap { -1 } else { 0 });
    for s in scopes {
        let key = `${p.f.table}|${p.k.table}|${p.f.column}+${s.f.column}|${p.k.column}+${s.k.column}`;
        let mirror = `${p.f.table}|${p.k.table}|${s.f.column}+${p.f.column}|${s.k.column}+${p.k.column}`;
        if seen.contains(key) || seen.contains(mirror) { continue; }

        let kt = p.k.table;
        let both = `"${p.k.column}" IS NOT NULL AND "${s.k.column}" IS NOT NULL`;
        let filled = db.query(`SELECT count(*) AS c FROM "${kt}" WHERE ${both}`)
            .col("c").value_at(0).parse_int();
        let dst = db.query(
            `SELECT count(*) AS c FROM
             (SELECT DISTINCT "${p.k.column}", "${s.k.column}" FROM "${kt}" WHERE ${both})`,
        ).col("c").value_at(0).parse_int();
        if filled == 0 || dst < 2 || dst.to_float() / filled.to_float() < 0.9 { continue; }

        let ft = p.f.table;
        let fboth = `"${p.f.column}" IS NOT NULL AND "${s.f.column}" IS NOT NULL`;
        let ffilled = db.query(`SELECT count(*) AS c FROM "${ft}" WHERE ${fboth}`)
            .col("c").value_at(0).parse_int();
        let fpairs = db.query(
            `SELECT count(*) AS c FROM
             (SELECT DISTINCT "${p.f.column}", "${s.f.column}" FROM "${ft}" WHERE ${fboth})`,
        ).col("c").value_at(0).parse_int();
        if fpairs == 0 { continue; }
        let m = db.query(
            `SELECT count(*) AS m FROM
             (SELECT DISTINCT "${p.f.column}" AS v1, "${s.f.column}" AS v2
              FROM "${ft}" WHERE ${fboth}) a
             JOIN (SELECT DISTINCT "${p.k.column}" AS v1, "${s.k.column}" AS v2
              FROM "${kt}" WHERE ${both}) b
             ON a.v1 = b.v1 AND a.v2 = b.v2`,
        ).col("m").value_at(0).parse_int();
        let overlap = m.to_float() / fpairs.to_float();
        if overlap < 0.5 { continue; }

        seen.push(key);
        // The anchor is the identifying leg — the higher-cardinality to
        // side; the scope is the tenant leg. Which pair triggered the
        // rescue is iteration order, not evidence.
        let a = p;
        let sc = s;
        if s.k.distinct > p.k.distinct {
            a = s;
            sc = p;
        }
        candidates.push(#{
            from: `${a.f.table}.${a.f.column}`,
            to: `${a.k.table}.${a.k.column}`,
            key_columns: [#{
                from: `${sc.f.table}.${sc.f.column}`,
                to: `${sc.k.table}.${sc.k.column}`,
            }],
            cardinality: if fpairs == ffilled { "one-to-one" } else { "many-to-one" },
            overlap: overlap,
            matched: m,
            orphans: fpairs - m,
            from_distinct: fpairs,
            to_distinct: dst,
        });
        break;
    }
}

candidates.sort(|a, b| if a.overlap < b.overlap {
    1
} else if a.overlap > b.overlap {
    -1
} else {
    0
});

#{ candidates: candidates }
