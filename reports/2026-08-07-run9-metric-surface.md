# Run 9 — the metric surface, live on run 8's workspace

Date: 2026-08-07. On top of run 8's `fin2` (booksql SQLite, 810k-row
ledger), against the running serverd on 8113 carrying the day's builds
(the metric bind, the wave fix, `datasets`, `DESCRIBE`). Writes landed
as agent `run9-metric-surface`, superseding run 8's agent slot where
they overlap. Two proofs were the run's purpose; both landed.

## 1. The relationship sweep completes where it died

`SELECT detect_relationships() FROM fin2` — the call that exhausted the
fd limit three times in run 8 on this exact workspace and file layout —
completed through the door: **101 candidates, 15 composite**. Among
them, found from the data rather than harvested from declared FKs:

- `ledger.account_name -> accounts.account_name` scoped by
  `business_id` — overlap 1.0, 0 orphans;
- `ledger.product_service -> products.product_service` scoped by
  `business_id` — overlap 0.772, 79 orphan labels (the known
  uncatalogued generics).

That is the recall run 8 lost to the crash, restored by narrowing the
`sql_all` wave from 16 to 4. The candidate list carries plenty of
coincidental garbage (zip codes into transaction ids scoped by
`balance`) — by design; precision is the judge's half.

## 2. Value-at-read serves, composes, and propagates

- **Expansion ≡ inline.** `sum(value)` over `metric.billings()` equals
  the grounding SQL inlined verbatim: 167,743 rows both ways, totals
  agreeing to ten significant digits (the tail is float summation
  order).
- **The recorded evaluation serves.** `metric.dso()` returns run 8's
  monthly cohort-DSO series as an ordinary relation.
- **Composition equals the recording.** dso re-composed live from
  `metric.billings()` and `metric.receivables_open()` per the formula
  matches the recorded evaluation on **all 361 months, 0 mismatches**
  at a 0.005 tolerance — one statement holding three metric expansions.
- **Re-recorded in the composing form.** dso's grounding now reads
  `FROM metric.billings() … JOIN … metric.receivables_open()` (a
  `composition` assumption records the proof), so a re-pinned or
  re-grounded component propagates into dso with no further act. The
  engineer pin and the annualising assumption ride along unchanged.
- `SELECT * FROM datasets` and `DESCRIBE ledger` answer at the live
  door — the latter showing `transaction_date: Utf8` in one screen,
  run 8's finding 1 made visible at last.

## One observation — the two homes, found in the wild

The formulas gloss pins `dso = receivables_open[w] / billings[w] *
days_in(w)`; the recorded evaluation annualises with 365 at month
grain. The two forms of the definition differ — and the framework
already absorbed it: the recording carries the choice as a disclosed
assumption (`365 days when annualising`, agent judgment, 0.85,
unpinned). No machinery needed; the metrics skill now says the rule in
one line — formula and recording are one definition in two forms:
change one, update the other in the same act, or carry the difference
as a disclosed assumption. The 365 convention remains an open pin
question for the engineer.
