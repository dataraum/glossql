// Slot-disagreement detector (fixture 06): sees slots and threshold, never
// table data. Score is the fraction of extra distinct values across slots;
// band maps score against the witness threshold.
let values = [];
for s in context.slots {
    let v = if s.body != () && type_of(s.body) == "map" && "value" in s.body {
        s.body.value
    } else {
        s.body
    };
    if !values.contains(v) {
        values.push(v);
    }
}
let n = context.slots.len();
let score = if n <= 1 { 0.0 } else { (values.len() - 1).to_float() / (n - 1).to_float() };
let threshold = if context.threshold != () { context.threshold } else { 1.0 };
let band = if score == 0.0 {
    "green"
} else if score <= threshold {
    "yellow"
} else {
    "red"
};
#{
    subject: context.subject, aspect: context.aspect, witness: context.witness,
    band: band, score: score, computed_at: "",
}
