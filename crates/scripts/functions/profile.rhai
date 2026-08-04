// Column profile (fixture 11): the deterministic profile plane as a
// measurement. Subject is a column path; the data is the raw table.
let parts = subject.split(".");
let table = parts[0];
let column = parts[1];
let t = db.query(`SELECT "${column}" AS v FROM "${table}_raw"`);
let c = t.col("v");
let n = c.count();
let nulls = c.null_count();
#{
    null_ratio: if n == 0 { 0.0 } else { nulls.to_float() / n.to_float() },
    distinct: c.distinct(),
    min: c.min(),
    max: c.max(),
}
