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

The actor kinds of the pipeline map onto the ways of speaking:
deterministic phases are **functions** (MEASUREMENT aspects), LLM phases
are **agent glosses**, teaches and parks are **human glosses**. The typing
phase maps to none of them — it becomes **authorship**, the
probe-and-recipe conversation below.

Human registers the source; the agent probes it through the same statement
door — recipe-shaped SQL, executed at the source, landing nothing (v0.3's
"probe query" step, returned to its place):

```glossql
USE fin;
DECLARE SOURCE erp_export SET (type: parquet, location: 'lake/erp');

SELECT * FROM read_parquet('erp_export/orders/*.parquet') LIMIT 50;
SELECT count("order_date") AS filled,
       count(try_to_date("order_date", '%d.%m.%Y')) AS parsed
FROM read_parquet('erp_export/orders/*.parquet');
```

Typing is authored, not decided (ruled 2026-08-04): the recipe carries the
casts. The agent writes it from the probes and the taught patterns
(fixture 13 — still FACT glosses, now read by the author instead of
consumed by machinery); the human approves. The default is `SELECT *`;
the landed table is the typed table, snapshotted by Iceberg on every
import:

```glossql
DECLARE RECIPE orders ON fin FROM erp_export AS $$
  SELECT order_id,
         try_cast(amount AS DECIMAL(12,2)) AS amount,
         try_to_date(order_date, '%d.%m.%Y') AS order_date
  FROM read_parquet('orders/*.parquet')$$;

SELECT sum(amount) FROM orders;
```

The table is its recipe's result — identity is content, the hash of the
SQL and the schema it produces (the v0.3 engine already keys recipes this
way). A data update re-runs the same recipe and appends a snapshot; it
must reproduce the schema or it errors. Correcting a wrong recipe is
removal first:

```glossql
DROP TABLE orders;
```

— refused while the table holds data (PoC rule); a wrong recipe gets a
new name instead, because a different SQL is a different table. The
deletion cascade is future work. Rows the recipe filtered away are the
author's to judge, on the files, outside the box; the engine keeps one
number:

```glossql
SELECT dropped_rows_count FROM imports WHERE table_name = 'orders';
```

Framing the vertical is replaying the vertical folder's declarations —
aspects, check functions, witnesses (fixtures 01, 02, 04); no construct.

The deterministic profile plane — declared once (vertical/global), fanned
out per column (extraction grain is the subject; the fan-out is the
caller's loop, the grammar carries no ordering). The quality plane chains
on it through `ACCEPTS`: the outlier fences reuse the profile's quartiles
and MAD, and a re-profile kills the outlier cache. An all-null column
needs no machinery — the author leaves it out of the recipe, or keeps it,
deliberately:

```glossql
DECLARE ASPECT column_profile WITH $${
  "type": "object",
  "properties": {"null_ratio": {}, "distinct": {}, "min": {}, "max": {},
                 "top_values": {"type": "array"}}
}$$ AS MEASUREMENT;
DECLARE FUNCTION profile FOR GLOBAL FROM 'functions/profile.rhai'
  RETURNS $${"type": "object",
    "properties": {"null_ratio": {}, "distinct": {}, "min": {}, "max": {},
                   "top_values": {"type": "array"}}}$$;
DECLARE WITNESS column_profile_w ON column_profile BY (FUNCTION profile);

DECLARE ASPECT outlier_profile WITH $${
  "type": "object", "required": ["applicable"],
  "properties": {"applicable": {"type": "boolean"},
                 "iqr": {"type": "object"}, "zscore": {"type": "object"}}
}$$ AS MEASUREMENT;
DECLARE FUNCTION outliers FOR GLOBAL FROM 'functions/outliers.rhai'
  ACCEPTS (column_profile)
  RETURNS $${"type": "object", "required": ["applicable"]}$$;
DECLARE WITNESS outlier_profile_w ON outlier_profile BY (FUNCTION outliers);

SELECT profile(), outliers() FROM fin.orders.amount;
```

Semantic annotation stays agent glosses (an agent connection, reading the
measurements first). A typing correction is a recipe correction — the
same SQL hands that wrote it — never a gloss:

```glossql
SELECT * FROM GLOSSARY(fin.orders.amount);

GLOSS meaning ON orders.amount AS $${"value": "gross invoiced amount per order line"}$$;
GLOSS behavior ON orders.amount AS $${"value": "flow"}$$;
GLOSS unit ON orders.amount AS $${"value": "EUR", "source_column": "currency_code"}$$;
```

Adjudication replaces the detect/resolve/readiness tail: witnesses on the
contested aspects, bands read back; the auto-ground loop is an agent skill
sweeping the attest relation and re-glossing where it may:

```glossql
SELECT * FROM ATTEST(fin::behavior);
SELECT subject, band, score FROM ATTEST(fin::unit) WHERE band = 'red';
```

A human closes what the agent could not — the same statements on a human
connection supersede the human slot (fixture 08); nothing parks in a queue
that the grammar knows about.

## Findings

- **Location is a root, not a glob** (respelled 2026-08-04, with the M3
  build-out): the original transcription had `location:
  'lake/erp/*.parquet'` while the recipe read `'orders/*.parquet'` — two
  globs that cannot compose. The source names the root directory; the globs
  belong to recipe SQL, resolving under it.
- **The flow transcribes with no flow construct.** Sequencing, retries,
  budgets, the replay-or-surface loop, and the column-limit gate are
  orchestration — app concern. The grammar carries no ordering surface at
  all (`SEQUENTIAL | PARALLEL` was dropped 2026-08-03): the caller either
  sends one extraction with many calls or several statements in sequence.
- **Typing is authored in the recipe** (ruled 2026-08-04 — the third
  respell of this finding, and the arc is the record): the original
  transcription hand-wrote `CREATE VIEW orders_typed` with strict CASTs;
  the M4 build derived the typed view from `type` glosses, with
  `orders_raw` and `orders_quarantined` beside it. Both put typing in
  machinery. The ruling puts it in authorship: the recipe carries the
  casts, written by the agent from probes and patterns, approved by the
  human, and the landed table is the typed table — served types are
  catalog fact, not judgment. `type`, `type_candidates`, and `eligible`
  leave the engine's vocabulary; the derived pair, the raw twin, and
  reactive view invalidation leave the engine.
- **Eligibility dissolved into authorship** (ruled 2026-08-04, hours after
  the projection gate landed): column selection is the recipe's SELECT
  list. The v0.3 findings stand — the phase's `ALTER`-drop was
  irreversible with no override, and its `WARN` tier was read by nobody —
  but the corrected answer is a line the author writes, not a gate the
  engine owns.
- **Table lifecycle is content identity plus coarse rules** (ruled
  2026-08-04, after holding the design against dbt and dlt): identity is
  the recipe-and-schema hash; a data update must reproduce the schema or
  error (the frozen-contract rule); `DROP TABLE` refuses while data
  exists (PoC), so replacement means a new name; the deletion cascade is
  future work — tricky through relations and actor-generated SQL. No
  reactive invalidation of definitions anywhere: declared `ACCEPTS` edges
  and snapshot staleness are the only freshness mechanisms.
- **Filtered rows are the author's judgment** (ruled 2026-08-04): the
  engine keeps one number, `dropped_rows_count` — source rows minus
  landed rows — transcribed here as an `imports` relation beside `cache`
  (spelling open: relation, or a table-grain glossary row). Which rows
  were dropped is the agent's question, answered on the files.
- **Probe queries need a source binding** (open fork): a probe is
  recipe-shaped SQL executed at the source without landing, transcribed
  here with the source name as the path's first segment
  (`read_parquet('erp_export/orders/*.parquet')`). The alternative is a
  scoped form naming the source outside the path. Undecided.
- **Benford's law dropped** (ruled 2026-08-04): the only domain-leaning
  measurement in the deterministic plane — and the only numpy/scipy
  dependency in it — consumed by nothing as a signal. It never ports;
  whoever wants it writes a script.
- `run_id` versioning and the promote/head flip are the cache and
  supersession mechanics — implementation by ground rule; the only surface
  is deleting cached rows to force recomputation (REFRESH was dropped
  2026-08-03 with the sqlparser respell).
- The vertical binding (`workspace_settings.active_vertical`) is replaying a
  folder (fixture 01) — confirmed against the real frame step, which writes
  seed rows exactly like a replay would.
