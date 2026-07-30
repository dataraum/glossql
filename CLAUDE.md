# glossql — workspace rules

The context language (SPEC.md) and, later, its DataFusion-based server. Current
phase: **language spec under review**. There is no implementation and none should be
started before the grammar is agreed. One carve-out: a disposable §9.1 validation
harness (grammar parser, replay/pooling simulator, constrained-decoding authoring
test) may exist; its only outputs are transcription verdicts and SPEC.md diffs,
and it does not survive into the implementation.

## The one-document rule (amended 2026-07-30)

**SPEC.md is the only normative prose.** No satellite design docs, no assumption
files, no per-topic notes. Open questions live in SPEC.md §8 and get folded into
the body when decided, not appended as history.

Four non-prose artifacts are first-class — they are fixtures and machinery, not
documents, and they exist precisely so the spec stops absorbing untested ideas:

- `grammar.ebnf` — the machine-readable grammar; the source of truth for syntax.
  Productions marked [REPAIR] are pending SPEC.md diffs.
- `corpus/` — transcriptions of **real** `../dataraum-context` artifacts
  (` ```glossql ` must parse; ` ```glossql-gap ` documents a gap and must fail).
- `harness/` — the §9.1 machinery (parser now; replay/pooling simulator and
  constrained-decoding authoring test to come). Disposable; does not survive
  into the implementation.
- `reports/` — §9.1 outputs: review verdicts and sprint fork write-ups.

**Standing invariant:** `python3 harness/check.py` passes — every ```sql block
in SPEC.md parses, every corpus fixture behaves as tagged. A grammar edit that
breaks it doesn't land.

**Ideation before prose:** no idea enters SPEC.md until it has survived a
corpus test — write 2–3 competing statement forms for the same real artifact,
check them against grammar and the real table shapes, present the forks to the
project lead. Only the surviving fork becomes a SPEC.md diff, and the diff
should shrink or hold the spec, never grow it by essay. An open §8 question
closes only by a transcription verdict, never by argument. Progress is corpus
burn-down over §2's rows, not lines written.

## Grounding

- `../dataraum-context` (sibling repo) is the running v0.3 system and the empirical
  source of the statement vocabulary. When a coverage or semantics question arises,
  grep that repo rather than reasoning from memory — engine metadata models:
  `packages/engine/src/dataraum/`; generated schemas: `packages/engine/schema*.sql`;
  agent context assembly: `packages/cockpit/src/tools/query-context.ts`; config
  plane: `packages/dataraum-config/`. Read its `CLAUDE.md` before working in it.
- SPEC.md §2 is the map from that system's artifacts to grammar constructs. Keep it
  truthful: if the system and the map disagree, verify in code, then fix the map.

## Settled vs. held open

- Settled: language before implementation · DataFusion as engine substrate ·
  log/lake split with state = f(log, lake) · four planes (declarations,
  observations, policies, derived) · the concept/data split with the grounding
  statement (`DECLARE GROUNDING`) as the only bridge · `GLOSS` is the read
  verb, and no word holds two grammatical roles · judgment in policy, never in
  results · authored prose is opaque.
- Held open (do not decide in passing): persistence backend · DataFusion mapping ·
  governance.

## Design authority

- The language design has a single owner: the project lead. Every grammar change is
  reviewed by them. Propose as SPEC.md edits with rationale; don't let the grammar
  drift through implementation convenience.
- Sober docs voice: definition before significance, claims sized to named
  mechanisms, no selling.
