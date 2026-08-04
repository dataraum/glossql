// Outliers (fixture 11): IQR fences and the modified Z-score — universal
// distribution facts, no domain rules. ACCEPTS column_profile: the
// quartiles and MAD come from the cached profile, never recomputed — the
// door only counts what lies beyond the fences, and a re-profiled column
// invalidates this cache through the ACCEPTS rule. Reads the landed
// table like the profile it chains on; TRY_CAST is identity on a typed
// numeric column and keeps the door honest on an untyped one.
let p = context.column_profile;
if p == () || !("numeric" in p) {
    #{ applicable: false }
} else {
    let parts = subject.split(".");
    let column = parts[1];
    let table = parts[0];
    let q1 = p.numeric.percentiles.p25;
    let q3 = p.numeric.percentiles.p75;
    let iqr_low = q1 - 1.5 * (q3 - q1);
    let iqr_high = q3 + 1.5 * (q3 - q1);
    // Modified Z-score |0.6745 (x - median) / MAD| > 3.5, folded into
    // fences; MAD 0 (half the values identical) makes every deviation
    // infinite, so the check abstains.
    let median = p.numeric.percentiles.p50;
    let mad = p.numeric.mad;
    let z_low = if mad > 0.0 { median - 3.5 * mad / 0.6745 } else { iqr_low };
    let z_high = if mad > 0.0 { median + 3.5 * mad / 0.6745 } else { iqr_high };
    let v = `TRY_CAST("${column}" AS DOUBLE)`;
    let t = db.query(`SELECT count(${v}) AS parsed,
        count(CASE WHEN ${v} < ${iqr_low} OR ${v} > ${iqr_high} THEN 1 END) AS iqr_out,
        count(CASE WHEN ${v} < ${z_low} OR ${v} > ${z_high} THEN 1 END) AS z_out
        FROM "${table}"`);
    let parsed = t.cell("parsed").parse_int();
    let iqr_out = t.cell("iqr_out").parse_int();
    let z_out = if mad > 0.0 { t.cell("z_out").parse_int() } else { 0 };
    let ratio = |count| if parsed == 0 { 0.0 } else { count.to_float() / parsed.to_float() };
    #{
        applicable: true,
        iqr: #{ lower: iqr_low, upper: iqr_high, count: iqr_out, ratio: ratio.call(iqr_out) },
        zscore: #{ count: z_out, ratio: ratio.call(z_out) },
    }
}
