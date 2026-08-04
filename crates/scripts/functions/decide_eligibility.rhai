// Eligibility (fixture 11): an all-null column serves nothing and trips
// every column-scanning consumer, so its pick is `false` and the typed
// view drops it — raw and the glossary keep it, and a superseding gloss
// brings it back. The rule set IS this script: v0.3 kept its rules in
// YAML behind an eval() and its WARN tier died unread; one rule ever
// fired on real data, so one rule is what ports.
let p = context.column_profile;
if p == () {
    // Not yet profiled — fail open, the column serves.
    #{ value: true }
} else if p.null_ratio >= 1.0 {
    #{ value: false, reason: "all null" }
} else {
    #{ value: true }
}
