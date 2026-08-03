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
classifier owning the glossql forms, (b) substrate strings handed to
DataFusion verbatim, (c) two registered table functions. No sqlparser fork,
no patched DataFusion. The in-tree example
`datafusion-examples/examples/sql_ops/custom_sql_parser.rs` (`CREATE
EXTERNAL CATALOG`) is the official form of this pattern: a `CustomStatement`
enum wrapping `DFParser` delegation — ours with two heads instead of one.

### Statement spelling vs sqlparser (verified, sqlparser 0.60 × generic/postgres/duckdb)

Question: could re-spelled heads make glossql parseable without the router?
Measured:

- Already valid SQL **today**: `USE`, `SELECT … FROM GLOSSARY(fin.orders.amount)`,
  `… ATTEST(a.b -> c.d.e)` (`->` parses as a binary op; `<->` on the
  postgres dialect only). The language is mostly SQL already.
- `CREATE GLOSS` / `CREATE ASPECT` / `CREATE WITNESS` / `CREATE SOURCE` /
  `UPDATE GLOSS`: **all fail in every dialect** — `CREATE`'s second word is
  a closed set. A CREATE-flavored re-spelling still needs the router; it
  buys nothing and costs the distinct write verb.
- The only router-free spellings are full dissolution (`CALL gloss(…)`,
  `INSERT INTO glossary …` — parse everywhere) with JSON bodies as quoted
  strings. Rejected: the bare `AS {json}` body is the agent-ergonomic core,
  and no SQL dialect accepts bare braces. (`CALL` forms remain available as
  a zero-grammar-cost wire alias for SQL-only clients, if ever needed.)
- `SELECT … FROM t PARALLEL` was a silent-misparse hazard — it *parses* as
  a table alias named PARALLEL. Resolved by decision, not routing:
  **`SEQUENTIAL | PARALLEL` is dropped from the grammar** (2026-08-03).
  Ordering is the caller's choice — one statement with many calls, or many
  statements. `REFRESH` remains the only extraction modifier.

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

### The plan (converged same day, after review with the project lead)

- **Iceberg, baked in.** iceberg-rust as an in-process library behind its
  `Catalog` trait. The glossql server is the only Iceberg client in the
  system — every agent speaks statements over a connection — so a catalog
  server would have exactly one consumer. The `sql` catalog (SQLite file in
  the workspace; Postgres by connection string) runs in-process; a REST
  catalog (e.g. Lakekeeper) is the same trait later — configuration, not
  architecture. Ownership checked: iceberg-rust is Apache-governed with a
  broad committer base; Lakekeeper is a consumer/contributor ("based on
  apache/iceberg-rust"), not the owner. Ref/branch changes travel as
  ordinary `TableUpdate` ops in the commit protocol (Lakekeeper's commit
  path applies `SetSnapshotRef`), so branch semantics are client-side — no
  server anywhere needs to "support branches."
- **Publication = Write–Audit–Publish, implemented post-PoC.** The grammar
  has no transaction surface (sessions ride the connection, like actor), so
  postponing costs nothing at the language level. The server keeps a
  publication seam behind the session boundary:
  - PoC: direct commits to main, **typed flipped first** — typed is the only
    table downstream reads, so mid-session states are benign; raw and
    quarantine are provenance.
  - Post-PoC: session = branch. Write raw/typed to the branch; the audit is
    the measurement/witness phase reading the branch across connections;
    publish is a fast-forward of main. Client gaps are thin and tracked:
    apache/iceberg-rust PR 2709 (`ManageSnapshotsAction` — create/rename/
    replace/fast-forward branches; open, active, unreviewed) plus
    parameterizing the append target ref
    (`transaction/snapshot.rs:540` hardcodes `MAIN_BRANCH`). Carried on a
    fork via `[patch.crates-io]` until upstream lands them.
- **Glossary and declarations: relational — not parquet, not K/V.** Small
  JSON rows; supersession reads are windowed SQL a K/V store would force us
  to reimplement. SQLite in the workspace, Postgres by connection string.
  From day one, gloss and measurement rows carry the data snapshot id they
  were computed against — provenance and staleness need it anyway, and it
  stitches the two ACID domains by reference: readers join glossary rows to
  the snapshot they see, so cross-domain atomicity degrades to referential
  consistency rather than depending on flip timing.
- **Not chosen.** The DuckLake shape (all metadata in one SQL database) —
  evaluated including `datafusion-contrib/datafusion-ducklake` (real, alpha,
  per-table commits, DuckLake compatibility goals that are not ours): good
  inspiration for the commit-time model, wrong substrate to build on. A
  DuckDB extension (PEG parser + ducklake) would have the strongest raw
  transaction story but reopens both stack-eval discriminators — grammar on
  a pre-release parser, scripts back in the muddle.

Snapshots are the versions; refs are the heads; nothing version-shaped
lives in rows or query text.

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
