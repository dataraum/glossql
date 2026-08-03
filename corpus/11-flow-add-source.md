# 11 · Flow: add source — modelled as a statement sequence

Source: the running system's add-source pipeline (verified 2026-08-03).
Cockpit acquisition: `packages/cockpit/src/server/import-sources.ts`
(`persistImportSet`, collision guard), `routes/api/upload.ts`. Engine spine:
`packages/engine/src/dataraum/worker/workflows.py:211` (`AddSourceWorkflow`),
phases in `pipeline/phases/`. Ordered steps and what each produces:

| step | kind | produces (today) |
|---|---|---|
| stage upload / probe query | human | staged bytes / recipe spec |
| frame vertical | LLM | concepts, conventions, validations, cycles, metrics rows |
| register sources | deterministic | `sources` rows |
| import | deterministic | raw all-VARCHAR tables + table/column rows |
| typing | deterministic | `type_candidates`, `type_decisions`, typed + quarantine tables |
| statistics / eligibility / quality / temporal | deterministic | `statistical_profiles`, `column_eligibility`, `statistical_quality_metrics`, `temporal_column_profiles` |
| semantic_per_column | LLM | `semantic_annotations` per column |
| detect | deterministic | `entropy_objects`, `claim_witnesses`, `entropy_readiness` |
| promote_to_latest | deterministic | snapshot head flip |
| assess & auto-ground loop | LLM + orchestration | teach rows or `awaiting_input` park |

## Transcription

The three actor kinds of the pipeline map onto the three ways of speaking:
deterministic phases are **functions** (MEASUREMENT aspects), LLM phases are
**agent glosses**, teaches and parks are **human glosses**.

Human registers the source; recipes land the tables:

```glossql
USE fin;
DECLARE SOURCE erp_export SET (type: parquet, location: 'lake/erp/*.parquet');
DECLARE RECIPE orders ON fin FROM erp_export AS SELECT * FROM read_parquet('orders/*.parquet');
```

Framing the vertical is replaying the vertical folder's declarations —
aspects, check functions, witnesses (fixtures 01, 02, 04); no construct.

The deterministic profile plane — declared once (vertical/global), fanned out
per table:

```glossql
DECLARE ASPECT column_profile WITH {
  "type": "object",
  "properties": {"null_ratio": {}, "distinct": {}, "min": {}, "max": {},
                 "top_values": {"type": "array"}}
} AS MEASUREMENT;
DECLARE FUNCTION profile FOR GLOBAL FROM 'functions/profile.py'
  RETURNS {"type": "object",
    "properties": {"null_ratio": {}, "distinct": {}, "min": {}, "max": {},
                   "top_values": {"type": "array"}}};
DECLARE WITNESS column_profile_w ON column_profile BY (FUNCTION profile);

DECLARE ASPECT type_candidates WITH {
  "type": "object",
  "properties": {"candidates": {"type": "array",
    "items": {"type": "object",
      "properties": {"type": {"type": "string"}, "confidence": {"type": "number"}}}}}
} AS MEASUREMENT;
DECLARE FUNCTION infer_types FOR GLOBAL FROM 'functions/infer_types.py'
  RETURNS {"type": "object", "properties": {"candidates": {"type": "array"}}};
DECLARE WITNESS type_candidates_w ON type_candidates BY (FUNCTION infer_types);

SELECT profile(), infer_types() FROM fin.orders PARALLEL;
```

Typing decisions and semantic annotation are agent glosses (an agent
connection, reading the measurements first); the typed table is a view:

```glossql
SELECT * FROM GLOSSARY(fin.orders.amount);

GLOSS type ON orders.amount AS {"value": "DECIMAL(12,2)"};
GLOSS meaning ON orders.amount AS {"value": "gross invoiced amount per order line"};
GLOSS behavior ON orders.amount AS {"value": "flow"};
GLOSS unit ON orders.amount AS {"value": "EUR", "source_column": "currency_code"};

CREATE VIEW orders_typed AS
  SELECT CAST(order_id AS BIGINT) AS order_id,
         CAST(amount AS DECIMAL(12,2)) AS amount,
         TRY_CAST(order_date AS DATE) AS order_date
  FROM orders;
```

Adjudication replaces the detect/resolve/readiness tail: witnesses on the
contested aspects, bands read back; the auto-ground loop is an agent skill
sweeping the attest relation and re-glossing where it may:

```glossql
SELECT * FROM ATTEST(fin.behavior);
SELECT subject, band, score FROM ATTEST(fin.type_agreement) WHERE band = 'red';
```

A human closes what the agent could not — the same statements on a human
connection supersede the human slot (fixture 08); nothing parks in a queue
that the grammar knows about.

## Findings

- **The flow transcribes with no flow construct.** Sequencing, retries,
  budgets, the replay-or-surface loop, and the column-limit gate are
  orchestration — app concern; `SEQUENTIAL | PARALLEL` on extraction is the
  only ordering surface the grammar carries.
- Quarantine tables and cast-failure routing are the typed view's business
  (script/SQL), not statements.
- `run_id` versioning and the promote/head flip are the cache and
  supersession mechanics — implementation by ground rule; `REFRESH` is the
  only surface.
- The vertical binding (`workspace_settings.active_vertical`) is replaying a
  folder (fixture 01) — confirmed against the real frame step, which writes
  seed rows exactly like a replay would.
