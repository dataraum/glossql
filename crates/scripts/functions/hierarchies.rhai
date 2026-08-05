// Hierarchy candidates (port list item 3, second part): the cheap SQL
// core of v0.3's dimension-identity stack (analysis/hierarchies,
// transcribed 2026-08-05) — pairwise FD screens at high recall over one
// table's dimension-like columns. Runs at table grain
// (`SELECT detect_hierarchies() FROM journal_lines`).
//
// v0.3's decision layer, dispositioned by the recall ruling (the
// measurement's job is recall; the judge removes false positives):
// - Null policy PORTED: NULL is a category — a null-coded binary
//   {1, NULL} is a lane, not a silent constant-drop. GROUP BY keeps
//   the NULL group, and distinct counts here include it.
// - The g3 row screen PORTED loose: candidates ship at g3 ≤ 0.05;
//   v0.3 asserted edges at 0.01 — the extra band is the judge's to
//   kill against the data.
// - Goodman–Kruskal λ SERVED beside every candidate, never gated:
//   λ < 0.5 is the recorded vacuous-skew signature (a ≥98%-dominant
//   dependent passes g3 vacuously; v0.3's pre-registered floor killed
//   48 false positives with zero truth lost). The judge reads it.
// - Permutation nulls + false-discovery control NOT ported: precision
//   apparatus compensating for judge-less operation.
// - Measures stay out by dtype (Float/Decimal): the additivity lane
//   floods FD discovery (`revenue → tier` is asserted by every
//   statistic). v0.3 excluded by semantic role; dtype is the proxy a
//   measurement can see.
// - Guards are FULL-scan (the rel-hm fold-key lesson: a row sample
//   makes fold keys look near-key; never sample the guards).
//
// A perfect 1:1 both ways is served as kind "alias" — but whether it
// is a code↔label relabeling or a coincidence is exactly what no
// statistic can settle; the identity judgment stays with the reader
// (the glossql-dimensions skill carries the lessons).
//
// Cost: one LIMIT 0, one count, one aggregate scan for the pool, one
// modal probe per pool column, three grouped scans per unordered pair.

let parts = subject.split(".");
let out = #{ applicable: false };

if parts.len() == 1 {
    let table = parts[0];
    let head = db.query(`SELECT * FROM "${table}" LIMIT 0`);
    let n = db.query(`SELECT count(*) AS n FROM "${table}"`).cell("n");
    let n = if n == () { 0 } else { n.parse_int() };
    if n == 0 {
        out = #{ applicable: false, reason: "empty table" };
    } else {
        let sel = "";
        let names = [];
        for c in head.columns() {
            let d = head.col(c).dtype();
            if d.starts_with("Float") || d.starts_with("Decimal") { continue; }
            names.push(c);
            if sel != "" { sel += ", "; }
            sel += `count("${c}") AS "f_${c}", count(DISTINCT "${c}") AS "d_${c}"`;
        }
        let pool = [];
        if names.len() > 0 {
            let agg = db.query(`SELECT ${sel} FROM "${table}"`);
            for c in names {
                let filled = agg.col(`f_${c}`).value_at(0).parse_int();
                let distinct = agg.col(`d_${c}`).value_at(0).parse_int();
                // NULL is a category: the null rows form one more group.
                let nac = distinct + if filled < n { 1 } else { 0 };
                if nac < 2 { continue; }
                if filled > 0 && distinct.to_float() / filled.to_float() >= 0.9 {
                    continue; // near-key, by fraction — full-scan exact
                }
                pool.push(#{ name: c, nac: nac });
            }
        }
        if pool.len() < 2 {
            out = #{ applicable: false,
                     reason: "fewer than two dimension-like columns" };
        } else {
            // Modal group size per pool column — λ's baseline: how
            // often a guess of the modal value is right with no
            // determinant at all.
            for i in 0..pool.len() {
                let c = pool[i].name;
                let m = db.query(`SELECT max(c) AS m FROM
                    (SELECT count(*) AS c FROM "${table}" GROUP BY "${c}") AS g`);
                pool[i].modal = m.cell("m").parse_int();
            }

            let candidates = [];
            for i in 0..pool.len() {
                for j in i + 1..pool.len() {
                    let a = pool[i];
                    let b = pool[j];
                    let ab = `SELECT "${a.name}" AS ga, "${b.name}" AS gb,
                              count(*) AS c FROM "${table}" GROUP BY 1, 2`;
                    let pair_groups = db.query(
                        `SELECT count(*) AS v FROM (${ab}) AS ab`)
                        .cell("v").parse_int();
                    // Σ per-determinant max: the rows an exact FD would
                    // keep. g3 = the fraction it would have to drop.
                    let agree_ab = db.query(`SELECT sum(mx) AS v FROM
                        (SELECT ga, max(c) AS mx FROM (${ab}) AS ab GROUP BY ga) AS am`)
                        .cell("v").parse_int();
                    let agree_ba = db.query(`SELECT sum(mx) AS v FROM
                        (SELECT gb, max(c) AS mx FROM (${ab}) AS ab GROUP BY gb) AS bm`)
                        .cell("v").parse_int();
                    let g3_ab = (n - agree_ab).to_float() / n.to_float();
                    let g3_ba = (n - agree_ba).to_float() / n.to_float();
                    // Near-copy semantics, conservative: alias only when
                    // both directions hold at v0.3's asserted line.
                    let alias = g3_ab <= 0.01 && g3_ba <= 0.01;
                    if g3_ab <= 0.05 {
                        candidates.push(#{
                            from: a.name, to: b.name,
                            distinct_from: a.nac, distinct_to: b.nac,
                            pair_groups: pair_groups, g3: g3_ab,
                            lambda: (agree_ab - b.modal).to_float()
                                / (n - b.modal).to_float(),
                            kind: if alias { "alias" } else { "edge" },
                        });
                    }
                    if g3_ba <= 0.05 {
                        candidates.push(#{
                            from: b.name, to: a.name,
                            distinct_from: b.nac, distinct_to: a.nac,
                            pair_groups: pair_groups, g3: g3_ba,
                            lambda: (agree_ba - a.modal).to_float()
                                / (n - a.modal).to_float(),
                            kind: if alias { "alias" } else { "edge" },
                        });
                    }
                }
            }
            out = #{ applicable: true, rows: n, candidates: candidates };
        }
    }
}

out
