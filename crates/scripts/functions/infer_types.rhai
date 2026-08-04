// Type candidates (fixture 11/13): patterns nominate on values, trial casts
// verify — never column names. Patterns arrive as context (ACCEPTS
// type_patterns); their exprs carry a `{col}` placeholder in the
// substrate's SQL spelling.
let parts = subject.split(".");
let table = parts[0];
let column = parts[1];
let t = db.query(`SELECT "${column}" AS v FROM "${table}_raw"`);
let c = t.col("v");
let candidates = [];

let cfg = context.type_patterns;
if cfg != () {
    for p in cfg.patterns {
        let mr = c.match_rate(p.pattern);
        if mr >= 0.5 {
            if "standardization_expr" in p {
                // The pattern quotes its own `{col}` placeholder.
                let expr = p.standardization_expr;
                expr.replace("{col}", column);
                let trial = db.query(`SELECT count("${column}") AS nn, count(TRY_CAST(${expr} AS ${p.inferred_type})) AS ok FROM "${table}_raw"`);
                let nn = trial.col("nn").value_at(0).parse_int();
                let ok = trial.col("ok").value_at(0).parse_int();
                let pr = if nn == 0 { 1.0 } else { ok.to_float() / nn.to_float() };
                candidates.push(#{
                    type: p.inferred_type, confidence: (mr + pr) / 2.0,
                    pattern: p.name, expr: expr,
                });
            } else {
                let pr = c.parse_rate(p.inferred_type);
                candidates.push(#{
                    type: p.inferred_type, confidence: (mr + pr) / 2.0,
                    pattern: p.name,
                });
            }
        }
    }
}

// Bare trial casts, patternless.
for ty in ["BIGINT", "DOUBLE", "DATE", "BOOLEAN"] {
    let pr = c.parse_rate(ty);
    if pr >= 0.5 {
        candidates.push(#{ type: ty, confidence: pr });
    }
}

#{ candidates: candidates }
