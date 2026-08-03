# glossql — language specification

Status: **working draft**, 2026-08-03. This is the simplified language; it
supersedes the 2026-07 draft (git history holds it; the pivot record is
`reports/2026-08-03-simplification.md`). SPEC.md is the only normative prose.
`grammar.ebnf` is the source of truth for syntax; `corpus/` holds the evidence
that every construct transcribes a real `../dataraum-context` artifact.

## 1. Overview

glossql is a declarative context language over a SQL host. It describes a
dataset — its sources, tables, relationships, meanings, checks — so that
agents and humans can work on the same data with the same context. The
language adds a small set of statements and two table functions to the host;
it does not re-specify SQL. Recipes, views, SELECT bodies, and deletes are
host SQL and stay opaque to the grammar.

Ground rules:

- **Context stays folded.** Everything that is context — even structured
  context — is a JSON document validated by a [JSON Schema](https://json-schema.org).
  Rendering conventions ride the schema. Authored prose is opaque.
- **The actor rides the connection.** Every connection carries an actor
  (agent_id or human_id), DuckDB-style. There is no BY clause anywhere; the
  engine stamps writer and actor kind on every statement.
- **The grammar fixes keys, not mechanics.** History, replay, and supersession
  mechanics are implementation. The grammar fixes what supersedes what: the
  key is (subject, aspect, actor kind).
- **Functions are scripts.** Analytical logic (metrics, checks, profiling,
  detection) lives in registered scripts with JSON contracts — addable,
  removable, ported by copying. It does not live in the grammar.

## 2. Map

Every construct is backed by a transcription of a real artifact from the
running v0.3 system (`../dataraum-context`). If the system and this map
disagree, verify in code, then fix the map.

| dataraum-context artifact | construct | fixture |
|---|---|---|
| ontology concepts (`ontology.yaml`) | QUERY aspects | 01 |
| conventions (+ `targets`, `concept_groups`) | FACT aspect, in-blob | 02 |
| metrics (`dso.yaml`, `metrics` tables) | function scripts | 03 |
| validations (`validations` table) | aspect + witness + ATTEST | 04 |
| cycles (`cycles.yaml`) | FACT aspects, in-blob | 05 |
| claim witnesses + reliabilities | witness slots + detector | 06 |
| groundings (`sql_snippets`) | QUERY glosses | 07 |
| teach payloads (8 types) | re-gloss on a human connection | 08 |
| answer-agent served context | dropped — reads + agent skills | 09 |
| catalog annotations, statistics, sources | FACT/MEASUREMENT glosses, SOURCE/RECIPE | 10 |

## 3. Sources and datasets

A **source** names where data comes from. A **recipe** materializes a table
from a source. A **dataset** is the working unit: one dataset per workspace
(the binding lives in the app, not the grammar).

```sql
DECLARE SOURCE erp_export SET (type: parquet, location: 'lake/erp/*.parquet');
DECLARE SOURCE crm SET (type: relational_db, location: 'postgres://crm.internal/prod', via: crm_prod);
```

- `type`: `relational_db | parquet | csv | json`.
- `location`: url or path — never credentials.
- `via`: a reference to engine-held secrets. Secrets never appear in
  statements, so they never enter the log.

```sql
DECLARE RECIPE segments ON fin FROM crm AS SELECT id, segment FROM customer_segments;
```

The recipe SQL runs **at the source, in the source's dialect**; the result
lands as table `segments` in dataset `fin`. Statement identity is content
hash (implementation) — re-declaring an unchanged recipe is a no-op.

```sql
DECLARE DATASET fin SET (purpose: 'working-capital analysis over ERP and CRM exports');
USE fin;
```

`USE` sets the resolution context: unprefixed `table.column` paths resolve
against the USE'd dataset; the full `dataset.table.column` prefix is always
allowed.

Derived tables are plain SQL — enrichment, cleaning, dedup are dataset→dataset:

```sql
CREATE VIEW orders_enriched AS
  SELECT o.order_id, o.amount, c.region
  FROM orders o JOIN customers c ON o.customer_id = c.id;
```

Views are glossable like tables.

## 4. Subjects and relationships

A **subject** is what a gloss, a function SELECT, or a witness attaches to:

- `dataset`
- `dataset.table` — views count as tables
- `dataset.table.column`
- `table.column -> table.column` — a declared relationship, addressed by its
  pair path (relationships have no names)

```sql
DECLARE RELATIONSHIP orders.customer_id -> customers.id;
DECLARE RELATIONSHIP invoices.order_id <-> orders.id;
```

- `->` is many-to-one (the FK direction); one-to-many is `->` written from
  the other side. `<->` is one-to-one. Many-to-many decomposes via a junction
  table.
- Relationships are **detected → verified → declared**: a function proposes
  candidates (a MEASUREMENT aspect, §5.1), an agent or human declares. Only
  declared relationships exist; there is no rejected or negative form — a
  rejected candidate is simply not declared, and detection functions are
  deterministic, so it does not resurface as new knowledge.
- Composite keys are detected by a function, materialized as a column (e.g.
  in a view), then declared as a single-column relationship.

## 5. The glossary

The glossary is the context store. An **aspect** is a declared vocabulary
entry — a name with a JSON Schema and a kind. A **gloss** applies an aspect
to a subject with a JSON body. There are no fact names: the aspect is the key.

### 5.1 Aspects

```sql
DECLARE ASPECT unit WITH {
  "type": "object",
  "properties": {"value": {"type": "string"}, "source_column": {"type": "string"}}
} AS FACT;

DECLARE ASPECT revenue WITH {
  "title": "revenue",
  "description": "Income from sales or services",
  "x-kind": "measure",
  "x-indicators": ["revenue", "sales", "income", "turnover", "receipts"]
} AS QUERY;

DECLARE ASPECT min_max WITH {
  "type": "object",
  "properties": {"min": {}, "max": {}}
} AS MEASUREMENT;
```

The kind fixes the aspect's role:

- **FACT** — an authored JSON assertion (units are USD, `created_at` is a
  timestamp, this convention holds). The `WITH` schema validates the gloss
  body. Constants and formulas are FACT aspects — "cannot be grounded" means
  simply not `AS QUERY`.
- **QUERY** — an SQL-grounded concept (revenue, accounts_receivable). Its
  glosses validate against the **standard grounding schema** (§5.2), not the
  `WITH` schema; the `WITH` schema carries the ontology entry (description,
  indicators, unit, rendering).
- **MEASUREMENT** — a statistical evaluation (min_max, outliers,
  relationship_candidates). Never glossed: its value is the bound function's
  cached JSON output (§6, §7), served by `GLOSSARY()` beside facts and
  groundings. How it is cached is implementation.

Multiplicity lives inside the blob — array-typed schemas — never in extra
statements or slots.

### 5.2 Glosses

One uniform statement; every body is JSON:

```sql
GLOSS unit ON orders.amount AS {"value": "EUR", "source_column": "currency_code"};

GLOSS revenue ON fin.journal_lines AS {
  "sql": "SELECT debit_amount - credit_amount FROM journal_lines WHERE account_type = 'revenue'",
  "assumptions": [
    {"dimension": "sign", "assumption": "ledger stores debits positive",
     "basis": "column_stats", "confidence": 0.9}
  ]
};

GLOSS fk_note ON orders.customer_id -> customers.id AS {"value": "2% orphaned rows"};
```

- You cannot gloss an aspect that was not declared. Admission validates the
  body by the aspect's kind: FACT → the aspect's `WITH` schema, QUERY → the
  standard grounding schema, MEASUREMENT → rejected.
- The **standard grounding schema** is fixed, like the attest schema (§7.2):

```json
{
  "type": "object",
  "required": ["sql"],
  "additionalProperties": false,
  "properties": {
    "sql": {"type": "string"},
    "assumptions": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["assumption"],
        "properties": {
          "dimension": {"type": "string"},
          "assumption": {"type": "string"},
          "basis": {"type": "string"},
          "confidence": {"type": "number", "minimum": 0, "maximum": 1}
        }
      }
    }
  }
}
```
- **Supersession key: (subject, aspect, actor kind).** A human re-gloss
  supersedes the human's value; an agent's supersedes the agent's. The slots
  stay separate; a witness adjudicates across them (§7).
- Two QUERY glosses of the same aspect on different tables may coexist — two
  ways to calculate revenue arriving at the same number is a correct state.
  Whether they reconcile is a witness's job (a detector runs both and returns
  band + score).
- The glossary is an ordinary queryable relation; removal is SQL:

```sql
DELETE FROM glossary WHERE subject = 'orders.amount' AND aspect = 'unit';
```

### 5.3 Reading

One table function, plain SQL:

```sql
SELECT * FROM GLOSSARY(orders.amount);
```

The default, collapsed read: one row per (subject, aspect) —
`(subject, aspect, value, band, score)`. The value is collapsed by the
aspect's witness detector; it is NULL when the slot is empty or entropy is
above the witness's threshold. An agent reading NULL knows the context is
absent or contested — judgment stays in the read policy, never in fabricated
values.

```sql
SELECT * FROM GLOSSARY(orders.amount, all => true);
```

The raw read: one row per (subject, aspect, kind, witness) —
`(subject, aspect, kind, witness, actor, body, written_at)` — all current
values side by side; precedence between them is the reader's business.

## 6. The function library

Scripts registered as functions, with name and contract; static by nature —
ported by copying the script.

```sql
DECLARE FUNCTION dso FOR fin FROM 'functions/dso.py'
  ACCEPTS {
    "type": "object",
    "properties": {"days_in_period": {"type": "integer", "default": 30, "enum": [30, 90, 365]}}
  }
  RETURNS {
    "type": "object",
    "required": ["value"],
    "properties": {"value": {"type": "number"}, "unit": {"const": "days"}}
  };
```

- `FOR` scopes the function to a dataset, or `GLOBAL`.
- `FROM` names the script.
- `ACCEPTS` is the input contract: a JSON Schema, or a pointer to a single
  value inside another producer's schema — `ACCEPTS period_grain#/properties/days`.
  Arguments are passed by name.
- `RETURNS` is a JSON Schema; functions return JSON per it. How results are
  cached is implementation.
- Every function implicitly receives its subject and the subject's SQL schema.
- A function bound to a MEASUREMENT aspect (§7) has that aspect's schema as
  its RETURNS — `GLOSSARY()` serves its output as-is.

Extraction:

```sql
SELECT dso(days_in_period => 90) FROM fin;
SELECT profile_min_max() FROM orders PARALLEL REFRESH;
```

The first run computes and caches; later selects read the cache; `REFRESH`
re-runs. `SEQUENTIAL | PARALLEL` orders multi-function extraction. Functions
never write the glossary; their results live in the cache.

## 7. Witnesses

A witness is declared per aspect, dataset-wide. Per (subject, aspect) it
holds one slot per speaker: the measurement's reading (served from the value
function's cache), the agent's gloss, the human's gloss — one current value
each.

### 7.1 Declaration

```sql
DECLARE WITNESS behavior_w ON behavior
  BY (FUNCTION temporal_behavior, AGENT, HUMAN)
  DETECTOR behavior_entropy
  THRESHOLD 0.7;

DECLARE WITNESS min_max_w ON min_max BY (FUNCTION profile_min_max);
```

- `BY` lists who may speak to the aspect. A MEASUREMENT aspect is
  `BY (FUNCTION fn)` only — the witness is its function binding. FACT and
  QUERY aspects may admit all three.
- `DETECTOR` names the function that examines the slots and returns band +
  score. A function is eligible as detector only if its RETURNS conforms to
  the standard attest schema. `DETECTOR` and `THRESHOLD` are optional — a
  pure measurement with nothing to adjudicate needs neither.
- `THRESHOLD` (0..1) is the entropy cutoff used by the collapsed
  `GLOSSARY()` read (§5.3).

### 7.2 Attestation

```sql
SELECT * FROM ATTEST(orders.amount.behavior);
SELECT subject, band FROM ATTEST(fin.trial_balance) WHERE band = 'red';
```

The **standard attest schema** is fixed:
`(subject, aspect, witness, band, score, computed_at)` — `band` in
`green | yellow | orange | red`, `score` the disagreement/entropy in 0..1.
Same cache/REFRESH semantics as function SELECT; detail lives in the value
function's own cached output, reachable by SELECT. Sweeps ("all contested
behavior columns") are WHERE clauses over the attest relation, never a
special form.

Judgment lives here — in detector functions and read policy — never in
results: no construct writes a verdict into data.

## 8. Skills

Agents use the language through skills; agents are not part of the grammar.
The skill set mirrors the statement set: create a source, create a dataset,
create a function, declare an aspect, write a gloss (body kind follows the
aspect's declared kind), declare a witness, read the glossary. The flow docs
(`corpus/11-flow-add-source.md`, `corpus/12-flow-begin-session.md`) model the
running system's two operational flows as statement sequences.

## 9. Open

One open question, raised by fixture 09's disclosure benchmark: the collapsed
`GLOSSARY()` read (§5.3). NULL may be too simple — it conflates three states
a reading agent must distinguish: **never assessed** (no witness ran),
**contested** (entropy above threshold), **gated** (awaiting human
confirmation). Related: whether the read enumerates the declared-aspect grid
(so "never assessed" is a visible row) or serves only existing rows (absence
reads as nonexistence). Closes by corpus test against the real served context
(fixture 09), not by argument.

PoC notes: batch visibility comes from (long-running) transactions — the
running system's run_id + snapshot-head pointer is the verbose version of
the same guarantee · actor transport rides the connection, DuckDB-style.

Deferred, not under discussion: access rights · portability · persistence
backend and engine mapping.
