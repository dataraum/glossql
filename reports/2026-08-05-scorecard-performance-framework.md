# Scorecard: the performance-framework target

Date: 2026-08-05. The first target scorecard (ruled same day: porting
and kernel work are pulled by targets, and the eval framework is as
important as the engine). The engineer-side artifact of the two-roles
model: a run works this scorecard on the generator data; when it is
green, the workspace is the thing that would deploy for analysts, its
metric set validated. Grading is manual-but-recorded for now and
mechanizes when the operating-model phase produces numeric outputs to
diff (prior ruling — no harness before it has a consumer).

## The target, as a user would say it

> Set up a performance framework on this finance workspace: measure
> financial performance — revenue, expenses, gross profit, working
> capital (AR, AP, cash), DSO/DPO, free cash flow — monthly and
> annually, with the validations that tell us the numbers can be
> trusted.

Dataset: `../dataraum-testdata/output/clean/` (8 CSVs, generator seed
42, strategy `clean`). Oracle: `ground_truth.yaml` beside it.

## Preconditions — the correctness floor

Not scored against the oracle; scored by the run's own grounding.
Every miss here surfaces later as a wrong metric:

- **Grain** declared per table (`journal_lines` is line grain;
  `trial_balance` is `(account_id, period)`); no header-value
  multiplication in any aggregate.
- **Relationships** judged and declared (lines→entries,
  invoices→payments, lines→accounts; orphans grounded as populations).
- **Behavior**: balances are stocks (end-of-period, never summed
  across periods); revenue/expense flows sum. The oracle itself
  separates them (`ar_balance` vs `revenue`).
- **Units/signs**: money in one currency after fx; the ledger's sign
  convention stated before any P&L split.

## Metric criteria (oracle: ground_truth.yaml)

Money to ±0.01; ratios to ±0.1 (the oracle's own precision); growth
to ±0.01 pp. Annual AND all 12 monthly values — the monthly series is
what catches period-boundary and stock/flow errors.

| metric | annual oracle | definition-sensitive |
|---|---|---|
| total_revenue | 51,766,199.72 | no — a ledger sum |
| total_expenses | 23,527,077.59 | no |
| gross_profit | 28,239,122.13 | no (revenue − expenses) |
| ending_ar_balance | 13,070,114.83 | no — a stock, end of period |
| ending_ap_balance | 3,129,373.08 | no |
| ending_cash_balance | 18,327,138.82 | no |
| annual_dso | 92.2 | **yes** |
| annual_dpo | 48.5 | **yes** |
| free_cash_flow | 18,366,239.07 | **yes** |
| revenue_growth_pct (monthly) | series | no |

**Definition-sensitive** means legitimate formula families exist
(e.g. DSO variants). The criterion there: match the oracle, or a
grounded derivation of the difference — the agent must state its
formula and reconcile the gap to the oracle's number. An unexplained
mismatch fails; a defensible alternative with the delta explained is
a *finding*, and fixing the ambiguity is exactly what the two-roles
model's "fixed ground measurements" are for: the engineer pins the
definition, and from then on it is not the analyst's question.

## Validation criteria (the oracle's invariants)

- journal balanced: debits == credits, exactly.
- trial balance balanced per period.
- invoice–payment matching holds.
- bank reconciliation rate ≈ 0.8951 (the generator's own dirt — a
  framework reporting 1.0 has overcleaned; that is a failure, not a
  success).

## What this scorecard pulls

The floor ports (`entity`, `behavior_evidence`) serve every criterion
above; nothing in this target needs the dimensional superstructure
(no bus matrix, no slicing) — which is the point of target-first: a
cost-drivers target would pull those, this one does not. Later
strategies of the generator (fault injection) turn the same scorecard
into a robustness eval; held-out months turn it into a prediction
oracle; parameter-intervened re-generation into a what-if oracle no
production dataset can offer.
