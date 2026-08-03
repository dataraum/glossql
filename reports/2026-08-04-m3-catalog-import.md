# 2026-08-04 — M3 decisions: catalog + import, the Iceberg front door

Decision record for the M3 build-out (`crates/catalog`, `crates/import`,
recipe materialization in `crates/session`). Everything below was verified
against the vendored 0.10.1 sources the same day; file references name
crate-versioned paths under the cargo registry.

## Why Iceberg (re-confirmed by the project lead before the build)

The data plane needs three things: atomic table swaps now (a recipe rebuild
must never tear under a reader — Iceberg commits are a pointer flip),
version identity now (glosses and measurements record the snapshot they saw
— `snapshot_id` stamping), and transactions/updates later (session-as-branch
WAP, the recorded post-PoC plan). M3 uses the first two and keeps the third.
Honestly noted: without the WAP future, plain parquet plus a version column
would cover the PoC — Iceberg is the investment in that future, at the cost
of one catalog file and one thin crate.

## sqlx 0.9 → 0.8 (approved)

iceberg-catalog-sql 0.10.1 pins sqlx `^0.8` with only the `any` feature
(`iceberg-catalog-sql-0.10.1/Cargo.toml:47-50`); the SQLite driver reaches
its `AnyPool` purely by feature unification, which does not cross semver
boundaries. Keeping the store on 0.9 would have compiled two sqlx copies
with the 0.8 one driverless at runtime. One sqlx everywhere; the 0.9-only
`AssertSqlSafe` wrappers reverted to plain strings. Lockstep rule extended:
sqlx moves with iceberg-catalog-sql, as datafusion moves with
iceberg-datafusion.

## The front door, not a parallel layer

The session uses iceberg-datafusion's own surfaces end to end; the earlier
custom-SchemaProvider sketch is dead (project-lead skepticism, confirmed in
source):

- **Tables are live, namespaces are frozen.** `IcebergSchemaProvider` keeps
  its table map in a `DashMap`; `register_table` creates the Iceberg table
  and inserts the provider in one move
  (`iceberg-datafusion-0.10.1/src/schema.rs:148-213`), converting the
  schema itself via `arrow_schema_to_schema_auto_assign_ids` (:163). Only
  the namespace list is built once (`catalog.rs:49-80`), so
  `DECLARE DATASET` remounts — once per workspace in practice.
- **Materialization is CREATE-through-the-provider + INSERT.** An empty
  `MemTable` carrying the recipe result's schema registers the table; the
  staged batches append through `TableProvider::insert_into` (append-only,
  parquet-only — `table/mod.rs:158-240`), which writes the data files and
  commits with retry. The snapshot id is read off the catalog afterwards.
- **Mounting**: the dataset's namespace schema registers in the session's
  default catalog under the dataset name (`fin.orders` resolves); `USE`
  additionally mounts bare-name aliases for the dataset's tables, so
  `orders` resolves while `CREATE VIEW orders_typed` lands beside the
  aliases in the default schema — views never touch the Iceberg schema,
  whose `register_table` would refuse them (`schema.rs:182-185`).
- Freebie noted: the provider serves Iceberg metadata tables through SQL —
  `SELECT * FROM fin."orders$snapshots"` is the table's history.

## Import: file sources run on the server (ruled)

"At the source, in the source's dialect" holds literally for relational
sources (their executor — ADBC — is planned, deferred until Iceberg was
tackled; declaring such a source stores it, running its recipe errors
loudly). A parquet folder has no engine, so the server runs file-source
recipes itself: a scratch DataFusion context where `read_parquet` /
`read_csv` / `read_json` are table functions resolving paths under the
source's `location` root — fixture 11's recipe runs as written. Paths are
fenced to the root (no absolute, no `..`).

- **Location is a root, not a glob** (fixture 11 respell): the original
  transcription's `location: 'lake/erp/*.parquet'` could not compose with
  the recipe's `'orders/*.parquet'`. The globs belong to recipe SQL.
- **Raw shapes**: csv reads with an explicit all-Utf8 schema (leading zeros
  survive — no inferred typing to undo) and csv/json results land raw
  all-VARCHAR, mirroring the running system's import; parquet keeps its
  file types. Typing stays the typed view's business.
- **Type folding**: Iceberg 0.10.1 maps ns timestamps to `timestamp_ns` — a
  format-v3 type (`iceberg-0.10.1/src/arrow/schema.rs`, `TimestampNs`) —
  and rejects `UInt64` outright. Import folds batches to v2-safe types
  before landing: ns/ms/s → µs, `Date64` → `Date32`, `UInt64` → `Int64`,
  `Float16` → `Float32`, view/large string and binary types to the shapes
  Iceberg reads back (`Utf8`/`LargeBinary`).

## Recipe re-declaration (ruled)

Unchanged content is a no-op (and a replay onto a fresh warehouse
materializes the missing table). Changed content rebuilds the table — but is
**refused while glosses exist under it**: a different SQL is a different
table; declare it under another name. Same shape as the aspect rule.

## snapshot_id returns (per the substrate plan)

`glossary` and `cache` rows carry `snapshot_id` — the subject's table
snapshot at write time; NULL for dataset-level subjects, pair paths, tables
not yet materialized, or sessions without a data plane. The §5.3 read shapes
are unchanged; the column lives on the two relations. Provenance and
staleness are a join against snapshot history, never a guess.

## Format version: v2, upgrade-in-place held open

`TableCreation` defaults to format v2 and M3 keeps it deliberately: no v3
feature is usable from iceberg-rust 0.10.1 (deletion vectors need delete
operations the crate lacks; row lineage is not surfaced; the one v3 type
we'd meet — `timestamp_ns` — we fold to µs on purpose), v3 support in
iceberg-rust is itself still being built (apache/iceberg-rust issue 2411,
open), and the wider ecosystem reliably reads v2 today.
`Transaction::upgrade_table_version` moves a table v2→v3 in place when
WAP-era updates make deletion vectors real; there is no downgrade —
defaulting to v3 now would be the anticipation rule broken in file-format
form.

## Accepted limits, named

- **Replacement window.** A changed recipe (unglossed) rebuilds as
  drop-then-create-then-append. The recipe runs before the drop, so a
  failed run leaves the old table standing; between drop and commit the
  table is briefly absent. Benign under the single writer; a true atomic
  REPLACE is an overwrite transaction 0.10.1 does not ship — the WAP
  column again.
- **One staging name.** Materialization stages batches as
  `__glossql_staged` in the session context. Statements execute serially
  per session, so no collision exists today; the Flight layer (M5) must
  keep one-connection-serial semantics or the name grows a nonce.
- **json is the least-tested import path.** It shares the csv code but has
  no test of its own, and nested json fails loudly at the all-VARCHAR cast
  rather than landing. The verdict waits for a real json artifact in the
  corpus.
- **Forwarded DELETE dialect.** The M2 caveat holds, with one silent
  divergence now named: `LIKE` is case-insensitive in SQLite and
  case-sensitive in postgres — theoretical under lower-case subject
  conventions, but real. Resolution recorded (project lead, 2026-08-04):
  the statement surface stays postgres-flavored; the store rides Postgres
  by connection string later, which dissolves the mismatch entirely.

## Deliberately not built

No branch or ref surface (commits target `main` — hardcoded in 0.10.1,
`transaction/snapshot.rs:509-526` — which is exactly the PoC plan), no
user-facing write path beyond recipes (an `INSERT` against a lake table does
work through the provider; nothing depends on it), no ADBC, no view
persistence across sessions (fixture 11 runs in one), no typed/quarantine
materialization — those arrive with the typing flow and reuse the same
materialize path.
