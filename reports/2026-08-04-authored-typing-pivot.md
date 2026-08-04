# Pivot: typing is authored in the recipe (2026-08-04)

Ruled by the project lead the same day the M4 typing machinery finished —
the recorded-reads invalidation build (commit `1927d81`) was one day old
when this superseded it. The corpus respell in
`corpus/11-flow-add-source.md` carried the whole flow with **zero grammar
change**, which is the transcription verdict that authorized the build.

## The ruling

The recipe carries the casts. The agent probes the source through the
statement door (recipe-shaped SQL, landing nothing), reads the taught
patterns (fixture 13 — still FACT glosses, now authoring knowledge), writes
the recipe with `try_cast` / `try_to_date` and the column choices, the
human approves. The landed table IS the typed table, snapshotted by
Iceberg on every import. Default recipe: `SELECT *`.

The project lead's argument, condensed: the raw/typed/quarantined
machinery only ever mattered for CSV/JSON (parquet and DB arrive typed),
those are not the main sources, and the landed table is already the frozen
copy — Iceberg snapshots every import, so reproducibility never depended
on an all-VARCHAR twin. The typing dance was v0.3's pipeline reflex
rebuilt in gloss form; v0.3 automated typing because there was no one to
talk to. Our target actor is an agent with full SQL.

## The lifecycle discipline (held against dbt and dlt)

The project lead did not trust reactive invalidation ("we should not take
it easy and think we can just solve it") — and the mature-tool comparison
agreed: nobody in that world reactively invalidates on definition changes.
dbt detects changed models by content checksum and rebuilds coarsely along
declared `ref()` edges; dlt loads append-only with frozen schema
contracts. Definition changes are coarse, explicit, rebuild-from-scratch;
only data updates are incremental; dependencies are declared, never
sniffed.

Adopted:

- **Identity is content** — the recipe SQL and the schema it produces
  (the v0.3 engine already keys recipes this way). Unchanged re-declare is
  a no-op; changed is refused outright.
- **Data updates must reproduce the schema** or error (the frozen
  contract). A data-update verb is future work — unchanged re-declaration
  stays a no-op so vertical replay stays idempotent.
- **`DROP TABLE` refuses while the table holds data or glosses** (PoC), so
  it only removes mis-declarations — whole: lake table, recipe row, cached
  evidence, import records. The deletion cascade is future work (tricky
  through relations and actor-generated SQL).
- **The substrate is allowlisted**: queries and probes pass, `DROP TABLE`
  routes to the rules above, forwarded deletes on `glossary`/`cache` pass,
  everything else (CREATE/ALTER/INSERT/UPDATE/COPY…) is refused. This also
  closed a live hole: `CREATE OR REPLACE VIEW orders AS …` would
  previously have shadowed a served table.
- **Invalidation is two mechanisms, both boring**: the declared `ACCEPTS`
  edge (context-in — our `ref()`), and snapshot staleness marked at read
  (data-in). A table's definition never changes underneath its evidence,
  because change is refused and removal takes the evidence along.

## What was deleted

The derived typed/quarantined pair and `refresh_derived`; the `_raw`
suffix and all raw-name aliasing; `type` and `eligible` as engine-known
aspects with their collapse special cases; the type-expr admission probe;
`infer_types` / `decide_types` / `decide_eligibility` scripts and the
`raw_of` kernel; the recorded-reads invalidation (door recording, `reads`
column, `derived` table, `advance_derived`) — one day old, superseded by
the discipline above; the reserved-suffix rule; type-decision staleness.
The eligibility projection gate (also one day old): column selection is
the recipe's SELECT list.

## What was built

Probe routing (`read_*` references route the statement to the import
context; a probe path's first segment names the source); `try_to_date` /
`try_to_timestamp` moved to `glossql-import::casts` (recipe vocabulary,
registered in recipe, probe, and session contexts); source-row counting in
`run_recipe` and the `imports` relation (`dropped_rows_count` = source
rows minus landed — which rows is the author's question, answered on the
files); the substrate allowlist; `DROP TABLE` with the refusal rules.

## What survives untouched

Witnesses, detectors, bands, the collapse and serve-and-mark; `ACCEPTS`
invalidation; `profile` and `outliers` (reading the landed table);
`slot_entropy`; snapshot stamping; fixture 13's patterns as FACT glosses.

## Open spellings (transcribed one way, not ruled)

- Probe source binding: source name as path prefix vs. a scoped form.
- `imports` as a relation beside `cache` vs. a table-grain glossary row.
