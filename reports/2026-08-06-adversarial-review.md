# Adversarial review — the PoC server, and the ledger of what is open

Date: 2026-08-06. The first adversarial pass over `crates/` (11,589
lines): three independent reviewers — correctness and robustness,
performance and memory, the doors — plus `cargo clippy --workspace
--all-targets` (clean) and the workspace suite (30 binaries, green).
Every defect below was then **reproduced live** against a throwaway
workspace on serverd, or refuted there. Reproduction beats report: one
reviewer finding died on contact with the running server, and the
severity of the top finding was only visible once the store was
inspected afterwards.

## 1. Confirmed defects, ranked

### 1.1 A shared detector holds one verdict, and it is served for every aspect

The evidence cache is keyed `(dataset, subject, function)` — the
aspect and the witness are not in the key (`store.rs:137-145`,
`cache_put` `:1221`). `ensure_verdicts` skips recomputation whenever a
row for `(subject, detector)` is newer than the newest slot write
(`reads.rs:120-127`); `collapsed_read` and `attest_read` then map that
single row onto **every** witness's aspect (`store.rs:1030-1047`,
`:1166-1195`).

Reproduced: two aspects on `t.c`, both witnessed by `slot_entropy`.
`alpha` is genuinely contested (a human and an agent disagree);
`beta` has one human slot. `ATTEST` reports **both** red at score 1.0,
and `GLOSSARY` reports `beta` as `contested` — withholding a value no
one disputes. The control — `beta` alone in its own dataset — reads
`current`, green.

Blast radius: sharing a detector is the shipped idiom. Both live
workspaces wire `slot_entropy` across `role | behavior | unit` and
`framework_bands` across 3–4 validation aspects, and both stores hold
exactly one cache row per `(subject, function)` — 64 subjects in
`~/glossql-ws`, 50 in `~/glossql-ws-fin`.

Past runs are **not** invalidated: every `(subject, aspect)` pair in
both workspaces has exactly one speaker kind, so every entropy is 0
and every verdict green whichever aspect produced it. The defect bites
the first time a human contests an agent — which is precisely the
two-roles deployment and the pinning agenda.

The fix is a design question, not a patch: the verdict is currently
the detector's cached *function output* (SPEC §7.2), and a function
value is legitimately keyed by subject alone. Either the detector's
rows carry their witness, or verdicts leave the function cache.

### 1.2 `DELETE FROM glossary|cache` forwards raw multi-statement SQL to SQLite

`store_delete` validates the parsed target, then hands
`inner.to_string()` to `sqlx::raw_sql`, which executes any number of
`;`-separated statements with no binding (`store.rs:1292`,
`session.rs:900`). Single-quoted literals survive the round trip
safely; **dollar-quoted** ones — the spelling every skill teaches —
are re-emitted byte-verbatim, and SQLite reads `$q$` as a bind
parameter rather than a quote, so the body is parsed as SQL.

Reproduced twice through the door: `DELETE FROM cache WHERE subject =
$q$ ; CREATE TABLE injected_marker(a); --$q$` created a table inside
the store, and the same shape ran `UPDATE glossary SET
actor_kind='human' WHERE actor_kind='agent'` — the agent's gloss now
outranks a human's in every collapsed read. The one invariant the
model tests is defeatable from outside. `DROP TABLE glossary`,
`ATTACH` of any SQLite file, and `VACUUM INTO '<path>'` are the same
statement away.

### 1.3 `PROBE` and `DECLARE RECIPE` bodies never meet the allowlist

Both call `ctx.sql()` on a scratch context with default `SQLOptions`,
so DDL, DML and `COPY` all plan and execute (`session.rs:439`,
`import/src/lib.rs:116-192`). The router's allowlist never sees this
SQL. Reproduced: `PROBE tmpsrc AS $$COPY (SELECT 42 AS pwn) TO
'<outside the workspace>' STORED AS PARQUET$$` wrote the file; a
source declared at `location '/private/tmp'` read a CSV from outside
the workspace and returned its rows. `db.query()` from any rhai
function reaches the same door on the **live** session context.

Three call sites, one line each:
`SQLOptions::new().with_allow_ddl(false).with_allow_dml(false)
.with_allow_statements(false)`.

### 1.4 `SELECT … INTO` creates a session-lifetime table past the allowlist

`SELECT … INTO x` is a `Statement::Query`, so it passes the allowlist
keyed on statement variant (`session.rs:666-676`); DataFusion plans it
as `CreateMemoryTable` and executes it. Reproduced: `SELECT 1 AS a
INTO scratch_tbl` then `SELECT * FROM scratch_tbl` returns the row.
The substrate-is-closed invariant is bypassed, and the copy lives in
the session for the process's life.

### 1.5 Re-land drops the live table before the new recipe is known to work

On `Replaced`, the table is deregistered (dropped from the catalog)
and its evidence wiped **before** `materialize` first executes the new
SQL (`session.rs:307-333`). A typo'd path destroys a landing with no
rollback, leaving glosses pointing at a table that no longer exists.
The day-old ruling is right; the ordering is wrong.

### 1.6 Smaller, confirmed by reading

- A function that `ACCEPTS` the aspect it `RETURNS` invalidates its own
  cache row inside `cache_put`, then panics on
  `expect("row just written")` (`session.rs:608`, `store.rs:1243`).
  `declare_function` admits the declaration.
- Store relation names shadow landed tables: a table named `sources`
  or `imports` is served from the store, silently, unless qualified
  (`reads.rs:217-223`).
- `Scope::predicate` interpolates subjects into `LIKE` unescaped
  (`store.rs:63-80`): `order_items` and `orderxitems` share
  invalidation, and a `%` in a subject turns any invalidation into a
  dataset-wide cache wipe.
- Re-declaring an aspect counts glosses only, so cached function
  values survive under the new schema (`store.rs:446-464`) — and for
  MEASUREMENT aspects the guard can never fire, since glossing one is
  refused.
- A NaN in an imported float passes every guard in `classify_series`
  (`NaN > x` is false) and panics `median`'s `partial_cmp` expect
  (`scripts/src/lib.rs:812-839`).
- A failed materialization leaves the recipe row committed, so the
  identical retry answers `(unchanged)` over an empty table
  (`store.rs:357`, `session.rs:301`).

## 2. Performance and memory

Four advertised properties, checked against the code:

- **Streaming end to end on `/query`** — confirmed. `execute_stream`,
  a channel of capacity 2, and a client hangup cancels upstream
  (`query.rs:55-107`).
- **The MCP row cap bounds engine work** — confirmed for a bare single
  query (`wire.rs:48-70`), **refuted** for everything else:
  `Session::substrate` does `frame.collect()` and `rows_json` trims
  afterwards (`session.rs:679`), and `PROBE` collects in full. On
  those paths the cap is a display cap, not an engine bound — the same
  shape as the `top_k(20)` lesson, in a different place.
- **Content-keyed AST cache** — confirmed: read per invocation,
  full-text compare, recompile only on change.
- **Entropy is one pass** — confirmed, O(n) over typed `cell_keys`.

Costs worth naming, none structural: no secondary indexes on
`glossary`/`cache` under an O(n²) `NOT EXISTS` supersession predicate
(invisible at 300 glosses, tens of ms at 3k); SQLite in default
journal mode with no WAL and no transactions around multi-write flows
(a `GLOSS` is 4+ separately fsynced commits; a 50-column sweep ≈ 200);
read amplification — one collapsed `GLOSSARY()` issues O(W·F + W·S)
queries and fetches the slot set about three times; import buffers the
whole landing and re-scans every source to count rows; `distinct()`
builds a `String` per cell where the u64 `cell_keys` already exists;
`extract` runs sync file IO and rhai evaluation on the async executor
without `spawn_blocking`.

Worth doing now: WAL plus two indexes, and routing the execute path's
final query through the capped stream.

## 3. What came back clean, and what the tests refuted

Clean after genuine inspection: lock discipline (no guard held across
an `await`, so no panic above can poison one), the import
`..`/absolute-path fence, the script-root fence, the supersession SQL
and the human-over-agent rank mapping, the parser (no reachable
unchecked index or overflow), body caps (4 MiB `/mcp`, 2 MiB
`/query`), malformed-input handling (415/404/400; bad glossql returns
a tool error, not a protocol error), sqlparser's recursion guard,
session reuse per actor, and Iceberg read-context caching.

**Refuted by test:** the claim that the stateless MCP path collapses
every client to the actor `"rmcp"`. `initialize` is required (a bare
`tools/call` answers 422), and `clientInfo` rides: `client-alpha` and
`client-beta` landed as distinct actors in the store. Actor transport
works as designed on the door a real client uses.

Standing, by design: no authentication. It is load-bearing here —
every finding above is reachable by anything that can open a socket,
and `/query` is a full write door that speaks as the configured
**human** actor unconditionally (`query.rs:24-28`), so door choice is
rank choice.

## 4. The ledger — what is open

Ruled and waiting on a trigger: the `metric.` table-function bind (the
UI transformation) · the witness convention that bands agent-only
definitional glosses yellow · the conformed-group structured-field
fork (fixture 15) · RelBench grading, occasionally.

Waiting on the lead: the three definitions the scorecard run surfaced
(interest income in revenue, the gross-profit subtrahend, the DPO
denominator) · two scorecard corrections — `gross_profit` is
definition-sensitive and its row should carry the pinned formula, and
the oracle's FCF is bank-based.

Open in the spec: SPEC §9's remaining half — whether agents compose
their context from the reads, sweeping `state != 'current'` and
respecting bands. Runs 6 and 7 answer most of it; the honest reading
is that bands were never independently computed per aspect (§1.1), so
the band half of that question has not actually been tested.

Named but unbuilt: axis additivity (fixture 15, SEMANTICS UNDEFINED) ·
the `\N` sentinel wall, hit by two consecutive runs and still only
documented in a gloss.

Held open, untouched: persistence backend · engine substrate mapping ·
governance and access rights · cross-workspace portability.

## 5. What v0.3 still holds that we have not ported

Ported already: profiling with exact entropy, relationship detection,
stock/flow behavior evidence, dimension relevance, hierarchies, typing
at import, outliers, temporal profiling. Dropped by design: pooling
and calibration, served/curated context, envelopes, the bus-matrix UI,
declarative metric expressions. The remainder, classified by what
would pull it:

**Pulled by evidence already in hand — the correctness floor:**

1. **Null-token vocabulary and novel-sentinel detection**
   (`dataraum-config/null_values.yaml`,
   `entropy/detectors/value/null_token_adjudication.py`). Runs 5 and 6
   both hit `\N` literals; we document them in a gloss and nothing
   acts on them.
2. **Cast-failure visibility** (`entropy/detectors/structural/types.py`,
   the quarantine tables). We made typing authored, which means we own
   the risk: a `try_to_date` that fails silently nulls a column, and
   the import counts rows, never cells.
3. **The aggregation-lineage search** — the half of behavior evidence
   we did not port (`analysis/lineage/processor.py`): it *competes*
   event-time axes and enumerates sign conventions (`debit − credit`),
   selecting by Wilson lower bound with a ΔBIC>10 tiebreak. We port
   the reconciliation statistic that judges a convention, not the
   search that finds one — the finance agent did that by hand.
4. **Grounding-collision guard** (`graphs/grounding_collision.py`):
   two disjoint concepts grounding to the same rows make every ratio
   between them compute 1.0, silently. Cheap, and aimed straight at
   the metric framework.
5. **Slice-conditional nullness** (Cramér's V on nullness × slice
   under a Cochran gate,
   `entropy/detectors/value/slice_conditional_null.py`): a null
   concentrated in one slice biases every grouped read. Our profile
   reports a flat null ratio only.

**Pulled by the UI transformation** (already deferred): drivers
(`analysis/drivers/*` — permutation-null significance, ratio and
entity targets, hierarchy-collapsed candidates), the mosaic-sql
composition layer (`cockpit/src/duckdb/parts.ts` — recomposition from
persisted clause parts, never text mutation), drill-axis ordering, and
the **additivity classifier** (`graphs/additivity.py` +
`additivity_resolver.py`) — which is exactly fixture 15's undefined
item, and whose rules our metrics skill already teaches in prose.

**Pulled when a metric target demands it:** `period_resolver`
(days-in-period measured from the flow's own observed window, not a
constant — our DSO names `days[w]` and the agent computed it by hand)
· `boundary_resolver` (a period label is the instant the period
starts) · `validity_scope` (a posted-only/reconciled-only predicate
composed onto every grounding — the finance agent scoped by hand) ·
concept reconciliation (`semantic/reconciles_with.py` — two groundings
of one concept must tie out) · derived formulas (pulled when the data
embodies definitions as precomputed columns) · business cycles (the
taxonomy is opinion; the validity predicate it yields is the
load-bearing part) · surrogate and composite-key mint (booksql's
composite case).

**Not wanted:** enriched views (joins are inline, ruled) · column
eligibility (we do not drop columns) · run-versioned read views,
snapshot heads and the property graph (supersession replaces them) ·
LLM prompt infrastructure (skills replace it) · validation induction
as engine machinery (the agent does it) · readiness/loss rollup
(ATTEST bands) · Benford (no target pulls it) · graph topology and
dimensional entropy (the latter is orphaned in v0.3 itself).

## 6. Assessment

The code is compact and disciplined for its age — small modules,
comments that record why a rule exists rather than what a line does,
guards defaulting to refusal, and a supersession-as-a-read design that
avoids a whole class of update bugs. Every defect found clusters in
one shape: **a decision keyed by less state than it semantically
depends on** — a verdict keyed without its aspect, a cache
invalidated by an unescaped `LIKE`, an allowlist keyed on a statement
variant that hides a DDL, a delete forwarded on the assumption that a
Postgres round trip is safe in SQLite. None of it is structural
weakness; all of it is a narrow widening plus a test the suite does
not yet have.
