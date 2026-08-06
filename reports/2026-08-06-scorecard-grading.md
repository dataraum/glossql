# Scorecard grading — the performance framework, run 7 vs ground_truth.yaml

Date: 2026-08-06. The acceptance run the target scorecard
(2026-08-05) defines: a fresh workspace (`~/glossql-ws-fin`, serverd
on 8114), the agent driven only by the target in words and the
skills — **it never saw the scorecard file or the oracle** (both hold
the answer numbers; the run report's expectations were authored from
the data alone). Graded here against
`../dataraum-testdata/output/clean/ground_truth.yaml`. Run: 8 tables,
zero drops, 297 current glosses, 166 attest rows all green, 7 declared
edges, 11 groundings, 4 validations. One re-land exercised the
day-old supersede-and-reland ruling in production (trial_balance
gained its `period_start` axis; the 9 stale glosses were reviewed and
re-spoken).

## Annual criteria (money ±0.01, ratios ±0.1)

| metric | oracle | agent | verdict |
|---|---|---|---|
| total_expenses | 23,527,077.59 | 23,527,077.59 | **exact** |
| ending_ar_balance | 13,070,114.83 | 13,070,114.83 | **exact** |
| ending_ap_balance | 3,129,373.08 | 3,129,373.08 | **exact** |
| ending_cash_balance | 18,327,138.82 | 18,327,138.82 | **exact** |
| annual_dso | 92.2 | 92.20 | **exact** |
| total_revenue | 51,766,199.72 | 51,742,387.73 | **delta derived**: −23,811.99 = the agent's *stated* interest-income (4310) exclusion, reconciling to the cent |
| free_cash_flow | 18,366,239.07 | 18,327,138.82 | **delta derived**: −39,100.25 = the agent's *own discovered* bank-vs-ledger gap on the Operating account, exact — the oracle's FCF is bank-based, the agent's ledger-based |
| annual_dpo | 48.5 | 50.59 | **delta derived**: denominator family — agent used purchases (vendor-bill credits, 22,578,905.11, argued in the grounding); AP/total-expenses×365 = 48.55 recovers the oracle |
| gross_profit | 28,239,122.13 | 49,959,657.09 | **definitional collision**: agent used textbook revenue − COGS; the oracle computes revenue − *all* expenses (verified per month: 3,590,679.27 − 1,907,083.47 = 1,683,595.80). Delta fully derivable from the agent's served numbers |

The scorecard's criterion: an unexplained mismatch fails; a
defensible alternative with the delta explained is a finding. **Zero
unexplained mismatches.** Every delta above reconciles from numbers
the agent itself reported, before grading.

## Monthly criteria (the series that catches boundary and stock/flow errors)

Spot-checked Jan–Apr against the oracle's monthly block: AR, AP, and
cash balances match **to the cent in every checked month** (e.g. Jan
−169,476.33; Apr 5,456,951.59); expenses exact every month; monthly
DSO within the ±0.1 tolerance every month (24.70/24.7 · 30.11/30.1 ·
34.74/34.7 · 38.08/38.1); monthly revenue differs by the month's
interest slice, consistent with the annual exclusion; monthly DPO and
gross_profit carry the two definitional variants consistently. No
period-boundary or stock/flow error anywhere — the cumulative-level
machinery (stocks as month-end levels, flows as sums) is exact.

## Validation criteria

- journal balanced: 11,754/11,754, max imbalance 1.5e-11 — **met, exactly**.
- trial balance balanced per period: 14/14, and tied to journal
  turnover on 332/332 rows (the promoted behavior_evidence
  reconciliation) — **met**.
- invoice–payment matching: 0.98801 with the 31 partials named and
  status-consistent — **met, with the dirt kept visible**.
- bank reconciliation ≈ 0.8951: the agent authored **0.8951 blind**
  (observed 0.89513, stable 0.869–0.920 monthly) — a four-decimal
  match to an oracle it never saw, plus the 39,100.25 Operating-account
  gap reported rather than smoothed. The anti-overcleaning bar
  ("a framework reporting 1.0 has failed") — **met**.

## Verdict

**Green.** Every hard number the generator defines uniquely was
reproduced exactly; every difference is a stated, reconciled
definition choice; the run's own findings (the turnover-not-balance
trial balance, the AP-not-AR invoices, the bank gap, the empty
opening) are all true properties of the generated world. The
framework the two-roles model promises — correct metric definitions
with validations that say why the numbers can be trusted — exists in
`~/glossql-ws-fin` as glossary content.

## What grading feeds back

1. **Three definitions for the engineer to pin** (the "fixed ground
   measurements" moment the scorecard predicted): does total_revenue
   include interest income; is gross_profit revenue − COGS or
   revenue − all expenses; is the DPO denominator purchases or total
   expenses. Once pinned as the workspace's glosses, they stop being
   the analyst's question.
2. **A scorecard defect**: gross_profit was marked *not*
   definition-sensitive, and it is — the generator's "gross_profit"
   is closer to operating profit. The scorecard row should carry the
   pinned formula when the lead rules it.
3. The oracle's FCF is bank-based; worth stating in the scorecard so
   future runs know which cash the target means.
