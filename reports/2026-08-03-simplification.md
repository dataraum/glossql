# 2026-08-03 — the simplification pivot

On 2026-08-03 the project lead replaced the 2026-07 draft spec (ten review
sprints, ~1100 lines, ~20 statement classes) with a radically smaller
language, drafted as BRIEF.md and refined over ten Q&A rounds. The result was
folded into SPEC.md the same day; BRIEF.md is gone. This report is the pivot
record: what replaced what, what was deliberately dropped, and what the
re-transcription of the corpus showed. The old track — spec, grammar,
harness, sprint reports — lives in git history (commits "iteration 1–9").

## The shape of the new language

Seven declaration heads (SOURCE, RECIPE, DATASET, RELATIONSHIP, ASPECT,
FUNCTION, WITNESS), one write verb for context (GLOSS, body always JSON),
two table-function reads (GLOSSARY, ATTEST), plain SQL for everything else.
The load-bearing decisions:

- **Aspect trichotomy.** `DECLARE ASPECT [name] WITH [json_schema] AS
  [MEASUREMENT | FACT | QUERY]` — the kind lives on the declaration; the
  gloss statement is uniform. FACT bodies validate against the aspect's
  schema; QUERY bodies against the fixed standard grounding schema
  ({sql required, assumptions[] optional}); MEASUREMENT is never glossed and
  fills from the witness-bound function's cache.
- **No fact names; no BY clause.** The supersession key is (subject, aspect,
  actor kind); the actor rides the connection.
- **Functions are scripts** with ACCEPTS/RETURNS JSON contracts. Metrics,
  checks, profiling, and detection all leave the grammar.
- **Witness slot model.** Per (subject, aspect): one measurement slot, one
  agent slot, one human slot. A detector function reads the slots and returns
  band + score per the fixed attest schema; contested is a band, not a flag.

## Dropped by design

Each cut is a named decision, not an omission:

| dropped | old-track home | where it went |
|---|---|---|
| calibration, per-witness reliabilities, pooling ("calibration theater") | DECLARE RELIABILITY, posteriors | detector function internals — swappable code |
| serving / curated context | DECLARE SERVING, GLOSS-as-read | agent skills over GLOSSARY/ATTEST — the experiment (fixture 09) |
| pack envelopes, versioning, portability | GLOSS SEED … AS PACK, §6 | a vertical is a folder of scripts + declarations, ported by copying |
| negative/rejected forms | RELATIONSHIP … REJECTED, GROUNDING … REJECTED | not declared = does not exist; candidate memory is a MEASUREMENT aspect |
| declarative metric expressions, parameters, interpretation bands | DECLARE METRIC / POLICY | function scripts (fixture 03) |
| validation construct | DECLARE VALIDATION (6 clauses) | aspect + gloss + function + witness (fixture 04) |
| cycle constructs | ordered VALUES, TERMINAL, CYCLE FAMILY | FACT aspects with x-order/x-terminal annotations (fixture 05) |
| teach vocabulary | 8 Zod-validated teach types | re-gloss on a human connection (fixture 08) |
| readiness bands, contested flag, RETRACT, AT prefix | §3.4 policies, lifecycle | ATTEST bands; DELETE FROM glossary; history is implementation |

The accepted trade running through all of it: adjudication and analytics are
reproducible from statements **plus scripts**, no longer from statements
alone. The log stays small and diffable; the logic is code.

## Corpus re-transcription

All ten fixtures re-transcribed 2026-08-03 against the new grammar; harness
green (every ```glossql block parses). Verdicts:

- **TRANSCRIBES: 01, 02, 03, 04, 05, 07, 08, 10.** Fixture 04
  (trial_balance) is the strongest confirmation: the old track's richest
  construct reduces to four general-purpose statements. Fixture 02's
  `targets` routing — deferred for two sprints on the old track — lands
  trivially in-blob.
- **06:** slots transcribe; calibration dropped (above).
- **09:** dropped by design — the biggest bet: whether agents work from
  GLOSSARY/ATTEST reads + skills without a curated serving layer. The running
  system's DAT-660/622/671 fieldwork is the benchmark.
- Old-track gaps that closed by relocation rather than by grammar: display
  metadata (03), workspace-scoped vocabulary teaches (08), per-assumption
  records (07 — assumptions now ride inside the grounding body).

## Open after the pivot

- Standard grounding schema exact fields (sketch: sql required, assumptions[]
  optional) — SPEC §9.
- ACCEPTS pointer syntax ([schema]#[json_pointer]) — placeholder, SPEC §9.
- Postponed: actor transport · access rights · portability · persistence and
  engine substrate (tech-stack briefing upcoming).

Harness note: `replay.py` and `authoring_test.py` were deleted with the
pooling model they simulated; `check.py` + `glossql_parser.py` remain, with
JSON payload validation added (json.loads on every WITH/AS/ACCEPTS/RETURNS
body).
