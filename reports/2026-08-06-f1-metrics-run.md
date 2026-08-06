# Run 6 — the four flows on f1, first grading of the metric framework

Date: 2026-08-06. A fresh workspace over RelBench rel-f1, driven
end to end through add-source → relationships → dimensions →
glossql-metrics — the first live run over the operating-model
deliverable, hours after it landed (fixture 16, the skill, the flow
tests). 9 tables, 97,606 rows, 232 glosses, 14 declared edges, 201
witnessed slots all green. The workspace (`~/glossql-ws`) was
inspected after the run; every load-bearing claim below is verified
against the store, not just the transcript.

## The framework held — every piece from the day's landings exercised

- **Grain-free extracts, verified in the store**: `race_points`
  grounds as a row-grain SELECT carrying the time axis, season,
  round, and the judged dimensions as columns — joins inline, no
  GROUP BY anywhere. 5 base concepts, 5 derived metrics, a
  `formulas` gloss.
- **The validation shape ran for real**: three checks
  (`constructor_points_check`, `standings_points_check`,
  `result_grain_check`) written into the workspace as FOR-dataset
  functions, speaking their aspects' schemas (`outcome` + the
  measurement), `framework_bands` adjudicating the authored
  expectation beside the voice. Store shows compared/matched/
  observed_rate per check and green at score 0.0 on all three
  witnesses.
- **Expectations authored at observed rates** — the 0.895
  anti-overcleaning stance, generalized without being prompted: the
  reconciliations hold at 97.2% and 98.3%, *and should not be 1.0* —
  the residuals are the 1958–78 best-car-only rule, the
  dropped-scores era, and the sprint window. The authored gloss says
  so in words ("not all of them. Two eras break it structurally").
  A check reporting 1.0 there would have stopped seeing an era.
- **The judged negative used correctly on day one**: `car_number`
  scores 0.76/0.89 on even spread and was glossed `none` — it
  identifies an entry, not a thing whose performance is compared.
  Exactly the score-vs-interest split the enum was added for. Exact
  relevance (no truncation) on all 25 axes.
- **Judgment over recall, again**: 316 relationship candidates
  (dense 0..N surrogates all "resolve" into each other at overlap
  1.0) judged down to 13 real edges, zero orphans both directions,
  absentee populations verified rather than assumed. The
  `circuit_id → race_name` bait rejected on meaning (Nürburgring:
  four Grand Prix names; the French GP: seven venues).

## What the run found in the data

- **The sprint-points trap**: `results` holds Grand Prix points
  only — sprint points exist solely in the standings tables (3
  rounds/season from 2021; 3 drivers×3 points in 2021, 8×8 in
  2022–23, matching the published sprint tables). Any points metric
  off `results` understates the championship from 2021 — Red Bull
  2022: 724 vs the official 759. Found by gap analysis, confirmed
  against publication, carried in the glosses.
- **`grid_penalty` vanishes globally by arithmetic identity**: a
  grid is a permutation of the qualifying order, so the global mean
  is ~0 (−0.0169 over 9,750 rows) — discovered by *evaluating* the
  column, and the gloss corrected to per-entity or positive-tail
  reads. Evaluate-before-gloss, paying off.
- **Oracle spot-checks reproduce published history exactly**:
  Verstappen 454 points / 15 wins in 2022, 395.5 in 2021 with the
  half-points Belgian round, the real 2022 constructors' order.

## The one wall — table replacement is now pulled

`drivers.driver_code` carries 757 literal `\N` sentinels, caught
after landing. `DROP TABLE` is refused while a table holds data
(by design), and the agent reasoned correctly that a `drivers_v2`
would leave a dead `drivers` for every downstream flow to trip
over — so it kept the table and documented the sentinel in the
gloss. This is the **second consecutive run** to hit exactly this
wall (run 5: the same column). Two independent runs, same dead end,
correct agent reasoning both times: the replacement flow is pulled
by evidence, no longer a postponement without cost. The fork is the
lead's: re-declaring the same-name recipe as supersede-and-reland ·
allowing DROP after the glossary rows migrate or invalidate · a
dedicated replace form. Not decided here.

## Status against the target

This run graded the framework on f1 with published history as the
oracle. The performance-framework scorecard — the acceptance test,
on the finance generator with `ground_truth.yaml` — is still the
run ahead.
