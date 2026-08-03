# 2026-08-03 — PoC substrate verification: parser surface, transactions, Rhai boundary

Follow-up to `2026-08-03-stack-eval.md`. Three questions the eval left open,
answered against sources cloned today at main: `apache/datafusion`,
`apache/iceberg-rust`, `delta-io/delta-rs`. File references below are paths
in those repos.

## Statement parsing: what DataFusion actually provides

- The documented SQL extension surface
  (`docs/source/library-user-guide/extending-sql.md`) is **in-statement**:
  `ExprPlanner` (custom operators), `TypePlanner` (custom types),
  `RelationPlanner` (custom FROM elements), registered on the session. None
  of it admits a new top-level statement.
- New statements are done the way DataFusion does its own (`CREATE EXTERNAL
  TABLE`, `COPY`): `DFParser` (`datafusion/sql/src/parser.rs:375`) wraps the
  sqlparser tokenizer, peeks the leading keyword, hand-parses its own
  statement forms, and delegates everything else. The glossql front parser
  is the same pattern with `DECLARE`/`USE`/`GLOSS` heads; the §9.1 harness
  parser is the working model of exactly this split and ports directly.
- The reads need no parser work at all:
  `SessionContext::register_udtf`
  (`datafusion/core/src/execution/context/mod.rs:1621`) registers a
  `TableFunctionImpl` (`datafusion/session/src/table.rs:579`) —
  `GLOSSARY()`/`ATTEST()` become table functions inside otherwise plain SQL.

Consequence for the server: parsing is (a) a statement splitter and head
classifier owning the glossql forms and the `PARALLEL|SEQUENTIAL` tail,
(b) substrate strings handed to DataFusion verbatim, (c) two registered
table functions. No sqlparser fork, no patched DataFusion.

## Transactions: DataFusion has none; Iceberg supplies the per-table piece

Verified in iceberg-rust:

- Per-table optimistic transactions: `Transaction` with `fast_append`,
  schema/property updates, `expire_snapshots`
  (`crates/iceberg/src/transaction/mod.rs`); `commit(catalog)` retries
  retryable conflicts with configurable backoff. **Append-only today** — no
  overwrite or row-delete action yet.
- The DataFusion write path exists: `IcebergTableProvider::insert_into`
  (`crates/integrations/datafusion/src/table/mod.rs:153`), append inserts
  only, loads fresh metadata, commits through the catalog.
- Snapshot pinning for readers is first-class: `StaticTable`
  (`crates/iceberg/src/table.rs:344`) and `IcebergStaticTableProvider` scan
  at a fixed snapshot id with no catalog round-trip — read isolation for a
  long-running session.
- Catalog backends: rest, sql (SQLite/Postgres), glue, hms, s3tables. The
  REST spec's multi-table commit endpoint is mapped
  (`crates/catalog/rest/src/endpoint.rs:171` —
  `POST /v1/{prefix}/transactions/commit`) but no client method exposes it
  yet: cross-table commits are not available from the library today.

delta-rs for comparison: a much richer single-table write surface (delete,
update, merge, optimize, restore — `crates/core/src/operations/`) and
built-in DataFusion integration, but the same absence of cross-table
transactions, and a second table protocol alongside the parquet + Iceberg
line already named. Fallback if append-only Iceberg bites; not the plan.

### The plan: session = transaction

1. Every workspace table — raw, typed, quarantine, glossary — is an Iceberg
   table; per-table atomicity, history, and time travel come from snapshots.
2. A session opens by pinning: resolve every table once, serve all reads
   through static providers at the pinned snapshots. Long-running sessions
   read a consistent world by construction.
3. Session writes buffer as appends. Session end commits each touched
   table's transaction, then flips one **workspace manifest** — a single
   small file naming (table → snapshot id) — as the atomic cross-table
   visibility point. Readers enter through the manifest, so a half-committed
   session is never visible.
4. Growth path: the manifest does locally what the REST catalog's
   multi-table commit does remotely; when iceberg-rust exposes that endpoint
   the manifest can retire without changing the session model.

This replaces run-id-per-row plus head-flip verbosity with standard
machinery: snapshots are the versions, the manifest is the head.

## Rhai boundary: shared, measured

Spike (`rhai-spike`, rhai 1.25.1 / arrow 57.3.1, release build, Apple M5
Pro): a 50M-row Float64 column wrapped as `Col(ArrayRef)` — an Arc handle —
pushed into a Rhai scope.

| probe | result |
|---|---|
| buffer pointer inside the script vs host | identical (`0x4ca000000`) — zero copy; Rhai clones the Arc handle, never the buffer |
| arrow-arith sum on 50M via script call vs native | 7.7 ms vs 13.5 ms (order effects; boundary cost below noise) |
| elementwise `col.get(i)` loop inside the script | ~145 ns/value (1M values in 145 ms) |
| 1000 separate script invocations touching the column | 0.75 ms total |

Consequence: the host API exposes vectorized kernels (arrow compute /
polars-style) as registered functions on column handles; scripts compose
and orchestrate, never iterate rows. At that design point the script
boundary costs nothing measurable.
