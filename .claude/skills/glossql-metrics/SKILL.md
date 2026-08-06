---
name: glossql-metrics
description: Define the metric and validation framework on a glossed glossql workspace — concepts ground as grain-free extracts, derived metrics as formulas, validations as expectation + check voice + ATTEST. Use when the target asks for performance monitoring, after the add-source, relationships and dimensions flows.
---

# The metric framework

The operating-model deliverable: metrics the business trusts and the
validations that say why. Everything below rides existing constructs —
QUERY aspects, glosses, functions, witnesses. The governing rule:
**nothing is evaluated before a reader asks; everything a reader
proves may be recorded.**

## 1. Read the floor first

Every grounding cites the judged knowledge underneath it. Before
writing any SQL: no summed term without a `behavior` gloss under it
(`behavior_evidence` first), no join without its grain-check gloss on
the relationship, the sign convention stated before any P&L split,
units checked before cross-currency arithmetic. A grounding whose
assumptions cannot name their bases is not ready to write.

## 2. The vocabulary

One QUERY aspect per concept, on the dataset. Base concepts and
derived metrics declare uniformly — the difference is whether the SQL
half is an extract (§3) or a formula over siblings (§4):

```glossql
DECLARE ASPECT revenue WITH $${
  "title": "Revenue", "x-kind": "measure", "x-unit": "currency"
}$$ AS QUERY ON DATASET;
DECLARE ASPECT dso WITH $${
  "title": "Days Sales Outstanding", "x-kind": "metric", "x-unit": "days"
}$$ AS QUERY ON DATASET;
DECLARE ASPECT formulas WITH $${
  "type": "object", "properties": {"formulas": {"type": "object"}}
}$$ AS FACT ON DATASET;
```

## 3. Ground concepts as grain-free extracts

A grounding carries **no grain** — no GROUP BY, no window. It is the
semantic core: scoping, signs, the grain-preserving joins composed
inline, served as a row-grain relation with the time axis and the
judged dimensions as columns. Every assumption names its basis:

```glossql
GLOSS revenue ON fin AS $${
  "sql": "SELECT e.date, l.credit - l.debit AS value, l.cost_center FROM journal_lines l JOIN journal_entries e ON l.entry_id = e.entry_id JOIN chart_of_accounts a ON l.account_id = a.account_id WHERE a.account_type = 'revenue'",
  "assumptions": [
    {"dimension": "sign", "assumption": "revenue accounts carry credit balances", "basis": "conventions gloss", "confidence": 0.95},
    {"dimension": "grain", "assumption": "joins are grain-preserving", "basis": "relationship glosses", "confidence": 1.0},
    {"dimension": "behavior", "assumption": "a flow: sums valid over any partition", "basis": "behavior_evidence on journal_lines.credit", "confidence": 0.95}
  ]
}$$;
```

A stock's extract is bounded by its **source grain** (a trial balance
speaks per period; no read can answer finer) — serve the grain column
as-is and say so in the assumptions.

## 4. Evaluate at read — windows are read policy

Grain is the reader's: the app defaults to month, another reader asks
by day, the same definitions answer both. Compose the evaluation from
the served SQL through the door (until the `metric.` table-function
bind lands, inline it):

- **Flows sum** over any partition — time window or judged dimension.
- **Stocks take the last period per window**, never a sum across.
- **Ratios don't roll up**: compose them per the formula at the
  window asked — `dso[w] = accounts_receivable[end of w] /
  revenue[w] * days[w]`. The formula gloss is the pinned definition;
  it covers every window because it names none.

**Record what a read proves.** A composed evaluation you verified
(against the oracle, against the ledger) may land as the metric's own
QUERY gloss — durable executable knowledge, superseding as
definitions change. Recording a proven read is not pre-evaluation.

## 5. Validations — expectation beside check, ATTEST answers

The authored expectation is a FACT gloss; the check is a function
**voice** on the same aspect; a detector bands across both slots;
`ATTEST` is the verdict surface:

```glossql
DECLARE ASPECT journal_balanced WITH $${
  "type": "object", "required": ["outcome"],
  "properties": {"outcome": {"type": "string"}, "tolerance": {"type": "number"},
                 "severity": {"enum": ["critical", "warning", "info"]}}
}$$ AS FACT ON TABLE;
GLOSS journal_balanced ON journal_lines AS $${
  "outcome": "Total debits equal total credits, exactly.",
  "tolerance": 0.0, "severity": "critical"
}$$;
DECLARE FUNCTION journal_check FOR fin FROM 'functions/journal_check.rhai'
  ACCEPTS (imports) RETURNS journal_balanced;
DECLARE FUNCTION framework_bands FOR fin FROM 'functions/framework_bands.rhai';
DECLARE WITNESS journal_w ON journal_balanced BY (AGENT, HUMAN)
  DETECTOR framework_bands THRESHOLD 0.5;
SELECT journal_check() FROM journal_lines;
```

- **The expectation is authored, never assumed zero.** A source with
  known dirt expects its own rate (`"expected_rate": 0.895`) — a
  check reporting 1.0 there has overcleaned, itself a failure.
- **The check speaks the aspect's schema**: its output carries
  `outcome` like any slot, with the measurement beside it. One
  schema, every speaker.
- `ACCEPTS (imports)` keeps it honest: a new import invalidates the
  voice, and the next read recomputes.
- **Promote confirmed reconciliations.** A behavior_evidence
  convention that reconciled at ~0 residual (a balance equal to the
  sum of its journal lines) is a standing invariant — turn it into a
  check.
- Checks and detectors are workspace-authored (`FOR` the dataset,
  not GLOBAL) — write them per the glossql-functions skill.

## 6. Read back

```glossql
SELECT subject, band, score FROM ATTEST(fin) WHERE band = 'red';
SELECT count(*) FROM GLOSSARY(fin) WHERE state = 'unassessed';
```

Red bands are where a human closes what you could not; unassessed
rows are the vocabulary nobody has spoken to yet.
