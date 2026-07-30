# 02 · Convention `sign_natural_balance` — concept_groups RESOLVED (sprint 9); targets deferred

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

## Transcription — decided form (sprint 9): groups are concepts

```glossql
DECLARE CONCEPT credit_normal KIND group BY SEED finance;
DECLARE RELATIONSHIP revenue PART OF credit_normal BY SEED finance;
DECLARE RELATIONSHIP accounts_payable PART OF credit_normal BY SEED finance;
DECLARE RELATIONSHIP equity PART OF credit_normal BY SEED finance;
DECLARE CONCEPT debit_normal KIND group BY SEED finance;
DECLARE RELATIONSHIP accounts_receivable PART OF debit_normal BY SEED finance;
DECLARE RELATIONSHIP cash PART OF debit_normal BY SEED finance;

DECLARE CONVENTION sign_natural_balance
  STATEMENT 'Express every monetary measure in its natural-balance direction:
    credit_normal concepts as credits, debit_normal as debits — never
    normalize only one side of a comparison.'
  BY SEED finance;
```

## Findings

- **RESOLVED (sprint 9) — `concept_groups`.** Groups are declared concepts
  (`KIND group`) with `PART OF` members: the machine-checked half lives in
  concept space with ordinary supersession and the same declaration-time
  membership guarantee the engine's lint provides today
  (`concept_edge_store.py:78-90`, `ontology.py:103-118`); the prose refers to
  groups by name and stays opaque — §1.2(4) holds.
- **DEFERRED — `targets`** (`[extraction, qa]` consumer routing). Per-convention
  serving scope; §3.4's `INCLUDE` selects the conventions *family* only. Lands
  with the serving clause list (§3.4 flagged, fixture 09):

```glossql-gap
DECLARE CONVENTION sign_natural_balance
  STATEMENT '…'
  TARGETS (extraction, qa)
  BY SEED finance;
```
