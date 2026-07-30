# 04 · Validation `trial_balance` — GRAMMAR GAP + INFORMATION LOST

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

## Transcription — best legal approximation (semantically dishonest, see findings)

```glossql
DECLARE VALIDATION trial_balance
  KIND balance
  ON CYCLE journal_entry_cycle
  OVER (current_assets, operating_expense, current_liabilities, equity, revenue)
  TOLERANCE 0.01
  SEVERITY critical
  GUIDANCE 'Join the trial balance table with the chart of accounts …'
  BY SEED finance;
```

## Gaps

```glossql-gap
DECLARE VALIDATION trial_balance
  KIND balance
  ON CYCLES (journal_entry_cycle, accounts_receivable, accounts_payable)
  RELEVANT CONVENTIONS (sign_natural_balance)
  EXPECTED OUTCOME 'Total debits must equal total credits across all account types.'
  BY SEED finance;
```

## Findings

- check_type/tolerance/severity/guidance: clean — §3.1's envelope was visibly
  designed off this row.
- **GRAMMAR GAP — OVER cannot be filled honestly.** The real operands are
  account-type *families* resolved at bind time (asset+expense vs
  liability+equity+revenue); the OVER list above fabricates concept-shaped
  operands that are not what the check reads. No optional-OVER form exists.
- **GRAMMAR GAP — cycle scope is a list** (`relevant_cycles`, three members;
  empty-means-universal is defined semantics today). §3.1 offers singular
  `ON CYCLE`.
- **GRAMMAR GAP — `relevant_conventions`** (DAT-865): the typed
  validation→convention dependency the SQL binder is fed. No clause references
  declared CONVENTIONs.
- **INFORMATION LOST:** category, tags, `expected_outcome` (a second prose slot,
  distinct from guidance: what passing *means* vs how to *bind*).
- `check_type: expected_formula` + `{table, column, formula}`: §8.3-admitted gap
  (expectation teaches have no statement form).
