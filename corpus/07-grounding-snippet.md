# 07 · Grounding / `sql_snippets` — key RESOLVED (sprint 2, fork A); two losses remain

Source: `sql_snippets` (engine schema.sql), semantic key:

```sql
CONSTRAINT uq_snippet_semantic_key UNIQUE (snippet_type, standard_field,
  statement, aggregation, predicate, schema_mapping_id, parameter_value)
```

Verified 2026-07-30: `schema_mapping_id` ≈ workspace (DAT-506); `parameter_value`
is constants-only (not groundings); the statement axis has exactly two values in
the finance vertical; relation is not a key member; grounding is one extract per
concept per run (`grounding_collision.py`). `provenance` on healthy rows carries
`assumptions: [{dimension, assumption, basis, confidence}]`; retained failures
carry `failure_mode ∈ (execution_failed, verifier_rejected, provenance_invalid,
disjoint_collision)`.

## Transcription — decided key: (concept); row-level reading

```glossql
DECLARE RELATIONSHIP accounts_receivable PART OF balance_sheet BY SEED finance;

DECLARE GROUNDING accounts_receivable IN journal_lines_enriched
  AS debit_amount - credit_amount
  WHERE account_type = 'asset'
  BY AGENT grapher CONFIDENCE 0.9;

DECLARE METRIC dso
  AS 90 * avg(accounts_receivable) / sum(revenue)
  UNIT 'days'
  BY SEED finance;
```

## Findings

- **Supersession key — RESOLVED (fork A).** Key is the concept; one active
  grounding per concept per workspace; re-grounding supersedes (correction is
  addressable). The statement axis is `PART OF` structure; a differently-
  filtered reading is its own concept (`reconciled_count`); aggregation is
  owned by the metric expression — the spec's double ownership is repaired.
  DISJOINT concepts with byte-identical grounding bodies are rejected at
  admission (the collision guard, moved to declaration time). grammar.ebnf U3
  is closed — the parameter member is gone, constants derive from `PARAMETER`
  declarations.
- **INFORMATION LOST — per-assumption records** (open). `assumptions[]` feed
  the DAT-631 weakest-grounding confidence gate; statement-level CONFIDENCE is
  one number and §3.0 makes it non-adjudicating metadata while today's gate is
  a consumer mechanism. Sprint candidate.
- **INFORMATION LOST — retained failures** (open). `disjoint_collision` etc.
  are negative knowledge that stops re-authoring rejected groundings.
  RELATIONSHIP has `REJECTED`; GROUNDING has no negative form. Sprint
  candidate.
