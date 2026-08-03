# 04 · Validation `trial_balance` — TRANSCRIBES (aspect + witness, no dedicated construct)

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

## Transcription

The authored expectation is a FACT gloss; the check is a function; the witness
binds them; ATTEST is the verdict surface. No validation construct exists.

```glossql
DECLARE ASPECT trial_balance WITH $${
  "type": "object",
  "required": ["outcome"],
  "properties": {
    "tolerance": {"type": "number"},
    "severity": {"enum": ["critical", "warning", "info"]},
    "outcome": {"type": "string"},
    "guidance": {"type": "string"},
    "cycles": {"type": "array", "items": {"type": "string"}},
    "conventions": {"type": "array", "items": {"type": "string"}}
  }
}$$ AS FACT;

GLOSS trial_balance ON fin AS $${
  "tolerance": 0.01,
  "severity": "critical",
  "outcome": "Total debits must equal total credits across all account types.",
  "guidance": "Join the trial balance table with the chart of accounts before summing by account type.",
  "cycles": ["journal_entry_cycle", "accounts_receivable", "accounts_payable"],
  "conventions": ["sign_natural_balance"]
}$$;

DECLARE FUNCTION trial_balance_check FOR fin FROM 'functions/trial_balance.py'
  RETURNS $${
    "type": "object",
    "required": ["subject", "aspect", "witness", "band", "score", "computed_at"],
    "properties": {
      "subject": {"type": "string"},
      "aspect": {"type": "string"},
      "witness": {"type": "string"},
      "band": {"enum": ["green", "yellow", "orange", "red"]},
      "score": {"type": "number", "minimum": 0, "maximum": 1},
      "computed_at": {"type": "string", "format": "date-time"}
    }
  }$$;

DECLARE WITNESS tb ON trial_balance BY (FUNCTION trial_balance_check, HUMAN)
  DETECTOR trial_balance_check;

SELECT * FROM ATTEST(fin.trial_balance);
```

## Findings

- **TRANSCRIBES — the strongest confirmation in the corpus.** The old spec
  needed `DECLARE VALIDATION` with six clauses (KIND / ON CYCLES / OVER /
  CONVENTIONS / TOLERANCE / SEVERITY / GUIDANCE / OUTCOME); here the same
  artifact is an aspect, a gloss, a function, and a witness — all
  general-purpose constructs.
- The check reads its own expectation (tolerance, cycles) from the glossary —
  the function implicitly receives its subject; the gloss is data.
- `trial_balance_check` doubles as value function and detector: legal because
  its RETURNS conforms to the standard attest schema.
- **INFORMATION LOST (accepted):** category, tags, version — browsing
  metadata, same relocation as fixture 03.
