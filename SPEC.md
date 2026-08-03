# glossql — language specification

Status: **working draft**, 2026-08-04. This is the simplified language; it
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
- **Functions are scripts.** The engine's analytical machinery — profiling,
  typing, detection, adjudication — lives in registered rhai scripts with
  JSON contracts; a function is either a measurement or a detector, never a
  metric. Metrics are concepts: QUERY aspects, run as their SQL (§5.1).
  Analytical logic does not live in the grammar.

## 2. Map

Every construct is backed by a transcription of a real artifact from the
running v0.3 system (`../dataraum-context`). If the system and this map
disagree, verify in code, then fix the map.

| dataraum-context artifact | construct | fixture |
|---|---|---|
| ontology concepts (`ontology.yaml`) | QUERY aspects | 01 |
| conventions (+ `targets`, `concept_groups`) | FACT aspect, in-blob | 02 |
| metrics (`dso.yaml`, `metrics` tables) | QUERY aspect, grounded in SQL | 03 |
| validations (`validations` table) | aspect + witness + ATTEST | 04 |
| cycles (`cycles.yaml`) | FACT aspects, in-blob | 05 |
| claim witnesses + reliabilities | witness slots + detector | 06 |
| groundings (`sql_snippets`) | QUERY glosses | 07 |
| teach payloads (8 types) | re-gloss on a human connection | 08 |
| answer-agent served context | dropped — reads + agent skills | 09 |
| catalog annotations, statistics, sources | FACT/MEASUREMENT glosses, SOURCE/RECIPE | 10 |
| typing + null-value config (`typing.yaml`, `null_values.yaml`, overlay teaches) | FACT aspects, whole-body re-gloss | 13 |

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
DECLARE RECIPE segments ON fin FROM crm AS $$SELECT id, segment FROM customer_segments$$;
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
DECLARE ASPECT unit WITH $${
  "type": "object",
  "properties": {"value": {"type": "string"}, "source_column": {"type": "string"}}
}$$ AS FACT;

DECLARE ASPECT revenue WITH $${
  "title": "revenue",
  "description": "Income from sales or services",
  "x-kind": "measure",
  "x-indicators": ["revenue", "sales", "income", "turnover", "receipts"]
}$$ AS QUERY;

DECLARE ASPECT min_max WITH $${
  "type": "object",
  "properties": {"min": {}, "max": {}}
}$$ AS MEASUREMENT;
```

The kind fixes the aspect's role:

- **FACT** — an authored JSON assertion (units are USD, `created_at` is a
  timestamp, this convention holds). The `WITH` schema validates the gloss
  body. Constants and formulas are FACT aspects — "cannot be grounded" means
  simply not `AS QUERY`.
- **QUERY** — an SQL-grounded concept (revenue, accounts_receivable, dso).
  Metrics are QUERY aspects: the value materializes by running the grounding
  SQL, never through a function. Glosses validate against the **standard
  grounding schema** (§5.2), not the `WITH` schema; the `WITH` schema
  carries the ontology entry (description, indicators, unit, parameters,
  rendering).
- **MEASUREMENT** — a statistical evaluation (min_max, outliers,
  relationship_candidates). Never glossed: its value is the bound function's
  cached JSON output (§6, §7), served by `GLOSSARY()` beside facts and
  groundings, from the `cache` relation (§6).

Multiplicity lives inside the blob — array-typed schemas — never in extra
statements or slots.

Re-declaring an aspect with identical content is a no-op. Changing it while
glosses under it exist is refused — delete those rows first; existing bodies
never silently stop matching their schema.

### 5.2 Glosses

One uniform statement; every body is JSON. Bodies are dollar-quoted
(`$$ … $$`, postgres-style; `$tag$ … $tag$` if the body itself contains
`$$`), so the JSON document rides verbatim — no escaping, ever:

```sql
GLOSS unit ON orders.amount AS $${"value": "EUR", "source_column": "currency_code"}$$;

GLOSS revenue ON fin.journal_lines AS $${
  "sql": "SELECT debit_amount - credit_amount FROM journal_lines WHERE account_type = 'revenue'",
  "assumptions": [
    {"dimension": "sign", "assumption": "ledger stores debits positive",
     "basis": "column_stats", "confidence": 0.9}
  ]
}$$;

GLOSS fk_note ON orders.customer_id -> customers.id AS $${"value": "2% orphaned rows"}$$;
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
`kind` is the aspect's kind; who spoke is `actor`, under `witness`.

With no subject, `GLOSSARY()` sweeps the `USE`'d dataset. A subject serves
itself and what lies under it: a table serves its columns and every
relationship it participates in, from either side; the far endpoint's own
context is never pulled in.

`subject::aspect` narrows either read to one declared aspect, as in ATTEST
(§7.2) — a metric's declaration and grounding SQL are one narrowed read
away:

```sql
SELECT * FROM GLOSSARY(fin::dso);
```

## 6. The function library

Scripts registered as functions, with name and contract; static by nature —
ported by copying the script. A function is either a **measurement** — it
fills a MEASUREMENT aspect through that aspect's witness (§7) — or a
**detector** (§7.1). The library is the engine's analytical machinery
(profiling, typing, detection) moved into the server as rhai scripts;
metrics are not functions (§5.1).

```sql
DECLARE FUNCTION profile_min_max FOR fin FROM 'functions/profile_min_max.rhai'
  RETURNS $${
    "type": "object",
    "properties": {"min": {}, "max": {}}
  }$$;

DECLARE FUNCTION infer_types FOR GLOBAL FROM 'functions/infer_types.rhai'
  ACCEPTS (type_patterns, null_values)
  RETURNS $${
    "type": "object",
    "required": ["types"],
    "properties": {"types": {"type": "object"}}
  }$$;
```

- `FOR` scopes the function to a dataset, or `GLOBAL`.
- `FROM` names the script.
- `ACCEPTS` names the aspects whose current values the server hands the
  script as its context document — settings are context, never call
  arguments; calls are always bare `f()`. Absent `ACCEPTS`, the script
  receives no context.
- `RETURNS` is a JSON Schema; functions return JSON per it. Results land in
  the `cache` relation below.
- Every function implicitly receives its subject, with its SQL schema and
  neighborhood (parent, siblings, children) as metadata. Scripts run
  against the dataset — any SQL; determinism is the script's contract, the
  workspace its boundary.
- A detector receives the witness's slots and threshold, never table data
  (§7.1).
- A function bound to a MEASUREMENT aspect (§7) has that aspect's schema as
  its RETURNS — `GLOSSARY()` serves its output as-is.

Extraction:

```sql
SELECT profile_min_max() FROM orders;
SELECT infer_types() FROM orders;
```

The first run computes and caches; later selects read the cache. The cache
is an ordinary relation, like the glossary, named `cache`: one row per
(subject, function) — `(subject, function, body, computed_at)`. Re-running
is removal, not a modifier — DELETE at whatever
grain the WHERE clause picks, and select again:

```sql
DELETE FROM cache WHERE function = 'dso';
```

Whether multi-function
extraction fans out or runs one call after another is the caller's choice —
send one statement with many calls, or many statements; the grammar carries
no ordering surface. Functions never write the glossary; their results live
in the cache.

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
SELECT * FROM ATTEST(orders.amount::behavior);
SELECT subject, band FROM ATTEST(fin.trial_balance) WHERE band = 'red';
```

The **standard attest schema** is fixed:
`(subject, aspect, witness, band, score, computed_at)` — `band` in
`green | yellow | orange | red`, `score` the disagreement/entropy in 0..1.
Same cache semantics as function SELECT; detail lives in the value
function's own cached output, reachable by SELECT. Sweeps ("all contested
behavior columns") are WHERE clauses over the attest relation, never a
special form; with no argument, `ATTEST()` sweeps the `USE`'d dataset.
`subject::aspect` — the host's cast spelling — narrows attestation to one
declared aspect, unambiguously: `fin.trial_balance` names a table,
`fin::reconciliation` an aspect across the dataset.

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
the same guarantee · actor transport rides the connection, DuckDB-style ·
parked as a future enhancement (2026-08-03): `GLOSSARY` may someday also
materialize a QUERY aspect's value by running its grounding SQL at read —
today the read serves the SQL, and running it is the reader's act.

Deferred, not under discussion: access rights · portability · persistence
backend and engine mapping.
