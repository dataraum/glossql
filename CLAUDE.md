# glossql — workspace rules

The context language (SPEC.md) and, later, its DataFusion-based server. Current
phase: **language spec under review**. There is no implementation and none should be
started before the grammar is agreed.

## The one-document rule

**SPEC.md is the only normative document.** No satellite design docs, no assumption
files, no per-topic notes. Proposals are edits to SPEC.md; open questions live in
SPEC.md §8 and get folded into the body when decided, not appended as history. If a
discussion produces something worth keeping, it becomes a SPEC.md diff.

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
  observations, policies, derived) · the concept/data split with GROUND as the only
  bridge · judgment in policy, never in results · authored prose is opaque.
- Held open (do not decide in passing): persistence backend · DataFusion mapping ·
  governance.

## Design authority

- The language design has a single owner: the project lead. Every grammar change is
  reviewed by them. Propose as SPEC.md edits with rationale; don't let the grammar
  drift through implementation convenience.
- Sober docs voice: definition before significance, claims sized to named
  mechanisms, no selling.
