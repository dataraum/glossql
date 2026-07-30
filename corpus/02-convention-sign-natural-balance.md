# 02 · Convention `sign_natural_balance` — GRAMMAR GAP

Source: `dataraum-context/packages/dataraum-config/verticals/finance/ontology.yaml`

```yaml
- id: sign_natural_balance
  targets: [extraction, qa]
  statement: >
    Express every monetary measure in its natural-balance direction …
    never normalize only one side of a comparison.
  concept_groups:
    credit_normal: [revenue, accounts_payable, current_liabilities, equity]
    debit_normal: [cost_of_goods_sold, operating_expense, depreciation, tax,
                   accounts_receivable, inventory, current_assets, cash]
```

## Transcription — what §3.1 allows

```glossql
DECLARE CONVENTION sign_natural_balance
  STATEMENT 'Express every monetary measure in its natural-balance direction …'
  BY SEED finance;
```

## Gaps — two real fields have no clause

```glossql-gap
DECLARE CONVENTION sign_natural_balance
  STATEMENT '…'
  TARGETS (extraction, qa)
  GROUP credit_normal (revenue, accounts_payable, current_liabilities, equity)
  GROUP debit_normal (cost_of_goods_sold, operating_expense, depreciation, tax,
                      accounts_receivable, inventory, current_assets, cash)
  BY SEED finance;
```

- `targets` routes which SQL-authoring agents receive the convention. §3.4's
  `INCLUDE (conventions, …)` selects the *family* per serving policy, not
  individual conventions per consumer. No per-convention scope surface.
- `concept_groups` is machine-consumed today: the engine derives `disjoint_with`
  edges from it and resolves every member against declared concepts
  (`concept_edge_store.py:78-90`, `ontology.py:103-118`) — an OVER-style
  membership contract *inside* a convention. §1.2(4) declares convention prose
  opaque; this is exactly the part the engine does not treat as opaque. Folding
  it into prose loses the declaration-time membership check.
