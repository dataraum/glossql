# 01 · Concept `revenue` — TRANSCRIBES CLEANLY

Source: `dataraum-context/packages/dataraum-config/verticals/finance/ontology.yaml`

```yaml
- name: revenue
  description: Income from sales or services
  indicators: [revenue, sales, income, turnover, receipts]
  exclude_patterns: [cost, expense]
  kind: measure
  unit_from_concept: currency
```

Pack envelope around it (same file): `name: financial_reporting`,
`version: "1.0.0"`, pack-level `description`.

## Transcription

```glossql
DECLARE CONCEPT revenue
  KIND measure
  DESCRIPTION 'Income from sales or services'
  INDICATORS ('revenue', 'sales', 'income', 'turnover', 'receipts')
  EXCLUDE ('cost', 'expense')
  UNIT FROM currency
  BY SEED finance;
```

The `compositions:` block in the same file (`whole: current_assets`,
`parts: [cash, accounts_receivable, inventory]`) decomposes into edges:

```glossql
DECLARE RELATIONSHIP cash PART OF current_assets BY SEED finance;
DECLARE RELATIONSHIP accounts_receivable PART OF current_assets BY SEED finance;
DECLARE RELATIONSHIP inventory PART OF current_assets BY SEED finance;
```

## Findings

- Concept row: clean 1:1 clause mapping.
- **INFORMATION LOST — pack envelope.** `vertical_envelopes.version` ("1.0.0")
  and the pack description have no statement form; `BY SEED finance` carries the
  seed name only. §6 reserves versioning — deliberate, but drops a stored field.
- The `PART OF` statements were skeleton-underivable in §3.0 as written; covered
  by `relationship_decl` [REPAIR] in grammar.ebnf.
