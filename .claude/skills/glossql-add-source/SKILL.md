---
name: glossql-add-source
description: Drive the add-source flow in a glossql workspace end to end — probe a declared source, author the typing recipe, land the table, run the measurement plane, frame the semantic vocabulary, and gloss every column. Use when connecting a new data source or landing a new table.
---

# Adding a source

The statement shapes below assume the `glossql` skill (the door, the
outcome shape). A fresh workspace already holds the measurement
library — `profile`, `outliers`, `temporal`, `slot_entropy` and their
aspects arrive at boot; `SELECT * FROM functions` lists what is
declared.

## 1. Dataset and source

```glossql
USE fin;
DECLARE SOURCE erp_export SET (type: parquet, location: 'lake/erp');
```

The location is a root directory. Globs and file paths belong in
recipe SQL, resolving under that root.

## 2. Probe — look before you write

`PROBE` runs recipe-shaped SQL at the source and lands nothing. Use it
to count what parses, then to rehearse the exact schema:

```glossql
PROBE erp_export AS $$SELECT count(raw) AS filled, count(parsed) AS parsed
FROM (SELECT "amount" AS raw, try_cast("amount" AS DOUBLE) AS parsed
      FROM read_parquet('orders/*.parquet'))$$;

PROBE erp_export AS $$SELECT order_id,
       try_cast(amount AS DOUBLE) AS amount,
       try_to_date(order_date, '%d.%m.%Y') AS order_date
FROM read_parquet('orders/*.parquet') LIMIT 0$$;
```

Alias the casts in a subquery before aggregating over both the raw and
the parsed column: the engine names a cast after its inner expression,
so `count("amount")` and `count(try_cast("amount" AS DOUBLE))` collide
in one aggregate — the `AS` aliases arrive too late to separate them.

A `LIMIT 0` probe's empty result still carries its schema — it
rehearses exactly the identity the recipe will stamp. Taught format
patterns (date spellings, decimal marks) are FACT glosses on the
dataset — read them from the glossary before guessing formats.

## 3. Recipe — typing is authored

The recipe carries the casts and the column choices; there is no typing
machinery behind it. A value that fails its cast lands as NULL (a kept
row with a NULL cell, not a dropped row); a column you leave out of the
SELECT list is your judgment as author.

```glossql
DECLARE RECIPE orders ON fin FROM erp_export AS $$
  SELECT order_id,
         try_cast(amount AS DOUBLE) AS amount,
         try_to_date(order_date, '%d.%m.%Y') AS order_date
  FROM read_parquet('orders/*.parquet')$$;
```

The outcome carries the counts at the decision moment; history stays in
`SELECT * FROM imports`. The landed table is the typed table. Rules
that will refuse you: a changed recipe is a different table (new name),
and `DROP TABLE` is refused while the table holds data.

## 4. The measurement plane

Fan out the library per column — the grain is yours, the grammar
carries no ordering:

```glossql
SELECT profile() FROM orders.amount;
SELECT outliers() FROM orders.amount;
SELECT temporal() FROM orders.order_date;
```

Order matters only through `ACCEPTS`: `outliers` reads the cached
profile. If a result abstains with
`{"applicable": false, "missing_aspects": ["column_profile"]}`, run the
function that RETURNS the named aspect first — the abstention heals on
its own once the dependency lands. A bare `{"applicable": false}` means
the subject genuinely doesn't fit (a text column has no outliers); stop
trying.

## 5. Frame the semantic vocabulary

The workspace ships with measurements only. Declare the vocabulary
before glossing — send once, verbatim. The `ON` list is each aspect's
grain: glosses outside it are refused, and the `unassessed` grid stays
within it.

```glossql
DECLARE ASPECT meaning WITH $${
  "type": "object", "required": ["value"],
  "properties": {"value": {"type": "string"}, "term": {"type": "string"}}
}$$ AS FACT ON TABLE, COLUMN, RELATIONSHIP;
DECLARE ASPECT entity WITH $${
  "type": "object", "required": ["value"],
  "properties": {"value": {"type": "string"},
                 "role": {"enum": ["fact", "dimension"]},
                 "grain": {"type": "array", "items": {"type": "string"}},
                 "time_axis": {"type": "string"},
                 "identity_columns": {"type": "array", "items": {"type": "string"}}}
}$$ AS FACT ON TABLE;
DECLARE ASPECT role WITH $${
  "type": "object", "required": ["value"],
  "properties": {"value": {"enum": ["key", "measure", "dimension",
                                    "timestamp", "attribute"]}}
}$$ AS FACT ON COLUMN;
DECLARE ASPECT behavior WITH $${
  "type": "object", "required": ["value"],
  "properties": {"value": {"enum": ["stock", "flow"]}}
}$$ AS FACT ON COLUMN;
DECLARE ASPECT unit WITH $${
  "type": "object", "required": ["value"],
  "properties": {"value": {"type": "string"},
                 "source_column": {"type": "string"}}
}$$ AS FACT ON COLUMN;

DECLARE WITNESS meaning_w ON meaning BY (AGENT, HUMAN);
DECLARE WITNESS entity_w ON entity BY (AGENT, HUMAN);
DECLARE WITNESS role_w ON role BY (AGENT, HUMAN)
  DETECTOR slot_entropy THRESHOLD 0.7;
DECLARE WITNESS behavior_w ON behavior BY (AGENT, HUMAN)
  DETECTOR slot_entropy THRESHOLD 0.7;
DECLARE WITNESS unit_w ON unit BY (AGENT, HUMAN)
  DETECTOR slot_entropy THRESHOLD 0.7;
```

## 6. Gloss every table — the entity verdict

Before the columns, say what each table *is*. Every correct aggregate
downstream depends on this verdict, and it is judged from the data,
never from the table's name:

- **value** — what one row is, in business words ("one journal line",
  "a customer master record").
- **role** — `fact` (events/measures at volume, carrying the numbers)
  or `dimension` (descriptive, referenced by others). Read it from the
  evidence: measures, an event date, row counts, who references whom.
- **grain** — the columns that identify one row. Verify, never assert:
  `COUNT(*)` vs `COUNT(DISTINCT (col, …))` must agree. A table whose
  real grain is composite gets the composite; a table with no key gets
  none — say so in `meaning` rather than inventing one. Watch for
  document-header values repeated onto every line (constant within the
  document id): summing them at row grain multiplies by line count.
- **time_axis** — the column recording *when the row's event
  happened*. Attribute dates (due_date, hire_date) are not an axis;
  one anchor at most; a table with only attribute dates has none.
- **identity_columns** — structural observation only: which columns
  identify entities (theirs or another table's).

```glossql
GLOSS entity ON orders AS $${"value": "sales order line", "role": "fact",
  "grain": ["order_id", "line_no"], "time_axis": "order_date"}$$;
```

## 7. Gloss every column

This is the content the flow exists to produce. Read the measurements
first (`SELECT * FROM GLOSSARY(orders.amount)` serves the profile),
then speak to each aspect on every landed column:

- **meaning** — `value` is one sentence, specific to the business
  context, saying what the column contains and how it is used; `term`
  is the human-readable name a report would print (`txn_amt` →
  "Transaction Amount"). Never state stock-or-flow or summability in
  the prose — that verdict has one home, `behavior`.
- **role** — the structural role, judged from this table alone:
  `key` = primary identifier (unique, non-null) · `measure` = numeric
  value meant for aggregation · `dimension` = categorical value for
  grouping and filtering · `timestamp` = date or datetime ·
  `attribute` = descriptive, neither aggregated nor grouped on. Never
  call a column a foreign key here — references are
  `DECLARE RELATIONSHIP`, decided against the other table.
- **behavior** — numeric measures only. `stock` is a carried
  point-in-time level (balance, position, headcount) that must not be
  summed across periods; `flow` is a per-period movement (payment,
  sale, change) that accumulates and is summable. A column's own
  trajectory cannot decide this — a trending flow and a mean-reverting
  stock look alike — so read the evidence before glossing:
  `SELECT behavior_evidence() FROM orders.amount;` reconciles the
  column against period movements aggregated from event tables
  reachable over *declared* relationships (declare edges first; a new
  edge does not invalidate this cache —
  `DELETE FROM cache WHERE function = 'behavior_evidence';`
  recomputes). Each anchor carries a verdict beside its evidence —
  entity votes, agreement, both residuals, the runner-up
  conventions — and `abstain` is a complete answer, not a defect. The
  verdict is evidence for *your* judgment, never a ruling: you may
  out-judge it by testing against the ledger yourself. Names lie
  either way — a "trial balance" column can carry period turnover (a
  flow) rather than balances; the measurement reads the data, not the
  label. Unsure? Don't gloss: absence shows as an honest `unassessed`
  row; a guess does not.
- **unit** — where a magnitude has one: currency, quantity unit,
  percentage. `source_column` names the column carrying the unit when
  it rides beside the value.

```glossql
GLOSS meaning ON orders.amount AS $${"value": "gross invoiced amount per order line", "term": "Order Amount"}$$;
GLOSS role ON orders.amount AS $${"value": "measure"}$$;
GLOSS behavior ON orders.amount AS $${"value": "flow"}$$;
GLOSS unit ON orders.amount AS $${"value": "EUR", "source_column": "currency_code"}$$;
```

## 8. Read back what's open

```glossql
SELECT count(*) FROM GLOSSARY(fin) WHERE state = 'unassessed';
SELECT subject, band, score FROM ATTEST(fin::behavior) WHERE band = 'red';
```

Witnessed aspects nobody spoke to appear as rows — absence is visible,
not an omission. Red bands are where a human must close what you could
not.
