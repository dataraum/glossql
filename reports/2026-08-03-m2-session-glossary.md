# 2026-08-03 — M2 decisions: session + glossary, datafusion 53.1, the cache relation

Decision record for the M2 build-out (`crates/glossary`, `crates/session`).
Everything below was verified against resolved registry sources or probed
empirically the same day; file references name crate-versioned paths under
the cargo registry.

## datafusion 54.1 → 53.1 (approved by the project lead)

No iceberg-datafusion release supports datafusion 54: the pin history is
0.7.0→`^48.0.1`, 0.8.0→`^51.0`, 0.9.x→`^52.2`, **0.10.1 (latest)→`^53.1`**.
Cargo does not error on the mismatch — it silently duplicates the whole
datafusion crate family (53.1 and 54.1 in one tree), which would surface at
M3 as impenetrable trait-mismatch errors when registering
`IcebergTableProvider` on a 54.1 `SessionContext`. A 53.1 workspace unifies
cleanly with iceberg 0.10.1, and arrow stays a single 58.4 in both worlds.

Consequences, all verified:

- The parser moved to datafusion-sql 53.1 (sqlparser 0.61.0, down from
  0.62.0) with **zero code changes** — the full parser suite passed
  untouched. The corpus suite is the repin guard, as intended.
- Every seam M2 uses exists identically in 53.1 (locations below).
- Upgrade rule (now noted in the workspace `Cargo.toml`): datafusion moves
  in lockstep with iceberg-datafusion, never ahead of it.

## The read seam: RelationPlanner, not UDTF + ExprPlanner

The earlier sketch (`register_udtf` + `ExprPlanner` for pair paths) fails on
the spec: DataFusion's default table-function planning accepts only unnamed
arguments — `GLOSSARY(x, all => true)` parses but dies with "Unsupported
function argument type" (`datafusion-sql-53.1.0/src/relation/mod.rs:163`;
54.1 identical at `:170`).

One level up sits the official seam for custom FROM elements:
`RelationPlanner` (`datafusion-expr-53.1.0/src/planner.rs:379`; registered
via `SessionStateBuilder::with_relation_planners`,
`datafusion-53.1.0/src/execution/session_state.rs:1215`; the surface the
DataFusion blog post "Extending SQL in DataFusion: from ->> to TABLESAMPLE"
documents). Registered planners run **before** default relation planning
(`create_extension_relation` precedes `create_default_relation` in
`relation/mod.rs`), and they receive the raw sqlparser `TableFactor`.

Probe-verified (scratchpad `sqlprobe`, bins `udtf` and `relplanner`): one
planner intercepting `GLOSSARY`/`ATTEST` factors plans every read shape in
the corpus — `all => true` (arrives as `FunctionArg::ExprNamed`), zero-arg
sweeps, 1–3 segment subjects, `->`/`<->` pair paths as raw `BinaryOp` over
`CompoundIdentifier`s, aliases, joins of two reads, WHERE/projections over
the results. Consequences:

- No `register_udtf`, no `ExprPlanner`. Pair paths are decoded structurally
  from the sqlparser AST, so the JSON `->` operator
  (datafusion-functions-json, which the session registers for body queries)
  never collides with pair paths — inside these factors `->` never reaches
  expression planning.
- The same planner serves `glossary` and `cache` as plain readable
  relations (argument-less table factors), snapshotted at plan time.
- `DELETE FROM glossary|cache …` is routed by the session to the store and
  executes there verbatim — DataFusion has no DML execution for registered
  providers, and removal-is-SQL is store-side by design. Caveat, accepted
  for the PoC: the statement text crosses from the postgres-flavored parse
  to SQLite; predicates over the two relations' columns are dialect-neutral
  in practice.

## The cache relation (approved: `cache`, per-function removal via WHERE)

SPEC §9 reserved the cache relation's name and schema for when the store
lands; the store landed, so §6 now fixes them: relation `cache`, one row per
(subject, function, arguments) — `(subject, function, args, body,
computed_at)`. Per-function invalidation is the pattern §5.2 already uses
for the glossary:

```sql
DELETE FROM cache WHERE function = 'dso';
```

`DROP CACHE fn` (a new statement head) and `cache.fn_name` (a
table-per-function namespace) were both considered and rejected: the first
grows the grammar for what a WHERE clause already does — the same class of
decision as the REFRESH drop — and the second contradicts "the cache is an
ordinary relation".

## Anticipation removed (project lead ruling, same day)

The substrate report's "day-one `snapshot_id`" columns on `glossary` and
`cache` are **not** in M2: the ruling is zero code ahead of its milestone —
no transactional or provenance surface exists until something writes it. The
column returns at M3, when snapshots exist and extraction records the one it
ran against. Dead helpers went with it (`attest_returns_schema`,
`band_is_valid`, an unused `Session::store()` accessor). After the sweep,
nothing in the codebase points at a future design; the `FunctionRuntime`
trait stays because M2 tests exercise it — it is the extraction executor's
working interface, not a reservation.

## Q&A rulings (project lead, same day)

- **ATTEST aspect addressing is `subject::aspect`** — the host's cast
  spelling, probe-verified against the stock parser (arrives as
  `Expr::Cast` with a custom "type"). The dot form is retired: it was truly
  ambiguous — corpus fixtures had `ATTEST(fin.trial_balance)` (a table) and
  `ATTEST(fin.reconciliation)` (an aspect) as the same shape. Corpus
  (06/11/12), SPEC §7.2, and grammar.ebnf respelled; unknown aspects after
  `::` error loudly.
- **Raw-read `kind` is the aspect's kind** (fact | query | measurement);
  who spoke is `actor`, under `witness`. SPEC §5.3 now says so.
- **Aspect re-declaration**: identical content is a no-op; changing an
  aspect while glosses under it exist is refused — bodies never silently
  stop matching their schema. SPEC §5.1 now says so.
- **No `ACCEPTS`, no arguments** — passing any is an error ("otherwise it
  invites hacking tries"). SPEC §6 now says so.
- **Zero-arg sweeps** (`GLOSSARY()` / `ATTEST()` over the `USE`'d dataset)
  are now SPEC and grammar prose, not just corpus usage.
- **Detector adjudication happens at read time**: when checker functions
  exist (M4), the collapsed read is where the detector is consulted — noted
  so M4 shapes the read path, not a write hook.

## Correction: metrics are not functions (project lead ruling, same day)

Fixture 03's transcription ("metric = function script") was **wrong**, and
the running system confirms it: metrics move through declare → compose →
execute, where execute is "the agent runs the composed SQL" and the working
SQL lands as `sql_snippets`
(`packages/engine/src/dataraum/pipeline/phases/metrics_phase.py`);
validations are the same shape (bind grounds the spec as SQL, execute runs
it — `validation_phase.py`); statistics run as engine SQL on DuckDB.

The corrected model, per the project lead:

- **A metric is a concept** — a QUERY aspect, run as its SQL. Fixture 03 is
  respelled: the yaml's ontology half is the `WITH` schema, compose is an
  agent glossing the composed SQL, execute is running it. SPEC §1/§2/§5.1/§6
  updated; no extraction statement touches a metric.
- **The function library is the engine's analytical machinery moving into
  the server as rhai scripts — all of it.** A function is either a
  measurement (fills a MEASUREMENT aspect through its witness) or a
  detector. `DECLARE FUNCTION`, extraction, and the cache stay — they were
  never for metrics. The M2 extraction executor and `FunctionRuntime` stand.
- Still open with the project lead (follow-ups announced): the witness
  respell questions and parameter mechanics for composed metric SQL.

## Function interface (ruled 2026-08-04)

Design questions raised by the metric correction, answered by the project
lead; verified against the engine functions being ported
(`profile_statistics(table_id, duckdb_conn, session, max_workers, config)`,
`detect_outliers_iqr(table, column, duckdb_conn)` — data via a connection,
config from the config plane, no caller-chosen arguments anywhere):

- **Settings are context.** `ACCEPTS (aspect, …)` names declared aspects;
  the server assembles the script's context document from their current
  values — nearest value walking up from the subject (subject → parent →
  dataset), null when nothing is glossed. The inline-schema and
  `'producer#/pointer'` forms are gone (the pointer form was dso residue).
  Fixture 13 carries the corpus evidence (`infer_types ACCEPTS
  (type_patterns, null_values)`).
- **Calls are bare.** `f()` — named call arguments left the grammar; the
  cache is one row per (subject, function), its `args` column dropped. An
  old-spelling call (`f(x => 1)`) is not an extraction: it falls through to
  substrate SQL and fails loudly at planning.
- **Scripts are not fenced.** They may run any SQL against the dataset —
  the engine's functions already hold a connection; determinism is the
  script's contract and the workspace the boundary. The subject rides along
  as metadata with its SQL schema and neighborhood (parent, siblings,
  children). The M4 rhai runtime gets a query capability; nothing is built
  ahead of that milestone.
- **Detectors see no table data.** A detector receives the witness's slots
  and threshold, returns band + score. Functions that must *run* SQL stored
  in glosses (fixture 12's `reconcile_aggregates` executes a concept's
  groundings and compares the numbers) need nothing special: the SQL
  arrives as text in their context and runs through the same SQL door every
  script has. The parked §9 item stays what it always was — whether the
  *server* ever runs grounding SQL implicitly inside a `GLOSSARY()` read,
  rather than the reader doing it.

## Provisional semantics, flagged for their corpus tests

- **Collapse policy without a detector run** (detectors land M4): exactly
  one current slot value → serve it; more than one → NULL. Honest but
  minimal; the §9 open question (NULL conflating never-assessed / contested
  / gated) still closes only by the fixture-09 corpus test.
- **Sweep scope — resolved**: `GLOSSARY(orders)` serves the table, its
  columns, and every relationship it participates in (either side); the far
  endpoint's own context is never pulled in. `all` stays a boolean — the
  subject already names breadth, the `glossary` relation already serves full
  history. Now SPEC §5.3 prose.

## Transactions status check (same day)

- iceberg-rust released 0.10.1 on 2026-08-01. PR 2709
  (`ManageSnapshotsAction`) is still open, zero reviews, in no release. In
  0.10.1 the append target is hardcoded (`SetSnapshotRef { ref_name:
  MAIN_BRANCH }`, `iceberg-0.10.1/src/transaction/snapshot.rs:510,523`) and
  `TableCommit`'s constructor is `pub(crate)` — there is no public path to
  move an arbitrary ref. The recorded plan holds: M3 commits straight to
  main, typed flipped first; branch WAP stays post-PoC behind the
  publication seam, fork-with-`[patch.crates-io]` the price of doing it
  earlier.
- Lakekeeper implements the REST commit-transaction endpoint as a real
  atomic multi-table commit and authorizes `SetSnapshotRef` per ref — a
  later catalog option, but it does not close the client-side gap.
- **Lance** (branching + shallow clone, blog post 2026-02-16, author Jack
  Ye — who designed Iceberg's branching): by-root branches with physically
  isolated directories and per-branch history fix real Iceberg problems,
  but those problems target multi-tenant high-frequency experimentation;
  the post itself credits Iceberg's model with serving write–audit–publish
  well, and WAP with a single writer is exactly our shape. Decisive today:
  shipped Lance has create/checkout/tag/clone but **no merge or
  publish-back-to-main operation** (that is the aspirational `lance-git`
  section), and publish-as-pointer-flip is the one operation our
  transaction story needs. Verdict: Lance joins delta-rs on the fallback
  list — it is DataFusion-native, so it stays live — and nothing about
  M2/M3 changes.
