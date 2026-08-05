# First agent acceptance run — add-source through the MCP door

Date: 2026-08-05. One agent (Opus, Claude Code as MCP client, the three
skills, a bootstrapped workspace, eight finance CSVs) drove the
add-source flow end to end. Project lead's verdict: "scary good."

## What the agent did

- 1 dataset, 1 csv source, 8 recipes — 48,877 rows landed, 0 dropped,
  every authored cast parsed 100% on probe before landing.
- 8 relationships, each verified by anti-join before `DECLARE
  RELATIONSHIP`; the 140 apparent orphans in `invoices.entry_id` were
  grounded as exactly the 140 cancelled invoices (never posted).
- Measurement plane: 51 `profile()`, 9 `outliers()`, 6 `temporal()`.
- 128 glosses: `meaning` on all 51 columns + 8 tables, `role` on all
  51, `behavior`/`unit` on the 9 genuine measures. `role` attests green
  across the board; the remaining unassessed rows are structural
  absences, not gaps.
- Two findings grounded rather than assumed: `invoices` is accounts
  *payable* (entries hit Trade Payables; narrations read "Vendor
  payment"; all payment-linked bank movements negative), and
  `trial_balance` carries **period turnover, not balances** (February's
  ledger debits tie to `debit_balance` to the cent) — glossed `flow`
  *against* the skill's guidance, correctly.

## The defect the run surfaced

`run_recipe` folded every CSV/JSON result to Utf8 **after** the
recipe's SQL ran (`normalize::force_utf8` — its only caller): authored
casts computed, then discarded; all eight tables landed as strings.
Consequences the agent measured: string-ordered comparisons
(`amount > '1000'` → 2995 vs the true 1877), `max(amount)` =
`"9984.52"` vs 49751.36, and `temporal()` abstaining on all six date
columns (it types the column, sees Utf8). Sharpest form of the
contradiction: `run_probe` did **not** fold — the `LIMIT 0` rehearsal
promised `Float64`/`Date32` and the import broke the promise.

`force_utf8` was retired raw-twin machinery (its own doc: "typing being
the typed view's business") that outlived the 2026-08-04
authored-typing ruling. Fixed same day: CSV/JSON route through
`compat` like every source, `force_utf8` deleted, and the import test
respelled to the two truths that both hold — an *uncast* csv column
stays byte-exact raw text (the read side is all-Utf8; leading zeros
survive), an *authored* cast is the landed type. New regression: a csv
`try_cast` recipe lands `Float64`, matching its own probe.

## What held

The refusal rules did exactly their job mid-incident: a changed recipe
was refused, `DROP TABLE` was refused while data and glosses exist, and
the agent stopped rather than forcing — repair is a fresh workspace and
a re-run (the deletion cascade stays recorded future work). Batch
writes, the session-per-actor plane, the metadata-uncapped reads, and
the statement monitor all behaved as designed.

## Skill lesson folded back

The `behavior` guidance asserted "a trial-balance line is still a
stock" — this dataset disproved it, and the agent's grounding beat the
static wire (the model working as intended: judgment lives in reads,
not in taught rules). The line now instructs instead of asserting:
tie a trial-balance column to the period's movements before calling it
a stock.

## Second run (same day, fixed build): green

Fresh workspace, same eight CSVs. Everything the fix promised held:
recipes landed typed (ISO dates → DATE, money → DOUBLE, reconciled →
BOOLEAN; account codes deliberately kept VARCHAR — identifiers, not
numbers), all six `temporal()` profiles fired, and the agent's
reconciliations tie to the cent — which only arithmetic types can do.
48,877 rows, 0 dropped; 51 profiles, 9 outliers, 6 temporals; 128
glosses; 8 relationships including the chart-of-accounts
self-hierarchy; ATTEST all green.

The trial-balance trap recurred and the respelled skill line worked as
method: the agent tied all 332 rows to the period's journal postings
before glossing `flow`, and wrote the naming hazard into the `meaning`
prose. Deliberate absences stayed visible as unassessed rows
(behavior/unit on non-measures, role on tables) — silence as the honest
answer, exactly the disclosure design. Dataset facts surfaced for the
owner: `invoices.vendor_id` references 20 vendors with no vendor table
in the source; `fx_rates` is unused reference data (every amount USD).

What this run does **not** yet test: adjudication under disagreement.
One agent, no human voice — every witness saw one slot, so
`slot_entropy` never had to arbitrate, and no supersession or contested
collapse occurred. That is fixture 12's judge-pattern territory, and
§9's open experiment (agents composing context *from* the reads) also
still wants the answer-agent side, not the add-source side.

## Next

Fixture 12's judge pattern meets the doors: a human voice disagrees, a
slot contests, the detector bands it, a human closes it. The §9
answer-agent experiment is deliberately later (ruled 2026-08-05) —
fixture 12 lands first.
