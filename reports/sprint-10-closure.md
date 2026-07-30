# Sprint 10 · Closure — the review phase ends

**DECIDED 2026-07-31: all four forks = A** (project lead): audience-as-aspect
routing · actor-scoped pack export (`GLOSS SEED finance AS PACK`) · REJECTED
groundings with assumptions behind EVIDENCE · AT prefix kept (lake form is a
different scope). Mechanical closures applied from verified evidence:

- **§3.4** — readiness formula made normative (per-intent
  `risk = clamp01(Σ weight·(conflict, ignorance))`, worst intent bands — from
  `loss.yaml`); contract policy completed (`THRESHOLDS`, `WARNING MARGIN`,
  `BLOCK ON` — from `contracts.yaml`); audience routing example.
- **§3.5** — normative minimum relation schemas; `METRIC()` grain ladder
  (day/month/quarter/year) + parameter overrides; GLOSS mechanism guarantees
  (curation disclosure, confidence-state marking); actor-scoped `AS PACK`;
  AT flag closed.
- **§3.2** — `GROUNDING … REJECTED` negative form; confidence-gate sentence;
  VIEW admission and recipe boundary confirmed settled.
- **§3.0** — the semantic admission checklist (7 items), the spec-owned half
  of the envelope per §1.1.
- **§10** — the walkthrough was missing its `DECLARE RELIABILITY`: without it
  the witness pools at zero weight and station 5's contest never happens.
  Found by building the replay simulator, fixed in station 3.
- **Status block** — all sections Ready; held-open trio stays in §1.1.

**Validation now executes.** `harness/replay.py` implements §5's semantics
(slots, per-witness pooling, contested, prefix replay); `check.py` runs the
§10 walkthrough end-to-end with six assertions (declaration occupies slot →
witness pools → contested fires → teach supersedes → contested clears →
replay determinism). `harness/authoring_test.py` is the constrained-decoding
rig: point any LLM at a task + grammar.ebnf, run its output through the rig.

**Corpus complete:** fixture 10 covers the remaining §2.1 rows; every row is
now transcribed, reserved (§6), or dropped with a named replacement.

Remaining open, on purpose: §1.1's held-open trio (persistence backend,
DataFusion mapping, governance), §6's reserved space, and fixture 03's
display-metadata gap block. Next phase per §9.2: the adjudication slice on
DataFusion.
