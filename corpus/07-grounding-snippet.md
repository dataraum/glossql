# 07 · Grounding / `sql_snippets` — core CLEAN; supersession key GRAMMAR GAP

Source: `sql_snippets` (engine schema.sql), semantic key:

```sql
CONSTRAINT uq_snippet_semantic_key UNIQUE (snippet_type, standard_field,
  statement, aggregation, predicate, schema_mapping_id, parameter_value)
```

`parts` (DAT-671, `query/snippet_models.py`): `{select: [{expr, alias}],
from: [relation], where: [pred, …]}` — "the parts ARE the artifact; sql is their
one-time render", plus a `period_binding`. `provenance` on healthy rows:
`{column_mappings_basis: {concept: {measure_columns, filter_columns, filter,
filter_members}}, assumptions: [{dimension, assumption, basis, confidence}]}`;
retained failures carry `failure_mode ∈ (execution_failed, verifier_rejected,
provenance_invalid, disjoint_collision)` + `failure_reason`.

## Transcription — the extract core (clean)

```glossql
DECLARE GROUNDING accounts_receivable IN journal_lines_enriched
  AS sum(debit_amount) - sum(credit_amount)
  WHERE account_type = 'asset'
  BY AGENT grapher CONFIDENCE 0.9;
```

## Gap — the real key members have no syntax

```glossql-gap
DECLARE GROUNDING accounts_receivable IN journal_lines_enriched
  FOR STATEMENT balance_sheet
  AS sum(debit_amount) - sum(credit_amount)
  WHERE account_type = 'asset'
  BY AGENT grapher;
```

## Findings

- Core mapping concept/relation/expression/filter → GROUNDING clauses is real
  and direct (`standard_field`→concept, parts.from→IN, parts.select→AS,
  parts.where→WHERE); columns-used and rendered SQL do derive from the AST as
  §3.2 claims. **CLEAN** for the extract core.
- **GRAMMAR GAP — the supersession key cannot be written.** Spec key: (concept,
  relation, parameter) — the *parameter* member has no surface syntax anywhere
  (grammar.ebnf issue U3). Real key adds *statement* and *predicate*: dso needs
  `accounts_receivable` on the balance-sheet axis while a P&L metric needs the
  income-statement axis over the same relation (§8.2 punts this to part-of);
  DAT-838 makes `predicate` a key member (same field+aggregation, different row
  restriction = different measurement).
- **INFORMATION LOST — per-assumption records.** `assumptions[]` each carry
  dimension/basis/confidence and feed the DAT-631 weakest-grounding confidence
  gate; statement-level CONFIDENCE is one number and §3.0 makes it
  non-adjudicating metadata while today's gate is a consumer mechanism.
- **INFORMATION LOST — retained failures.** `disjoint_collision` etc. are
  negative knowledge fed back so the agent doesn't re-author a rejected
  grounding. RELATIONSHIP has `REJECTED`; GROUNDING has no negative form.
