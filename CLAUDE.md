# glossql — workspace rules

The context language (SPEC.md) and its server. Current phase: **PoC server
build-out** (started 2026-08-03; the language was agreed by the project lead
after the same-day simplification pivot — the 2026-07 draft lives in git
history, and the stack and storage decisions are recorded in `reports/`).
Milestone 1 is the statement spine; corpus fixture 11 is the PoC acceptance
test. Grammar changes still follow the corpus-first process below.

## The one-document rule

**SPEC.md is the only normative prose.** No satellite design docs, no
assumption files, no per-topic notes. Open questions live in SPEC.md §9 and
get folded into the body when decided, not appended as history.

Four non-prose artifacts are first-class — fixtures and machinery, not
documents:

- `grammar.ebnf` — the machine-readable grammar; the source of truth for syntax.
- `corpus/` — transcriptions of **real** `../dataraum-context` artifacts
  (` ```glossql ` must parse; ` ```glossql-gap ` documents a gap and must
  fail). Fixtures 11–12 model the system's operational flows as statement
  sequences.
- `crates/` — the Rust PoC server, a Cargo workspace at the repo root.
  Directories unprefixed, package names `glossql-*`; datafusion moves in
  lockstep with iceberg-datafusion, sqlx with iceberg-catalog-sql (see the
  workspace `Cargo.toml` comment and the M2/M3 reports). Built: `parser`
  (GlossqlParser wrapping DataFusion's DFParser; the corpus is its
  acceptance suite) · `glossary` (sqlx store, supersession, admission,
  collapse states, ACCEPTS-invalidation, imports counters) · `session`
  (SessionContext assembly, RelationPlanner reads, statement router with
  the substrate allowlist, recipe materialization, probe routing,
  DROP TABLE lifecycle, detector-at-read) · `catalog` (the workspace Lake:
  iceberg-rust SqlCatalog on SQLite + warehouse dir; datasets are
  namespaces) · `import` (recipe and probe execution over file sources,
  try_to_date/try_to_timestamp, source-row counting; ADBC executor
  planned) · `scripts` (rhai runtime behind FunctionRuntime, zero-copy
  column kernels, the reference function library under
  `crates/scripts/functions/`). Typing is authored in recipes (ruled
  2026-08-04) — no derived views, no raw twin, no typing functions.
  Planned (a crate is created only when it has real content): `serverd`
  (Flight SQL + MCP shim).
- `reports/` — pivot records, review verdicts, and evaluation records.

**Standing invariant:** workspace `cargo test` passes — every ```sql block
in SPEC.md parses and every corpus fixture behaves as tagged (the
`glossql-parser` suite), and the store and session suites hold the execution
semantics. A grammar edit that breaks it doesn't land. (The Python harness
retired 2026-08-03 when the parser suite replaced it.)

**Ideation before prose:** no idea enters SPEC.md until it has survived a
corpus test — write competing statement forms for the same real artifact,
check them against grammar and the real table shapes, present the forks to
the project lead. Only the surviving fork becomes a SPEC.md diff, and the
diff should shrink or hold the spec, never grow it by essay. An open §9
question closes only by a transcription verdict, never by argument.

## Grounding

- `../dataraum-context` (sibling repo) is the running v0.3 system and the
  empirical source of the statement vocabulary. When a coverage or semantics
  question arises, grep that repo rather than reasoning from memory — engine
  metadata models: `packages/engine/src/dataraum/`; generated schemas:
  `packages/engine/schema*.sql`; agent context assembly:
  `packages/cockpit/src/tools/query-context.ts`; config plane:
  `packages/dataraum-config/`. Read its `CLAUDE.md` before working in it.
- SPEC.md §2 is the map from that system's artifacts to grammar constructs.
  Keep it truthful: if the system and the map disagree, verify in code, then
  fix the map.

## Decided so far — work in progress, not settled

The project lead may reopen any of it; nothing below is sign-off:

- language before implementation · one dataset per workspace (binding in the
  app) · everything-context is JSON against JSON Schemas · the aspect
  trichotomy (`AS MEASUREMENT | FACT | QUERY`) with one uniform `GLOSS`
  statement · supersession key (subject, aspect, actor kind) · actor rides
  the connection, no BY clause · functions are scripts with JSON contracts ·
  witness slot model with detector adjudication (band + score) · judgment in
  detectors and read policy, never in results · authored prose is opaque ·
  `GLOSS` is the write verb, `GLOSSARY()`/`ATTEST()` are the reads.
- Dropped by design (see `reports/2026-08-03-simplification.md`): calibration
  and pooling, serving/curated-context constructs, pack envelopes and
  portability, negative/rejected forms, declarative metric expressions.

## Held open (do not decide in passing)

Persistence backend · engine substrate and its mapping (tech-stack briefing
by the project lead is upcoming) · governance and access rights · actor
transport mechanics · cross-workspace portability.

## Design authority

- The language design has a single owner: the project lead. Every grammar
  change is reviewed by them. Propose as SPEC.md edits with rationale; don't
  let the grammar drift through implementation convenience.
- Sober docs voice: definition before significance, claims sized to named
  mechanisms, no selling.
