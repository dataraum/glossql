// Temporal profile (fixture 11): the deterministic temporal family as one
// measurement — window, cadence, completeness, gaps. Ported from v0.3's
// analyze_basic_temporal (analysis/temporal/detection.py, verified
// 2026-08-04). Cadence is the nearest named grain to the median gap
// between DISTINCT instants, so duplicate-heavy fact columns count each
// day once; completeness counts calendar buckets over the column's own
// window — a grain's nominal seconds serve inference only, never a period
// denominator (months vary); gaps are the stretches beyond twice the
// median. Two v0.3 fields stayed behind: `is_stale`, the family's only
// wall-clock-dependent field (a verdict about now is judgment, and
// judgment lives in detectors and read policy, never in results), and
// `last_period_complete` (low-signal by its own documentation, read by
// nothing).
let parts = subject.split(".");
let table = parts[0];
let column = parts[1];

// A point in time bounds a window; durations and times-of-day do not
// (v0.3's TIME_POINT_TYPES rule, by type never by name). The door's
// empty results still carry schema, so LIMIT 0 types the column
// without scanning it.
let t = db.query(`SELECT "${column}" AS v FROM "${table}" LIMIT 0`);
let ty = t.col("v").dtype();
if !(ty.starts_with("Date") || ty.starts_with("Timestamp")) {
    #{ applicable: false }
} else {
    let ts = `CAST("${column}" AS TIMESTAMP)`;
    // Aggregate once, compute on the bounds outside — mixing max("c") with
    // max(CAST("c" …)) in one aggregate collides after the optimizer
    // normalizes the cast away.
    let w = db.query(`SELECT present, min_v, max_v,
        to_unixtime(CAST(max_v AS TIMESTAMP))
          - to_unixtime(CAST(min_v AS TIMESTAMP)) AS span_seconds
        FROM (SELECT count("${column}") AS present,
                     min("${column}") AS min_v, max("${column}") AS max_v
              FROM "${table}") AS bounds`);
    if w.cell("present").parse_int() == 0 {
        // Nothing to bound a window with — abstain like outliers does.
        #{ applicable: false }
    } else {
        let g = db.query(`
            WITH ordered_ts AS (SELECT DISTINCT ${ts} AS t
                                FROM "${table}" WHERE "${column}" IS NOT NULL),
            gaps AS (SELECT CAST(to_unixtime(lead(t) OVER (ORDER BY t))
                                 - to_unixtime(t) AS DOUBLE) AS gap_seconds
                     FROM ordered_ts)
            SELECT median(gap_seconds) AS med, min(gap_seconds) AS lo,
                   max(gap_seconds) AS hi
            FROM gaps WHERE gap_seconds IS NOT NULL`);
        let med = g.cell("med");
        let granularity = "unknown";
        let confidence = 0.0;
        if med != () {
            med = med.parse_float();
            granularity = "irregular";
            confidence = 0.3;
            // [name, nominal seconds, tolerance] — nearest definition wins.
            let grains = [
                ["second", 1.0, 0.5], ["minute", 60.0, 5.0],
                ["hour", 3600.0, 300.0], ["day", 86400.0, 3600.0],
                ["week", 604800.0, 86400.0], ["month", 2592000.0, 259200.0],
                ["quarter", 7776000.0, 777600.0],
                ["year", 31536000.0, 3153600.0],
            ];
            let best = 1.0e300;
            for grain in grains {
                let d = abs(med - grain[1]);
                if d < grain[2] && d < best {
                    granularity = grain[0];
                    best = d;
                }
            }
            if granularity != "irregular" {
                let lo = g.cell("lo").parse_float();
                let hi = g.cell("hi").parse_float();
                // v0.3's exact rule: a zero min gap (sub-second neighbours
                // floored to 0) keeps the flat default instead of the
                // variation formula; the 0.5 cap doubles as the floor.
                if lo > 0.0 && hi > 0.0 && med > 0.0 {
                    let v = (hi - lo) / med / 10.0;
                    if v > 0.5 { v = 0.5; }
                    confidence = 1.0 - v;
                } else {
                    confidence = 0.7;
                }
            }
        }

        let gaps = #{ count: 0 };
        if med != () && med > 0.0 {
            let threshold = med * 2.0;
            let sig = db.query(`
                WITH ordered_ts AS (SELECT DISTINCT ${ts} AS t
                                    FROM "${table}" WHERE "${column}" IS NOT NULL),
                g AS (SELECT t AS gap_start, lead(t) OVER (ORDER BY t) AS gap_end,
                             CAST(to_unixtime(lead(t) OVER (ORDER BY t))
                                  - to_unixtime(t) AS DOUBLE) AS gap_seconds
                      FROM ordered_ts)
                SELECT gap_start, gap_end, gap_seconds FROM g
                WHERE gap_seconds > ${threshold} ORDER BY gap_seconds DESC`);
            let n = sig.num_rows();
            gaps.count = n;
            if n > 0 {
                let starts = sig.col("gap_start");
                let ends = sig.col("gap_end");
                let seconds = sig.col("gap_seconds");
                gaps.largest_days = seconds.value_at(0).parse_float() / 86400.0;
                // The sample keeps the 20 largest; the count stays true.
                let sample = [];
                let k = if n < 20 { n } else { 20 };
                for i in 0..k {
                    let s = seconds.value_at(i).parse_float();
                    sample.push(#{
                        // start/end are the observations around the hole,
                        // not the hole's own endpoints.
                        start: starts.value_at(i),
                        end: ends.value_at(i),
                        days: s / 86400.0,
                        missing_periods: (s / med).to_int() - 1,
                        severity: if s > med * 10.0 { "severe" }
                                  else if s > med * 5.0 { "moderate" }
                                  else { "minor" },
                    });
                }
                gaps.sample = sample;
            }
        }

        let out = #{
            applicable: true,
            min: w.cell("min_v"),
            max: w.cell("max_v"),
            span_days: w.cell("span_seconds").parse_float() / 86400.0,
            granularity: granularity,
            confidence: confidence,
            gaps: gaps,
        };
        if granularity != "unknown" && granularity != "irregular" {
            // Calendar grains count by calendar arithmetic; fixed grains by
            // epoch steps between truncated bounds — both inclusive.
            let year_diff = `extract(year FROM hi_t) - extract(year FROM lo_t)`;
            let expected_expr = switch granularity {
                "year" => `${year_diff} + 1`,
                "quarter" => `(${year_diff}) * 4
                    + extract(quarter FROM hi_t) - extract(quarter FROM lo_t) + 1`,
                "month" => `(${year_diff}) * 12
                    + extract(month FROM hi_t) - extract(month FROM lo_t) + 1`,
                _ => {
                    let step = switch granularity {
                        "second" => 1, "minute" => 60, "hour" => 3600,
                        "day" => 86400, "week" => 604800,
                    };
                    `(to_unixtime(date_trunc('${granularity}', hi_t))
                      - to_unixtime(date_trunc('${granularity}', lo_t))) / ${step} + 1`
                }
            };
            let c = db.query(`SELECT actual, CAST(${expected_expr} AS BIGINT) AS expected
                FROM (SELECT count(DISTINCT date_trunc('${granularity}', ${ts})) AS actual,
                             min(${ts}) AS lo_t, max(${ts}) AS hi_t
                      FROM "${table}" WHERE "${column}" IS NOT NULL) AS bounds`);
            let actual = c.cell("actual").parse_int();
            let expected = c.cell("expected").parse_int();
            out.completeness = #{
                expected: expected,
                actual: actual,
                ratio: actual.to_float() / expected.to_float(),
            };
        }
        out
    }
}
