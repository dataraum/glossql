# 10 · Remaining §2.1 rows — TRANSCRIBE CLEANLY (coverage completion, sprint 10)

The §2.1 rows not covered by fixtures 01–09, each against its real table shape
(engine `schema.sql`; verdicts A5–A18 in `reports/2026-07-30-adversarial-review.md`).

## Sources, tables, recipes (`sources`, `recipe_hash` — A18)

```glossql
DECLARE SOURCE erp_export FROM 'lake/erp/*.parquet' BY USER analyst;
DECLARE TABLE orders FROM erp_export BY USER analyst;
DECLARE SOURCE crm CONNECTION postgres VIA 'crm_prod' BY USER analyst;
DECLARE TABLE segments FROM crm
  AS 'SELECT id, segment FROM customer_segments' BY USER analyst;
```

`recipe_hash` needs no clause: statement identity is content hash (§1.1/§3.0
checklist item 7) — the generalization the spec promised.

## Column annotations + column concepts (`semantic_annotations`, `column_concepts` — A7/A8)

```glossql
DECLARE role(orders.amount, value := measure) BY AGENT cataloguer CONFIDENCE 0.9;
DECLARE meaning(orders.amount, value := 'gross invoiced amount per order line')
  BY AGENT cataloguer CONFIDENCE 0.92;
DECLARE behavior(orders.amount, value := flow) BY AGENT cataloguer CONFIDENCE 0.92;
DECLARE unit(orders.amount, value := 'EUR') BY AGENT cataloguer;
DECLARE unit_source(orders.amount, column := currency_code) BY AGENT cataloguer;
DECLARE stored_sign(journal_lines.amount, value := ledger_signed)
  BY AGENT cataloguer CONFIDENCE 0.8;
```

The stored-sign family (A8's omission) is just another core-pack aspect.

## Table entities (`table_entities` — A9)

```glossql
DECLARE entity(orders, value := 'sales order') BY AGENT cataloguer CONFIDENCE 0.9;
DECLARE role(orders, value := fact) BY AGENT cataloguer;
DECLARE grain(orders, columns := (order_id, line_no)) BY AGENT cataloguer;
DECLARE time_axis(orders, column := order_date, anchor := true) BY AGENT cataloguer;
DECLARE identity(orders, columns := (order_id)) BY AGENT cataloguer;
```

## Relationships + surrogate keys (`relationships`, `surrogate_key_intents` — A10/A15)

```glossql
DECLARE RELATIONSHIP orders.customer_id REFERENCES customers.id
  CARDINALITY many_to_one BY AGENT judge CONFIDENCE 0.97;
DECLARE RELATIONSHIP txn (account, business_id) REFERENCES coa (account_name, business_id)
  CARDINALITY many_to_one BY AGENT judge;
DECLARE RELATIONSHIP orders.customer_id REFERENCES customers.id REJECTED BY USER analyst;
DECLARE key(txn, columns := (account, business_id), value := confirmed) BY AGENT judge;
```

## Hierarchies + slices (`dimension_hierarchies`, `slice_definitions` — A11/A12)

```glossql
DECLARE HIERARCHY geo IN customers LEVELS (country > region > city)
  KIND drilldown BY AGENT judge;
DECLARE dimension(orders.channel, priority := 0.8,
  context := 'primary go-to-market split') BY AGENT slicer;
DECLARE dimension(orders, via := customer_id, to := customers.segment,
  priority := 0.7, interest := supporting) BY AGENT slicer;
```

`slice_relevance` → `priority`; the second axis `slice_interest` rides the same
argument surface (`interest := primary | supporting`).

## Enrichment (`enriched_views` — A13)

```glossql
DECLARE VIEW orders_enriched AS
  SELECT o.order_id, o.line_no, o.amount, c.region, c.segment
  FROM orders o JOIN customers c ON o.customer_id = c.id
  BY AGENT enricher;
```

Exposed columns = the select list; joins-used derive from the AST;
`is_grain_verified` is observation-derived (§3.2).

## Workspace calendar + vertical binding (`workspace_calendar`, `workspace_settings` — A6)

```glossql
DECLARE calendar(workspace, fiscal_year_starts := april) BY USER analyst;
```

The active-vertical binding has no construct by design: importing a pack is
replaying it (§3.1); `workspace_settings.active_vertical` is engine bookkeeping.

## Type decisions (`type_decisions` — sprint 8)

```glossql
DECLARE type(orders.amount, value := 'DECIMAL(12,2)') BY AGENT typing;
DECLARE type(orders.order_date, value := 'DATE') BY USER analyst;
```

Coverage note: with fixtures 01–09, every §2.1 row is now fixture-backed —
transcribed, reserved (bus matrix, §6), or dropped with a named replacement
(run/promotion axis, §2.6/§7).
