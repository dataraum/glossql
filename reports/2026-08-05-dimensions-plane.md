# The dimensions plane — port list item 3

Date: 2026-08-05, green-lit by the project lead the same day (with
faer pre-authorized should algebra be needed — it was not: Pielou is a
log sum, the FD screens are GROUP BYs; the numeric-kernel question
stays parked until drivers). Three parts, one skill, two shipped
measurements, both landed under the standing rule — no statistic
without its oracle.

## What shipped

- `functions/dimension_relevance.rhai` — v0.3's
  `analysis/slicing/relevance.py` transcribed: `relevance = coverage ×
  evenness` (Pielou), zero free parameters, chained on the profile
  through `ACCEPTS (column_profile)` — no scan of its own, and the
  missing-profile abstention heals through the ACCEPTS edge. The
  falsification rides in the comments (the effective-group ratio
  scored a 99/1 boolean at 0.53; Pielou separates 0.08 vs 0.62), as do
  the recorded boundary cases (uniform k floating past 1.0, stale
  profiles going negative — both clamped). The truncated tail is one
  bucket: a truncated score is a lower bound, under-claiming by
  design. Admission gates are the recorded inventory: two buckets with
  NULL as one, null ratio ≤ 0.5, near-key as a fraction ceiling 0.9
  (the absolute-count version was a recorded bug).
- `functions/hierarchies.rhai` (`detect_hierarchies`, ON TABLE) — the
  cheap SQL core of the hierarchy stack, dispositioned by the recall
  ruling: null-as-category ported; the g3 row screen ported loose
  (ship at ≤ 0.05, v0.3 asserted at 0.01 — the band between is the
  judge's); Goodman–Kruskal λ *served beside every candidate, never
  gated* — λ < 0.5 is the recorded vacuous-skew signature (the
  pre-registered floor killed 48 false positives, zero truth lost),
  and the judge reads it; permutation nulls + FDR not ported
  (precision apparatus for judge-less operation). Measures stay out by
  dtype (Float/Decimal) — the additivity lane floods FD discovery.
  Guards are full-scan (the rel-hm fold-key lesson). A both-ways 1:1
  ships as `kind: alias`; the identity call stays with the reader.
- Bootstrap: both aspects (`dimension_relevance` ON COLUMN,
  `hierarchy_candidates` ON TABLE) and function declarations; the
  serverd embed and boot test extended (8 functions, 7 aspects).
- `crates/scripts/tests/dimensions.rs` — truth by construction: a
  50/30/20+nulls axis scores exactly 0.8520, a per-row key abstains as
  near-key, the missing-profile abstention heals through ACCEPTS; a
  zip→city→state nest arrives with g3 = 0 and λ = 1, reversals are
  screened, a code↔label bijection arrives as alias both ways, and a
  98%-dominant flag survives the g3 screen carrying the λ < 0.5
  signature the judge kills it by.
- `crates/scripts/tests/dimensions_oracle.rs` — the generator grades
  the score with numbers it never sees: `invoices.status` scores the
  Pielou of the yaml's own monthly invoice_count totals (0.3682,
  verified equal to the CSV distribution), and
  `bank_transactions.reconciled` reads back the generator's stated
  0.8951 reconciliation rate as 0.4842 — the same deliberate dirt the
  performance scorecard treats as a fidelity check.
- `.claude/skills/glossql-dimensions/SKILL.md` — the deliverable's
  method: frame the `dimension` FACT aspect (absolute labels; the
  retired-ordinal lesson), score axes and judge interest (the number
  never overrules business judgment), judge hierarchy candidates (the
  λ signature; alias-vs-coincidence separated only by meaning, never
  merged silently; same-family role columns never merged; transitive
  reduction), record nests as same-table relationships finer→coarser,
  and build enriched views behind the grain check — equal counts
  exactly, or the join stays out (v0.3 failed the run rather than
  ship a fan-out view).

## Placement notes

- Hierarchy nests reuse `DECLARE RELATIONSHIP` on same-table pairs —
  the relationships relation, pair-path glosses, and aspect grain all
  work unchanged, and the behavior-evidence anchor discovery is inert
  to same-table edges (verified in its traversal guards).
- The relationships skill's manual re-measure teaching
  (`DELETE FROM cache`) is retired — `detect_relationships` now heals
  through its `imports` ACCEPTS edge.

## Flagged, not fixed

- **The hierarchy oracle is synthetic.** The finance set has no true
  column-pair FD nest, so hierarchies grade on constructed truth only.
  RelBench (rel-f1's circuit→country and friends, with declared-FK
  metadata — the data v0.3 validated this stack against) is the real
  oracle and is not downloaded yet; landing it is
  `../dataraum-testdata` work, flagged there.
- **Cross-table conform (the bus-matrix floor) is out of this slice**,
  per the port list ("port later, simplified"). The skill carries one
  sentence of conform discipline (concept named in prose) so views
  don't silently improvise it.
- **Numeric banded axes are not scored.** v0.3 scored banded numeric
  slices onto the same scale; the banding machinery itself never
  ported. A numeric measure column abstains through the near-key gate
  today. Revisit when a target pulls it.
