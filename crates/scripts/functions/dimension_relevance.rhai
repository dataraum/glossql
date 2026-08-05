// Dimension relevance (port list item 3, first part; transcribed from
// v0.3's analysis/slicing/relevance.py, 2026-08-05):
//
//     relevance = coverage × evenness
//
// coverage is the fraction of rows landing in a named bucket; evenness
// is Pielou's index — Shannon entropy over its maximum for that many
// buckets, H / ln(k). Read together: of the data this axis can speak
// about, how much of its structure does it actually resolve? No free
// parameters — nothing tuned, no threshold to calibrate.
//
// The falsification that shaped it (v0.3's own record): the
// effective-group ratio exp(H)/k was tried first and scored a 99/1
// near-constant boolean at 0.53 — the same as a merely skewed four-way
// split. Pielou separates them (0.08 vs 0.62) and stays scale-free in
// k: an even 4-way axis and an even 800-way axis both score 1.0. What
// the score deliberately does NOT do: prefer few groups to many —
// which axis a reader wants first is business judgment, and the number
// never overrules it.
//
// Chained on the profile like outliers: buckets are top_values, and a
// truncated distribution's unseen tail is ONE bucket — the least even
// it could be — so a truncated score is a LOWER BOUND. Under-claiming
// is the safe direction: promoting an axis on an assumed-even tail
// that is really one dominant value is exactly the silently-bad
// recommendation the score exists to prevent.
//
// Admission gates are v0.3's recorded inventory: at least two buckets
// with NULL counted as one (a null-coded binary is a lane, not a
// silent constant-drop), null_ratio ≤ 0.5, and near-key as a FRACTION
// ceiling of 0.9 — the absolute-count version was a recorded bug.

let p = context.column_profile;
if p == () {
    #{ applicable: false, missing_aspects: ["column_profile"] }
} else {
    let total = p.total;
    let distinct = p.distinct;
    let null_ratio = p.null_ratio;
    let null_bucket = if null_ratio > 0.0 { 1 } else { 0 };
    if total == 0 {
        #{ applicable: false, reason: "empty column" }
    } else if distinct + null_bucket < 2 {
        #{ applicable: false, reason: "constant axis: one bucket, NULL included" }
    } else if null_ratio > 0.5 {
        #{ applicable: false, reason: "nulls dominate (ratio > 0.5)" }
    } else if p.cardinality_ratio >= 0.9 {
        #{ applicable: false, reason: "near-key axis (distinct/filled >= 0.9)" }
    } else {
        let nulls = (total.to_float() * null_ratio).round().to_int();
        let non_null = total - nulls;
        let observed = [];
        for b in p.top_values {
            if b.count > 0 { observed.push(b.count); }
        }
        if observed.len() == 0 {
            #{ applicable: false, reason: "no measured buckets in the profile" }
        } else {
            let seen = 0;
            for c in observed { seen += c; }
            // Stale-profile clamp (recorded): counts written in another
            // run than the row count can disagree, and a negative tail
            // must not reach a logarithm.
            let tail = non_null - seen;
            if tail < 0 { tail = 0; }
            // When the observed buckets already account for every
            // non-null row, groups the distinct count claims beyond
            // them hold zero rows — trust the counts over a distinct
            // they contradict.
            let claimed = if distinct > observed.len() { distinct } else { observed.len() };
            let groups = if tail > 0 { claimed } else { observed.len() };
            let coverage = non_null.to_float() / total.to_float();
            if groups < 2 {
                // A single group partitions nothing: evenness 0 by
                // definition (ln(1) would divide by zero).
                #{ applicable: true, relevance: 0.0, coverage: coverage,
                   evenness: 0.0, groups: groups, truncated: false }
            } else {
                let h = 0.0;
                for c in observed {
                    let pr = c.to_float() / non_null.to_float();
                    if pr > 0.0 { h -= pr * ln(pr); }
                }
                if tail > 0 {
                    let pr = tail.to_float() / non_null.to_float();
                    h -= pr * ln(pr);
                }
                let evenness = h / ln(groups.to_float());
                // Clamp at the boundary rather than trusting the
                // arithmetic — two REAL recorded inputs land outside
                // [0, 1]: exactly-uniform k in {5, 13, 19, ...} floats
                // to 1.0000000000000002, and a stale profile whose
                // counts exceed the row count goes negative.
                let r = coverage * evenness;
                if r < 0.0 { r = 0.0; }
                if r > 1.0 { r = 1.0; }
                #{ applicable: true, relevance: r, coverage: coverage,
                   evenness: evenness, groups: groups, truncated: tail > 0 }
            }
        }
    }
}
