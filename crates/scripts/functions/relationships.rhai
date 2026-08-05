// Relationship candidates (fixture 12): the high-recall half of the
// candidate -> verified -> declared arc. Runs at dataset grain
// (`SELECT detect_relationships() FROM fin`) and measures every
// plausible join pair across the landed tables — generous by design
// (ruled 2026-08-05): the statistical pass optimizes recall, and the
// judge reading this measurement removes the false positives against
// the data; this script never does. Core fields (from, to,
// cardinality, overlap) answer the aspect schema; the rest of the
// evidence rides its open remainder.
//
// The algorithm (2026-08-06, replacing a per-pair SQL join that was
// quadratic in engine round-trips): inclusion-dependency discovery on
// the SPIDER/SINDY shape — every candidate column's distinct values
// land ONCE as a sorted typed key vector (`key_vec`, engine-deduped
// DISTINCT with projection pushdown, batched through `query_all`), and
// containment between two columns is a linear merge in the kernel
// (`matched`). The statistic is containment — matched over the from
// side's distinct count — the join-discovery statistic (Jaccard
// punishes exactly the size-skewed pairs real FKs are). The only
// pruning is algebra: a to side with fewer than half the from side's
// distinct values cannot reach the 0.5 bar, so its comparison is
// skipped with zero recall lost. Exact while Σ distinct fits memory;
// past that the named ladder is BINDER-style hash-range partitioning
// and bottom-k sketches — not built until a dataset needs them.
//
// Cost: one LIMIT 0 and one aggregate scan per table, one batched
// DISTINCT per candidate column, in-memory merges for every pair; the
// composite rescue adds two batched probe phases and kernel merges.

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

// Every type-compatible pair, keys or not — the non-key pairs are the
// raw material for composite rescue. Same-table pairs stay in: a
// parent column pointing at its own table's key is a hierarchy
// candidate.
let specs = [];
for k in cols {
    if k.distinct < 2 { continue; }
    for f in cols {
        if f.table == k.table && f.column == k.column { continue; }
        if f.dtype != k.dtype { continue; }
        if f.distinct == 0 { continue; }
        // Implied by the acceptance bar, never a heuristic: overlap =
        // matched / from_distinct, and matched ≤ to_distinct — a to
        // side under half the from side's distinct count cannot reach
        // 0.5, so its comparison would be wasted.
        if k.distinct.to_float() < 0.5 * f.distinct.to_float() { continue; }
        specs.push(#{ f: f, k: k });
    }
}

// One DISTINCT per involved column, batched — the only scans the pair
// phase needs. Kernel key vectors from each.
let involved = [];
let dsqls = [];
for s in specs {
    for side in [s.f, s.k] {
        let key = `${side.table}|${side.column}`;
        if involved.contains(key) { continue; }
        involved.push(key);
        dsqls.push(`SELECT DISTINCT "${side.column}" AS v FROM "${side.table}"
                    WHERE "${side.column}" IS NOT NULL`);
    }
}
let danswers = db.query_all(dsqls);
let keys = #{};
for i in 0..involved.len() {
    keys[involved[i]] = danswers[i].col("v").key_vec();
}

// Pair mathematics, entirely in memory.
let pairs = [];
for s in specs {
    let m = keys[`${s.f.table}|${s.f.column}`]
        .matched(keys[`${s.k.table}|${s.k.column}`]);
    let overlap = m.to_float() / s.f.distinct.to_float();
    if overlap < 0.5 { continue; }
    pairs.push(#{ f: s.f, k: s.k, overlap: overlap, matched: m });
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
// tables as the scoping leg; the first (in overlap order) whose
// combination makes the to side near-unique and whose two-leg
// intersection resolves rescues the anchor. Width 2 only — anchor plus
// one scope — wider composites stay future work. Data decides, not
// names.
let attempts = [];
let seen = [];
for pi in 0..pairs.len() {
    let p = pairs[pi];
    if p.k.key_like { continue; }
    let scopes = [];
    for s in pairs {
        if s.f.table != p.f.table || s.k.table != p.k.table { continue; }
        if s.f.column == p.f.column || s.k.column == p.k.column { continue; }
        scopes.push(s);
    }
    scopes.sort(|a, b| if a.overlap < b.overlap { 1 } else if a.overlap > b.overlap { -1 } else { 0 });
    let order = 0;
    for s in scopes {
        // Implied by the near-unique bar, never a heuristic: the
        // combined to side's distinct pairs cannot exceed the product
        // of the legs' distinct counts — a product under 0.9 of the
        // co-filled floor can never key the table, so probing it is
        // wasted (this is what dense small-int columns proposing each
        // other as scopes needed).
        let floor = if p.k.filled < s.k.filled { p.k.filled } else { s.k.filled };
        if p.k.distinct.to_float() * s.k.distinct.to_float() < 0.9 * floor.to_float() {
            continue;
        }
        let key = `${p.f.table}|${p.k.table}|${p.f.column}+${s.f.column}|${p.k.column}+${s.k.column}`;
        let mirror = `${p.f.table}|${p.k.table}|${s.f.column}+${p.f.column}|${s.k.column}+${p.k.column}`;
        if seen.contains(key) || seen.contains(mirror) { continue; }
        seen.push(key);
        attempts.push(#{ anchor: pi, p: p, s: s, order: order });
        order += 1;
    }
}

// Phase 1, batched: is the combined to side a key inside the scope?
// The same combination backs many attempts — probed once; the pair
// domain comes back as rows and becomes kernel keys.
let to_keys = [];
let to_sqls = [];
for a in attempts {
    let kt = a.p.k.table;
    let key = `${kt}|${a.p.k.column}|${a.s.k.column}`;
    if to_keys.contains(key) { continue; }
    to_keys.push(key);
    let both = `"${a.p.k.column}" IS NOT NULL AND "${a.s.k.column}" IS NOT NULL`;
    to_sqls.push(`SELECT count(*) AS c FROM "${kt}" WHERE ${both}`);
    to_sqls.push(`SELECT DISTINCT "${a.p.k.column}", "${a.s.k.column}"
                  FROM "${kt}" WHERE ${both}`);
}
let to_answers = db.query_all(to_sqls);
let to_stats = #{};
for i in 0..to_keys.len() {
    let parts = to_keys[i].split("|");
    let filled = to_answers[i * 2].col("c").value_at(0).parse_int();
    let kv = to_answers[i * 2 + 1].pair_keys(parts[1], parts[2]);
    let dst = kv.count();
    to_stats[to_keys[i]] = #{
        filled: filled, dst: dst, kv: kv,
        ok: filled > 0 && dst >= 2 && dst.to_float() / filled.to_float() >= 0.9,
    };
}

// Phase 2, batched, survivors only: the from side's co-present pairs.
let from_keys = [];
let from_sqls = [];
for a in attempts {
    if !to_stats[`${a.p.k.table}|${a.p.k.column}|${a.s.k.column}`].ok { continue; }
    let ft = a.p.f.table;
    let key = `${ft}|${a.p.f.column}|${a.s.f.column}`;
    if from_keys.contains(key) { continue; }
    from_keys.push(key);
    let fboth = `"${a.p.f.column}" IS NOT NULL AND "${a.s.f.column}" IS NOT NULL`;
    from_sqls.push(`SELECT count(*) AS c FROM "${ft}" WHERE ${fboth}`);
    from_sqls.push(`SELECT DISTINCT "${a.p.f.column}", "${a.s.f.column}"
                    FROM "${ft}" WHERE ${fboth}`);
}
let from_answers = db.query_all(from_sqls);
let from_stats = #{};
for i in 0..from_keys.len() {
    let parts = from_keys[i].split("|");
    let kv = from_answers[i * 2 + 1].pair_keys(parts[1], parts[2]);
    from_stats[from_keys[i]] = #{
        ffilled: from_answers[i * 2].col("c").value_at(0).parse_int(),
        fpairs: kv.count(),
        kv: kv,
    };
}

// Phase 3, no SQL: the two-leg resolution is a kernel merge. First
// passing scope per anchor, in overlap order — the batched equivalent
// of the old first-success break.
let rescued = #{};
for a in attempts {
    let ts = to_stats[`${a.p.k.table}|${a.p.k.column}|${a.s.k.column}`];
    if !ts.ok { continue; }
    let fs = from_stats[`${a.p.f.table}|${a.p.f.column}|${a.s.f.column}`];
    if fs.fpairs == 0 { continue; }
    let m = fs.kv.matched(ts.kv);
    let overlap = m.to_float() / fs.fpairs.to_float();
    if overlap < 0.5 { continue; }
    let akey = `${a.anchor}`;
    if akey in rescued && rescued[akey].order <= a.order { continue; }
    rescued[akey] = #{ order: a.order, p: a.p, s: a.s, m: m, overlap: overlap,
                       dst: ts.dst, ffilled: fs.ffilled, fpairs: fs.fpairs };
}
for akey in rescued.keys() {
    let r = rescued[akey];
    // The anchor is the identifying leg — the higher-cardinality to
    // side; the scope is the tenant leg. Which pair triggered the
    // rescue is iteration order, not evidence.
    let a = r.p;
    let sc = r.s;
    if r.s.k.distinct > r.p.k.distinct {
        a = r.s;
        sc = r.p;
    }
    candidates.push(#{
        from: `${a.f.table}.${a.f.column}`,
        to: `${a.k.table}.${a.k.column}`,
        key_columns: [#{
            from: `${sc.f.table}.${sc.f.column}`,
            to: `${sc.k.table}.${sc.k.column}`,
        }],
        cardinality: if r.fpairs == r.ffilled { "one-to-one" } else { "many-to-one" },
        overlap: r.overlap,
        matched: r.m,
        orphans: r.fpairs - r.m,
        from_distinct: r.fpairs,
        to_distinct: r.dst,
    });
}

candidates.sort(|a, b| if a.overlap < b.overlap {
    1
} else if a.overlap > b.overlap {
    -1
} else {
    0
});

#{ candidates: candidates }
