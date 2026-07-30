# 04 · Validation `trial_balance` — envelope RESOLVED (sprint 7, fork A); OVER honesty open

Source: `validations` table (engine schema.sql) — fields: validation_id, name,
description, category, severity, check_type ∈ (aggregate|balance|comparison|
constraint), tolerance, guidance, expected_outcome, relevant_cycles JSON,
relevant_conventions JSON, tags, version, source. Seed YAML (worktree
epic-dat-853, `finance/validations/trial_balance.yaml`):

```yaml
validation_id: trial_balance
name: Trial Balance (Accounting Equation)
description: >
  Validates the expanded accounting equation:
  Assets + Expenses = Liabilities + Equity + Revenue. …
category: financial
severity: critical
version: "1.1"
tags: [accounting, trial-balance, balance-sheet, equation]
relevant_cycles: [journal_entry_cycle, accounts_receivable, accounts_payable]
check_type: balance
expected_outcome: >
  Total debits must equal total credits across all account types. …
```

## Transcription — decided envelope (sprint 7, fork A)

```glossql
DECLARE VALIDATION trial_balance
  KIND balance
  ON CYCLES (journal_entry_cycle, accounts_receivable, accounts_payable)
  OVER (current_assets, operating_expense, current_liabilities, equity, revenue)
  CONVENTIONS (sign_natural_balance)
  TOLERANCE 0.01
  SEVERITY critical
  GUIDANCE 'Join the trial balance table with the chart of accounts …'
  OUTCOME 'Total debits must equal total credits across all account types.'
  BY SEED finance;
```

## Findings

- **RESOLVED (sprint 7, fork A):** `ON CYCLES` is a list, absent = universal;
  `CONVENTIONS` carries the validation→convention dependency, membership-
  checked like `OVER` (the load-bearing pull direction — `validation_phase.py`
  errors on unresolved ids); `OUTCOME` is the second prose slot, kept separate
  from `GUIDANCE` as the binder consumes them.
- **OPEN — OVER cannot be filled honestly.** The real operands are account-type
  *families* resolved at bind time (asset+expense vs liability+equity+revenue);
  the OVER list above fabricates concept-shaped operands that are not what the
  check reads. No optional-OVER form exists. Sprint candidate.
- **INFORMATION LOST (accepted):** category, tags — browsing metadata; revisit
  as generic aspects if it ever earns a mechanism.
- `check_type: expected_formula` + `{table, column, formula}`: §8.3-admitted gap
  (expectation teaches have no statement form).
