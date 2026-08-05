---
name: glossql-relationships
description: Ground the join structure of a glossql dataset — run the shipped detect_relationships measurement, judge every candidate against the data, and declare only the survivors. Use after tables have landed, before views or cross-table analysis.
---

# Declaring relationships

The arc is candidate → verified → declared. A high-recall measurement
proposes; you judge; the grammar records. The workspace ships
`detect_relationships` at boot.

## 1. Measure

```glossql
USE fin;
SELECT detect_relationships() FROM fin;
SELECT value FROM GLOSSARY(fin::relationship_candidates) WHERE state = 'current';
```

It runs at dataset grain over every landed table: columns that look
like keys (near-unique) become `to` sides, every type-compatible
column is tried as a `from` side, and any pair where at least half the
from-side values resolve survives. Each candidate carries `from`,
`to`, `cardinality`, `overlap`, plus evidence — `matched`, `orphans`,
`from_distinct`, `to_distinct`. The list is deliberately generous:
high recall, false positives included, you are the precision. After
new tables land, re-measure:
`DELETE FROM cache WHERE function = 'detect_relationships';`

## 2. Judge every candidate

Before declaring anything, per candidate:

- **Anti-join both directions.** Count and *read* what doesn't
  resolve:
  ```glossql
  SELECT count(*) FROM orders o LEFT JOIN customers c
    ON o.customer_id = c.id WHERE c.id IS NULL;
  ```
- **Ground the orphans.** An orphan count is a question, not a
  verdict. Orphans that are exactly a business population (the
  cancelled invoices, the pre-migration accounts) confirm the edge —
  declare it and gloss the finding. Random misses argue against it.
- **Distrust coincidence.** Two unique integer columns overlap
  perfectly without meaning it — parallel row-number sequences are the
  classic false positive. A join must mean something: the names, the
  values, and the business objects have to agree.
- **Check the claimed cardinality** on the data
  (`GROUP BY … HAVING count(*) > 1`) rather than trusting the label.

## 3. Declare the survivors

```glossql
DECLARE RELATIONSHIP orders.customer_id -> customers.id;
DECLARE RELATIONSHIP invoices.order_id <-> orders.id;
```

`->` is a reference; `<->` when both sides resolve each other. A
same-table candidate (`coa.parent_code -> coa.account_code`) is a
hierarchy — declare it like any edge. A composite key is cured first,
then declared (the decided rule):

```glossql
CREATE VIEW txn_keyed AS
  SELECT *, account || ':' || business_id AS account_key FROM txn;
DECLARE RELATIONSHIP txn_keyed.account_key -> coa.account_key;
```

Rejected candidates are *not declared and not erased* — they stay
visible in the measurement, which is the record that they were seen
and judged.

## 4. Record the grounds

Declared edges accept glosses on the pair path. If the workspace has a
prose aspect (`meaning`), say why the edge holds and what the orphans
are:

```glossql
GLOSS meaning ON orders.customer_id -> customers.id AS
  $${"value": "each order belongs to one customer; 140 orphans are the cancelled orders, never posted"}$$;
```

## 5. Read back

```glossql
SELECT * FROM relationships;
SELECT subject, aspect, value FROM GLOSSARY(orders);
```

The `relationships` relation is the declared structure; a table's
`GLOSSARY()` sweep picks up the pair paths it participates in.
