# The behavior plane — behavior_evidence, the port list's second slice

Date: 2026-08-05. Port list item 2 (begin-session analysis, same day),
built under three rulings from the project lead: the name stays
`behavior_evidence` (evidence for the `behavior` verdict, never the
verdict — the FACT aspect keeps its agent and human voices, RETURNS
onto FACT stays unexercised); the aspect declares `ON COLUMN` (evidence
lands on the column being glossed, `unassessed` disclosure bounded by
the same-day grain ruling); anchors come from **declared relationships
only** (the plane order is relationships first, and the wrong-anchor
gate then guards misdeclared edges rather than a combinatorial sweep).

## The empirical surprise, pinned first

Run 3's "trial-balance-is-turnover" finding is now generator-verified
to the cent: for every account and month, `trial_balance.debit_balance`
equals that month's `SUM(journal_lines.debit)` — per-period turnover,
not a carried level. Two consequences shaped the slice:

- The oracle's sharpest assertion is a **name lie**: a column named
  `debit_balance` must read *flow*. A name-based judge says stock; the
  discriminator reads the data.
- The clean strategy lands **no true stock column at all** — the
  oracle derives one (`account_balances`, a running window sum over the
  same CSV) so the stock arm is graded on generator data too. A future
  generator strategy emitting a real balance-carrying table would make
  this unnecessary; noted for `../dataraum-testdata`.

## What shipped

- `functions/behavior_evidence.rhai` — v0.3's
  `analysis/lineage/reconcile.py` transcribed with its constants and
  their provenance in place: two scale-free residuals (flow `y ≈ m`,
  stock `Δy ≈ m`), the wrong-anchor gate at 0.5, the near-tie margin
  **derived** from it (1/3 — the coupled-gates warning is in the
  comments), entity voting (≥2, ≥0.8), abstention on short and dead
  series. Around the arithmetic, the candidate machinery v0.3 kept in
  its processor: movement conventions as each numeric event column plus
  every ordered pair difference, evaluated as dict arithmetic over
  stored per-period sums (linearity of SUM, no extra scans);
  support-first selection by Wilson lower bound over the pairing's
  common entity denominator (the recorded support-gameability trap:
  sparse pair differences out-race true singles on subsets); a
  single beats an equally supported pair. Time axes are a table's own
  date columns or one declared hop away (`journal_lines` borrows
  `journal_entries.date` through `entry_id`); entity alignment is a
  direct declared edge or two edges meeting at one dimension key;
  bucket grain is the coarsest named grain the measure axis is already
  truncated to, calendar month otherwise. Every anchor is served —
  verdicts, abstentions with reasons, runner-up conventions — and the
  judge reads all of it.
- Bootstrap: the `behavior_evidence` aspect (`AS MEASUREMENT ON
  COLUMN`) and function declaration; the serverd embed and boot test
  extended (6 functions, 5 aspects).
- `crates/scripts/tests/behavior_evidence.rs` — truth by construction:
  a running balance reads stock, its movement reads flow, a constant
  noise column abstains through the wrong-anchor gate.
- `crates/scripts/tests/behavior_oracle.rs` — the standing rule ("no
  statistic ports without its oracle") applied: lands five recipes over
  the finance CSVs, declares the four edges, and asserts the name lie
  (flow, convention `debit`, r_flow < 0.01), the derived stock
  (r_stock < 0.01), the pair-difference convention earning its keep
  (`net_amount` reconciles against the trial balance only as
  `debit_balance - credit_balance` — neither single fits), and whole
  abstention on `fx_rates.rate` (no declared edge → `applicable:
  false`, no improvised anchors). Skips when the sibling checkout is
  absent, so the workspace invariant stays self-contained.
- Skill prose: `glossql-add-source`'s behavior rule now reads the
  evidence first, carries the falsification lesson (a column's own
  trajectory cannot decide), keeps run 3's precedent (the agent may
  out-judge the measurement against the ledger), and teaches the cache
  recomputation after new edges.

## A runtime change riding along, flagged for review

rhai's expression-depth limits **halve in debug builds**
(rhai-1.25.1 `limits.rs:17` vs `:32`): the script parsed under release
defaults and failed under `cargo test`. `RhaiRuntime::new` now pins the
release defaults (`set_max_expr_depths(64, 32)`) so both builds run the
same contract — same spirit as the existing `set_max_operations`
backstop, but it is a server-surface line and should be seen.

## Flagged, not fixed

- **Relation writes don't invalidate.** ~~`ACCEPTS` edges are
  aspect-only~~ — ruled same day (project lead: "if the logic is
  already there, why not — caches repopulate on the next call") and
  landed: `ACCEPTS` admits the wired declaration relations
  (`relationships`, `imports`) as invalidation edges. No context entry
  arrives — the script reads them as tables — but a declared edge or a
  landed import kills the accepting function's cache dataset-wide
  through the same `invalidate()` path an aspect value uses.
  Abstentions heal on their own; the skill's manual
  `DELETE FROM cache` teaching is retired. An unwired relation name is
  refused rather than becoming a silent no-op edge; the rest join as
  consumers appear. SPEC.md §6 carries the two-sentence diff.
- **Intersection pairing.** Series pair on (entity, period) cells
  present on both sides; a calendar gap between kept cells makes Δy
  span it and read as noise. v0.3's calendar-complete period
  enumeration did not port; honest abstention absorbs the miss.
- **Composite endpoints skipped.** Tuple paths in `relationships` are
  ignored by anchor discovery; the composite cure (keyed view, then
  declare) produces simple paths anyway.
- **One-hop time borrow.** An event table whose date lives two joins
  away finds no axis; no current dataset needs the second hop.
