// Behavior evidence (port list item 2, ruled 2026-08-05): the stock/flow
// discriminator from v0.3's lineage reconcile (analysis/lineage/
// reconcile.py, transcribed 2026-08-05), reshaped as an evidence
// measurement — the judge reads it before glossing `behavior`; it is
// never a voice in the behavior slots. Runs at column grain
// (`SELECT behavior_evidence() FROM trial_balance.debit_balance`).
//
// The falsification that shaped it (v0.3's own record): a column's OWN
// trajectory cannot decide stock vs flow — a trending flow and a
// mean-reverting stock look alike. The evidence must be cross-table:
// an INDEPENDENT per-period movement m aggregated from a related event
// table, scored against two hypotheses with scale-free residuals
//   FLOW : y[t]  ≈ m[t]        R_flow  = Σ|y − m| / Σ|m|
//   STOCK: Δy[t] ≈ m[t] (t≥1)  R_stock = Σ|Δy − m'| / Σ|m'|
//
// Anchors come from DECLARED relationships only (ruled 2026-08-05): the
// plane order is relationships first, and the wrong-anchor gate then
// guards misdeclared edges rather than a combinatorial sweep. Composite
// (tuple) endpoints are skipped for now. A newly declared relationship
// does NOT invalidate this cache — ACCEPTS edges are aspect-only — so
// recompute by `DELETE FROM cache WHERE function = 'behavior_evidence'`
// after declaring; the add-source skill teaches this.
//
// Conventions (which event columns carry the movement) are v0.3's
// enumeration: each numeric column, and every ordered pair difference
// (debit − credit) — evaluated as dict arithmetic over stored per-period
// sums, never extra scans (linearity of SUM). Pairing is on the
// intersection of (entity, period) cells present on both sides; a
// calendar gap between kept cells makes Δy span it and read as noise —
// the calendar-complete enumeration did not port, honest abstention
// absorbs the miss.
//
// Cost: one LIMIT 0 per landed table, one grain probe per measure axis,
// two grouped aggregates per anchor; everything after is arithmetic.

// ---- ported arithmetic (constants verbatim; the two gates are COUPLED) ----
// MIN_PERIODS = 4          residuals on shorter series are not meaningful
// FIRE_RESIDUAL_MAX = 0.5  wrong-anchor gate: correct-anchor winning
//                          residual measured ≈ 0.0–0.1 (≤ ~0.5 under heavy
//                          noise); wrong-anchor min-residual ≈ 1.0 — the
//                          gate sits at the measured separation midpoint
// MIN_SEPARATION = (1 − 0.5)/(1 + 0.5) = 1/3 — DERIVED, no number of its
//                          own: the weakest fit still admitted (0.5)
//                          against the false hypothesis' ≈ 1.0 level.
//                          Moving FIRE_RESIDUAL_MAX to 1.0 silently
//                          disables this gate; to 0.0 lets nothing fire.
// MIN_ENTITIES_FIRED = 2, AGREEMENT_MIN = 0.8 — a lone entity or a split
//                          vote is ignorance, not lineage.

fn reconcile(y, m) {
    // 1.0e300 stands in for a dead normalizer: scale-free by
    // construction, and degrading to an absolute scale can mint a
    // spurious perfect fit — the hypothesis abstains instead.
    let denom_flow = 0.0;
    for v in m { denom_flow += abs(v); }
    let r_flow = 1.0e300;
    if denom_flow > 0.0 {
        let s = 0.0;
        for i in 0..y.len() { s += abs(y[i] - m[i]); }
        r_flow = s / denom_flow;
    }
    let denom_stock = 0.0;
    for i in 1..m.len() { denom_stock += abs(m[i]); }
    let r_stock = 1.0e300;
    if denom_stock > 0.0 {
        let s = 0.0;
        for i in 1..y.len() { s += abs((y[i] - y[i - 1]) - m[i]); }
        r_stock = s / denom_stock;
    }
    [r_flow, r_stock]
}

fn separation(rf, rs) {
    // (R_lose − R_win) / (R_lose + R_win): 1 when one hypothesis fits
    // and the other does not, 0 when indistinguishable. Two identically
    // zero residuals are NO separation — a series whose whole movement
    // is one terminal period fits both, and is evidence for neither.
    let r_win = if rf < rs { rf } else { rs };
    let r_lose = if rf < rs { rs } else { rf };
    if r_win >= 1.0e300 { return 0.0; }
    if r_lose >= 1.0e300 { return 1.0; }
    let total = r_win + r_lose;
    if total == 0.0 { return 0.0; }
    (r_lose - r_win) / total
}

fn classify(y, m) {
    // One entity's vote; label () = abstained. A dead measure abstains
    // symmetrically with a dead anchor: a series that never moves has
    // no stock/flow nature to detect.
    if y.len() < 4 { return #{ label: (), r_flow: 1.0e300, r_stock: 1.0e300 }; }
    let any_y = false;
    let any_m = false;
    for v in y { if v != 0.0 { any_y = true; } }
    for v in m { if v != 0.0 { any_m = true; } }
    if !any_y || !any_m { return #{ label: (), r_flow: 1.0e300, r_stock: 1.0e300 }; }
    let rr = reconcile(y, m);
    let rf = rr[0];
    let rs = rr[1];
    let r_min = if rf < rs { rf } else { rs };
    if r_min > 0.5 {
        // Wrong-anchor gate: neither hypothesis fits — abstain, never guess.
        return #{ label: (), r_flow: rf, r_stock: rs };
    }
    if separation(rf, rs) < (1.0 - 0.5) / (1.0 + 0.5) {
        // Near-tie gate: one hypothesis fits, but not measurably better
        // than the other — the winner would be the noise.
        return #{ label: (), r_flow: rf, r_stock: rs };
    }
    let label = if rs < rf { "stock" } else { "flow" };
    #{ label: label, r_flow: rf, r_stock: rs }
}

fn wilson_lcb(successes, n) {
    // Wilson score lower bound (Wilson 1927): the parameter-free way to
    // rank a vote rate under small n. n MUST be the pairing's COMMON
    // entity denominator, never a convention's own aligned subset —
    // v0.3's recorded support-gameability trap: sparse pair differences
    // structurally out-raced true singles on half-entity subsets.
    if n <= 0 { return 0.0; }
    let z = 1.96;
    let nf = n.to_float();
    let p = successes.to_float() / nf;
    let denom = 1.0 + z * z / nf;
    let centre = p + z * z / (2.0 * nf);
    let margin = z * sqrt(p * (1.0 - p) / nf + z * z / (4.0 * nf * nf));
    let v = (centre - margin) / denom;
    if v < 0.0 { 0.0 } else { v }
}

fn median_of(vals) {
    if vals.len() == 0 { return (); }
    let s = vals;
    s.sort();
    let n = s.len();
    if n % 2 == 1 { s[n / 2] } else { (s[n / 2 - 1] + s[n / 2]) / 2.0 }
}

// ---- subject and schema ----

let parts = subject.split(".");
let out = #{ applicable: false };

if parts.len() == 2 {
    let table = parts[0];
    let column = parts[1];

    // Landed tables and their shapes — LIMIT 0 types without scanning.
    let tables = [];
    let names = db.query("SELECT DISTINCT table_name FROM imports ORDER BY table_name")
        .col("table_name");
    for i in 0..names.count() { tables.push(names.value_at(i)); }

    let schemas = #{};
    for t in tables {
        let head = db.query(`SELECT * FROM "${t}" LIMIT 0`);
        let cols = [];
        for c in head.columns() {
            cols.push(#{ name: c, dtype: head.col(c).dtype() });
        }
        schemas[t] = cols;
    }

    let numeric = false;
    if table in schemas {
        for c in schemas[table] {
            if c.name == column {
                let d = c.dtype;
                numeric = d.starts_with("Int") || d.starts_with("UInt")
                    || d.starts_with("Float") || d.starts_with("Decimal");
            }
        }
    }

    if !numeric {
        // Not a numeric measure — genuinely doesn't fit, like outliers
        // on text.
        out = #{ applicable: false };
    } else {
        // Declared edges as directed pointers; "<->" points both ways.
        // Composite (tuple) endpoints are skipped for now.
        let pointers = [];
        let rels = db.query("SELECT left_path, op, right_path FROM relationships");
        let lcol = rels.col("left_path");
        let ocol = rels.col("op");
        let rcol = rels.col("right_path");
        for i in 0..rels.num_rows() {
            let lp = lcol.value_at(i);
            let rp = rcol.value_at(i);
            if lp.contains("(") || rp.contains("(") { continue; }
            let l = lp.split(".");
            let r = rp.split(".");
            if l.len() != 2 || r.len() != 2 { continue; }
            if !(l[0] in schemas) || !(r[0] in schemas) { continue; }
            pointers.push(#{ src_t: l[0], src_c: l[1], dst_t: r[0], dst_c: r[1] });
            if ocol.value_at(i) == "<->" {
                pointers.push(#{ src_t: r[0], src_c: r[1], dst_t: l[0], dst_c: l[1] });
            }
        }

        // Time axes of a table t: its own date columns, plus date columns
        // one declared hop away (journal_lines borrows journal_entries'
        // date through entry_id). Built inline for the measure table and
        // for each event candidate.
        let all_axes = #{};
        for t in tables {
            let axes = [];
            for c in schemas[t] {
                if c.dtype.starts_with("Date") || c.dtype.starts_with("Timestamp") {
                    axes.push(#{ label: `${t}.${c.name}`, borrowed: false,
                                 time_col: c.name });
                }
            }
            for p in pointers {
                if p.src_t != t || p.dst_t == t { continue; }
                for c in schemas[p.dst_t] {
                    if c.dtype.starts_with("Date") || c.dtype.starts_with("Timestamp") {
                        let label = `${p.dst_t}.${c.name} via ${p.src_c}`;
                        let seen = false;
                        for a in axes { if a.label == label { seen = true; } }
                        if !seen {
                            axes.push(#{ label: label, borrowed: true, fk: p.src_c,
                                         time_table: p.dst_t, key: p.dst_c,
                                         time_col: c.name });
                        }
                    }
                }
            }
            all_axes[t] = axes;
        }

        let m_axes = all_axes[table];
        if m_axes.len() == 0 {
            out = #{ applicable: false,
                     reason: `no period axis: no date column on ${table} or one declared hop away` };
        } else {
            let anchors = [];
            let ycache = #{};

            for maxis in m_axes {
                // FROM clause for the measure side; s = subject table,
                // tt = borrowed time table.
                let m_from = `"${table}" s`;
                let m_texpr = `s."${maxis.time_col}"`;
                if maxis.borrowed {
                    m_from += ` JOIN "${maxis.time_table}" tt ON s."${maxis.fk}" = tt."${maxis.key}"`;
                    m_texpr = `tt."${maxis.time_col}"`;
                }
                let m_ts = `CAST(${m_texpr} AS TIMESTAMP)`;

                // Bucket grain: the coarsest named grain the axis is
                // already truncated to (a period-keyed snapshot axis);
                // event-grain axes fall through to calendar months, the
                // dominant business period — sums are linear, so a flow
                // reconciles at any grain.
                let g = db.query(`SELECT
                    SUM(CASE WHEN date_trunc('year', ${m_ts}) = ${m_ts} THEN 0 ELSE 1 END) AS ny,
                    SUM(CASE WHEN date_trunc('quarter', ${m_ts}) = ${m_ts} THEN 0 ELSE 1 END) AS nq,
                    SUM(CASE WHEN date_trunc('month', ${m_ts}) = ${m_ts} THEN 0 ELSE 1 END) AS nm
                    FROM ${m_from} WHERE ${m_texpr} IS NOT NULL`);
                if g.cell("ny") == () { continue; }
                let grain = "month";
                if g.cell("ny").parse_int() == 0 { grain = "year"; }
                else if g.cell("nq").parse_int() == 0 { grain = "quarter"; }

                for ev in tables {
                    if ev == table { continue; }

                    // Entity alignments: a declared edge between the two
                    // tables directly, or two edges meeting at the same
                    // dimension key (tb.account_id and jl.account_id both
                    // pointing at chart_of_accounts.account_id).
                    let aligns = [];
                    for p in pointers {
                        if p.src_t == table && p.dst_t == ev {
                            aligns.push(#{ m_col: p.src_c, e_col: p.dst_c });
                        }
                        if p.src_t == ev && p.dst_t == table {
                            aligns.push(#{ m_col: p.dst_c, e_col: p.src_c });
                        }
                    }
                    for p in pointers {
                        if p.src_t != table { continue; }
                        for q in pointers {
                            if q.src_t == ev && q.dst_t == p.dst_t && q.dst_c == p.dst_c
                                && p.dst_t != table && p.dst_t != ev {
                                aligns.push(#{ m_col: p.src_c, e_col: q.src_c });
                            }
                        }
                    }
                    let seen = [];
                    let deduped = [];
                    for a in aligns {
                        let k = `${a.m_col}=${a.e_col}`;
                        if !(k in seen) { seen.push(k); deduped.push(a); }
                    }
                    if deduped.len() == 0 { continue; }

                    let e_axes = all_axes[ev];
                    if e_axes.len() == 0 { continue; }

                    for al in deduped {
                        // Movement candidates: the event table's numeric
                        // columns, minus the alignment column itself.
                        let terms_pool = [];
                        for c in schemas[ev] {
                            if c.name == al.e_col { continue; }
                            let d = c.dtype;
                            if d.starts_with("Int") || d.starts_with("UInt")
                                || d.starts_with("Float") || d.starts_with("Decimal") {
                                terms_pool.push(c.name);
                            }
                        }
                        if terms_pool.len() == 0 { continue; }
                        let convs = [];
                        for c in terms_pool { convs.push([c]); }
                        for c1 in terms_pool {
                            for c2 in terms_pool {
                                if c1 != c2 { convs.push([c1, c2]); }
                            }
                        }

                        // Measure-side series, cached per (axis, entity
                        // column): entity → bucket → Σy.
                        let ykey = `${maxis.label}|${al.m_col}|${grain}`;
                        if !(ykey in ycache) {
                            let ym = #{};
                            let yq = db.query(`SELECT s."${al.m_col}" AS e,
                                date_trunc('${grain}', ${m_ts}) AS b,
                                SUM(CAST(s."${column}" AS DOUBLE)) AS yv
                                FROM ${m_from}
                                WHERE s."${al.m_col}" IS NOT NULL AND ${m_texpr} IS NOT NULL
                                  AND s."${column}" IS NOT NULL
                                GROUP BY 1, 2 ORDER BY 1, 2`);
                            let ec = yq.col("e");
                            let bc = yq.col("b");
                            let vc = yq.col("yv");
                            for i in 0..yq.num_rows() {
                                let v = vc.value_at(i);
                                if v == () { continue; }
                                let e = ec.value_at(i);
                                let b = bc.value_at(i);
                                if !(e in ym) { ym[e] = #{}; }
                                ym[e][b] = v.parse_float();
                            }
                            ycache[ykey] = ym;
                        }
                        let ym = ycache[ykey];

                        for eaxis in e_axes {
                            let e_from = `"${ev}" s`;
                            let e_texpr = `s."${eaxis.time_col}"`;
                            if eaxis.borrowed {
                                e_from += ` JOIN "${eaxis.time_table}" tt ON s."${eaxis.fk}" = tt."${eaxis.key}"`;
                                e_texpr = `tt."${eaxis.time_col}"`;
                            }
                            let e_ts = `CAST(${e_texpr} AS TIMESTAMP)`;

                            // Event-side sums for every term at once:
                            // entity → bucket → column → Σ.
                            let sel = "";
                            for c in terms_pool {
                                sel += `, SUM(CAST(s."${c}" AS DOUBLE)) AS "s_${c}"`;
                            }
                            let mq = db.query(`SELECT s."${al.e_col}" AS e,
                                date_trunc('${grain}', ${e_ts}) AS b${sel}
                                FROM ${e_from}
                                WHERE s."${al.e_col}" IS NOT NULL AND ${e_texpr} IS NOT NULL
                                GROUP BY 1, 2 ORDER BY 1, 2`);
                            let mm = #{};
                            let ec = mq.col("e");
                            let bc = mq.col("b");
                            for i in 0..mq.num_rows() {
                                let e = ec.value_at(i);
                                let b = bc.value_at(i);
                                if !(e in mm) { mm[e] = #{}; }
                                if !(b in mm[e]) { mm[e][b] = #{}; }
                                for c in terms_pool {
                                    let v = mq.col(`s_${c}`).value_at(i);
                                    if v != () { mm[e][b][c] = v.parse_float(); }
                                }
                            }

                            // The pairing's common entity denominator —
                            // every convention is judged against it.
                            let common = [];
                            for e in ym.keys() {
                                if e in mm { common.push(e); }
                            }
                            let n_common = common.len();

                            let summaries = [];
                            for conv in convs {
                                let name = conv[0];
                                if conv.len() == 2 { name = `${conv[0]} - ${conv[1]}`; }
                                let flow_votes = 0;
                                let stock_votes = 0;
                                let rf_flow = [];
                                let rs_flow = [];
                                let rf_stock = [];
                                let rs_stock = [];
                                for e in common {
                                    let buckets = [];
                                    for b in ym[e].keys() {
                                        if b in mm[e] { buckets.push(b); }
                                    }
                                    buckets.sort();
                                    let ys = [];
                                    let ms = [];
                                    for b in buckets {
                                        let sums = mm[e][b];
                                        if !(conv[0] in sums) { continue; }
                                        let mv = sums[conv[0]];
                                        if conv.len() == 2 {
                                            if !(conv[1] in sums) { continue; }
                                            mv -= sums[conv[1]];
                                        }
                                        ys.push(ym[e][b]);
                                        ms.push(mv);
                                    }
                                    let r = classify(ys, ms);
                                    if r.label == "flow" {
                                        flow_votes += 1;
                                        rf_flow.push(r.r_flow);
                                        rs_flow.push(r.r_stock);
                                    } else if r.label == "stock" {
                                        stock_votes += 1;
                                        rf_stock.push(r.r_flow);
                                        rs_stock.push(r.r_stock);
                                    }
                                }
                                let voted = flow_votes + stock_votes;
                                let verdict = "abstain";
                                let agreement = 0.0;
                                let winners = 0;
                                let pattern = "flow";
                                if stock_votes > flow_votes { pattern = "stock"; }
                                if voted > 0 {
                                    winners = if pattern == "stock" { stock_votes } else { flow_votes };
                                    agreement = winners.to_float() / voted.to_float();
                                }
                                if voted >= 2 && agreement >= 0.8 { verdict = pattern; }
                                let s = #{ convention: name, terms: conv.len(),
                                           verdict: verdict, voted: voted,
                                           agreement: agreement,
                                           support: wilson_lcb(winners, n_common) };
                                // Medians over the winning-label voters
                                // only — a dissenting minority's residuals
                                // would contaminate the diagnostics.
                                if verdict != "abstain" {
                                    if pattern == "stock" {
                                        s.r_flow = median_of(rf_stock);
                                        s.r_stock = median_of(rs_stock);
                                    } else {
                                        s.r_flow = median_of(rf_flow);
                                        s.r_stock = median_of(rs_flow);
                                    }
                                }
                                summaries.push(s);
                            }

                            // Winner: support-first; on a support tie the
                            // fewer-term convention wins (a single beats
                            // an equally supported pair difference).
                            let winner = ();
                            for s in summaries {
                                if s.verdict == "abstain" { continue; }
                                if winner == () { winner = s; continue; }
                                if s.support > winner.support + 1.0e-9 { winner = s; continue; }
                                if abs(s.support - winner.support) <= 1.0e-9
                                    && s.terms < winner.terms { winner = s; }
                            }

                            // The runner-up field the judge sees beside
                            // the verdict: most-voted first, capped.
                            let alts = [];
                            let used = if winner != () { [winner.convention] } else { [] };
                            while alts.len() < 4 {
                                let best = ();
                                for s in summaries {
                                    if s.convention in used { continue; }
                                    if best == () { best = s; continue; }
                                    if s.voted > best.voted
                                        || (s.voted == best.voted && s.support > best.support) {
                                        best = s;
                                    }
                                }
                                if best == () { break; }
                                used.push(best.convention);
                                alts.push(#{ convention: best.convention,
                                             verdict: best.verdict,
                                             voted: best.voted,
                                             support: best.support });
                            }

                            let anchor = #{ event: ev,
                                            align: `${table}.${al.m_col} = ${ev}.${al.e_col}`,
                                            measure_time: maxis.label,
                                            event_time: eaxis.label,
                                            grain: grain,
                                            entities: n_common,
                                            alternatives: alts };
                            if winner != () {
                                anchor.verdict = winner.verdict;
                                anchor.convention = winner.convention;
                                anchor.voted = winner.voted;
                                anchor.agreement = winner.agreement;
                                anchor.support = winner.support;
                                anchor.r_flow = winner.r_flow;
                                anchor.r_stock = winner.r_stock;
                            } else {
                                anchor.verdict = "abstain";
                                let any_votes = false;
                                for s in summaries { if s.voted > 0 { any_votes = true; } }
                                anchor.reason = if any_votes {
                                    "votes below the candidacy floor (2 voters, 0.8 agreement)"
                                } else {
                                    "no entity series reconciled: wrong anchor, short series, or dead values"
                                };
                            }
                            anchors.push(anchor);
                        }
                    }
                }
            }

            if anchors.len() == 0 {
                out = #{ applicable: false,
                         reason: `no declared relationships connect ${table} to an event table with a time axis` };
            } else {
                out = #{ applicable: true, anchors: anchors };
            }
        }
    }
}

out
