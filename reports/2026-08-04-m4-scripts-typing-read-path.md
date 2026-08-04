# 2026-08-04 — M4 decisions: scripts, typing, the disclosing read

Decision record for the M4 build-out (`crates/scripts`, the derived
typed/quarantined pair, detectors in the read path, write-invalidation).
Substrate claims were verified in vendored sources the same day
(rhai 1.25.1, datafusion 53.1); the typing semantics were grounded in the
running v0.3 system before the design was ruled.

## The rulings, in order

- **Necessity is Rust, judgment is scripts.** The raw/typed/quarantined
  progression is engine mechanics; a configurable script can implement an
  invariant but cannot guarantee one. Inference stays rhai — changeable
  beyond the stock patterns is a feature there.
- **The typing decision is a witness slot.** `decide_types`' pick fills the
  `type` slot by default (v0.3's automatic decision, as a script); agent
  and human glosses supersede it by the ruled precedence. Typing needs no
  agent in the loop; the agent is the override case.
- **Naming: typed is the default surface.** The recipe lands `<t>_raw`;
  the bare name is always the engine-derived view (identity until decisions
  exist); `<t>_quarantined` is the complement. Suffixes are engine-owned
  (admission refuses recipes claiming them). Migration reads this as
  "drop the prefix": v0.3's `lake.typed."t"` consumers respell to the bare
  name; `lake.raw` references respell to `<t>_raw`.
- **Nothing triggers on a gloss — the read is the trigger.** Derivation
  compares the emitted view SQL per statement and `CREATE OR REPLACE`s only
  on change; v0.3's "apply the teaching" machinery does not come back.
- **Writes invalidate, reads recompute, judgment only supersedes.** A new
  aspect value kills the caches of functions that `ACCEPTS` it, at and
  under the subject; a changed type decision kills the table's evidence,
  sparing the typing machinery (speakers on `type`, plus speakers on the
  aspects those functions accept — derived from declarations, never from
  guessing what a script read). No machinery ever deletes a gloss.
- **The collapsed read discloses; fixture 09's mechanical half closed.**
  "Serving wrong information is not an experiment": the shape gains
  `state` — `unassessed` (witnessed, nobody spoke; the row appears),
  `contested` (withheld, band + score), `current`, `stale` (**served and
  marked** — suppression of judgment by machinery was rejected). What
  remains open in §9 is only whether agents use the surface.
- **Detectors run at read.** A verdict missing or older than the newest
  slot write recomputes inside `ATTEST()` / collapsed `GLOSSARY()` and
  caches; `DELETE FROM cache` still forces it. The deciding argument: the
  reads sweep, extraction has no sweep form, and the language carries no
  ordering surface to fan a detector out with.

## Grounded before building

- **Views on our stack cannot serve silently wrong data** (tested, not
  argued): an Iceberg-backed table re-reads the catalog per query, so a
  view over a rebuilt table serves fresh data when the schema held and
  fails loudly when it changed; only plain in-memory tables show the
  silent-staleness DataFusion is capable of (frozen provider Arc in the
  view's plan — no re-resolution anywhere in 53.1). Derivation regenerates
  on decision change anyway, which also covers the loud-error case.
- **rhai 1.25.1**: `sync` + `serde` are required (Engine/AST are not
  Send+Sync without `sync` — native.rs:37-60, ast.rs:19; the serde module
  is the JSON bridge both ways). `Engine::new()` rebuilds the standard
  library per call (packages/mod.rs:172-177), so the runtime assembles from
  `new_raw` plus one shared `StandardPackage`. The default module resolver
  reads the filesystem and its base path is not a jail (file.rs:272-290) —
  imports get the dummy resolver. Every limit defaults to unlimited; only
  `max_operations` is set (a runaway backstop — scripts are
  workspace-trusted by the M2 ruling, this is a tripwire, not a sandbox).
- **DataFusion 53.1 has no failure-tolerant format parsing**: `to_date` /
  `to_timestamp` take chrono formats but abort the whole scan on one dirty
  value, and no `try_` variant exists. The session registers
  `try_to_date` / `try_to_timestamp` (NULL on failure) — needed by
  patterns with or without the derived views. Fixtures 08/13 respelled
  (`STRPTIME` was DuckDB vocabulary); a corpus sweep found no other
  DuckDB-isms — everything else already executes in tests.

## Built

`crates/scripts`: `RhaiRuntime` behind the existing `FunctionRuntime`
seam — compile-once ASTs (recompiled when the file's text changes), scope
constants `subject` / `context` / `db`, the script's final expression is
the result. Kernels on the zero-copy `Col` handle: count, null_count,
distinct, min/max/sum, `match_rate(regex)`, `parse_rate(type)`. The trait
grew the SQL door (`db.query(...)` — the M2 promise); detectors get a door
that refuses (§7.1). The reference library rides in
`crates/scripts/functions/`: `profile`, `infer_types` (patterns nominate on
values, trial casts verify), `decide_types` (best candidate above
min_confidence, VARCHAR fallback), `slot_entropy` (fixture 06's contract).

Session: derivation module emits the pair as SQL text (the inspectable
record) with every decision wrapped in `TRY_CAST` — v0.3's `TRY_` rewrite
rule in gloss form: a bad expr yields NULLs and quarantine rows, never a
broken view. Store: collapse states, precedence, the two invalidation
rules, `newest_slot_write` for detector freshness.

The acceptance test (`crates/scripts/tests/fixture11.rs`) runs fixture 11
with the real scripts end to end: recipe → inference → automatic decision
(DOUBLE by trial cast, DATE through the taught pattern's expr) → typed
view under the bare name (`sum(amount)` = 127.65, the dirty row in
quarantine) → agent override (evidence dies, machinery survives, the view
follows) → detector green-then-red as a human disagrees → contested
withholding → unassessed disclosure rows → the teach flow (re-glossed
patterns invalidate, re-extraction re-decides, the view follows) — with
no orchestration statement anywhere.

## Accepted limits, named

- **`type` is a name convention.** The typing machinery keys on the aspect
  literally named `type` (`glossql-glossary`'s `TYPE_ASPECT`). Nothing
  marks it in the grammar; renaming the aspect renames the convention.
- **A script's data dependencies are undeclared.** Invalidation follows
  `ACCEPTS` and the subject; a script that reads *other* tables than its
  subject has a dependency nothing tracks. The subject is the dependency
  proxy; determinism stays the script's contract.
- **Detector `computed_at` in the body is hollow** (`""`): scripts have no
  clock worth trusting; the authoritative timestamp is the cache row's,
  which `ATTEST()` serves. The attest RETURNS schema still requires the
  field — a respell candidate if it grates.
- **Refresh cost rides every statement**: decisions are re-read and the
  pair's SQL re-emitted per substrate/extraction statement (cheap text
  comparison, no data movement). Fine at PoC scale; a decision-version
  counter is the obvious optimization if it ever shows.
- **`skip_physical_aggregate_schema_check` is on**: Iceberg's arrow fields
  carry `PARQUET:field_id` metadata; a cast in a derived view drops it
  logically but not physically, and 53.1's aggregate schema check trips on
  the difference (the config knob exists for exactly this,
  datafusion-common config.rs:532).
- **User-authored views over rebuilt tables** can still hit the loud
  schema-mismatch error (they regenerate nothing). Named, not fixed: it
  cannot produce wrong data.

## Deliberately not built

No eligibility implementation — but the fork closed (project lead,
2026-08-04): eligibility is **its own gloss**, never a boolean inside the
type body. Whole-body supersession decides it: the flag would be hostage
to every fresh typing decision (the function's pick knows nothing of
eligibility and would silently drop it), and the fix would be merge-on-
write — the overlay fixture 13 killed. Its own aspect supersedes
independently and can grow its own witness chain. The concrete spelling
lands with its fixture when eligibility is built. No `raw.` alias layer
(ruled the same day: not needed — nobody reads raw). No detector beyond
slot-entropy, no ADBC, no serverd; the M5 staging-name obligation stands.

## Recorded for M5 (project lead, 2026-08-04)

The cockpit's query door is a plain HTTP endpoint streaming Arrow IPC —
arrow-js consumes the stream natively (`RecordBatchReader.from(fetch)`),
in Node and the browser, and the batches already exist in memory. The
TypeScript Flight SQL client ecosystem was surveyed and rejected as a
dependency: every package is stale or tiny (a 2-star 0.1.0, a 3-year-old
publish, a 9-months-quiet repo) — thin wrappers worth copying if Flight
from TS is ever needed, not depending on. Flight SQL stays the
engine-grade door for Python and peer engines; serverd carries both,
plus the MCP shim.
