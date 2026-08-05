# Contested slots through the doors — and the correction that reframed it

Date: 2026-08-05. Built under the name "judge pattern"; the project lead
corrected the frame the same day. The judge pattern (fixture 12) is the
begin-session loop — a high-recall measurement produces candidates, an
agent judge removes the false positives — and that lands as the
relationships plane (next report). What this slice actually is: the
**disagreement mechanics** through the doors — kept, on the lead's
ruling, as the basis for more detectors.

## The arc as tested

`crates/serverd/tests/judge.rs`, real runtime, real bootstrap:

1. The agent glosses `behavior = flow` through the MCP door; one voice,
   `ATTEST` green.
2. The human glosses `stock` on the same slot through `/query`.
3. The dispute crosses the wire: band red (score 1.0 over the 0.7
   threshold), and `GLOSSARY()` serves `value = null,
   state = 'contested'` — withheld, not adjudicated.
4. Closure, two routes:
   - **Convergence** — a voice re-grounds and supersedes its own slot;
     the newer slot outdates the verdict, the next read recomputes,
     green, the value serves again by precedence.
   - **Strike** — the human deletes the disputed slot
     (`DELETE FROM glossary WHERE … AND actor_kind = 'agent'`); the
     surviving voice stands, green, `state = 'current'`.

## The rule that fell out: a strike invalidates verdicts

Verdict freshness at read (ruled 2026-08-04) is a timestamp
comparison: recompute when a slot write is newer than the verdict.
Deletion makes the slot set *smaller*, never newer — so before this
slice, a struck slot left the red verdict reading as fresh and the
collapsed read kept withholding a value nobody disputed anymore.

Folded in as store semantics (`Store::forward_delete`): a glossary
delete that removed rows drops every detector verdict cache; verdicts
recompute at the next read, which is where they live anyway. Blunt by
design — the forwarded SQL runs verbatim, so which subjects changed is
unknown, and a detector pass over slots is cheap. Extraction caches
are untouched (they answer to ACCEPTS invalidation, not slot
deletion). The strike-closure test fails without this rule and passes
with it.

The glossql skill grew the matching teaching: what `contested` means
and what an agent does about it — re-ground, supersede only if the
evidence moved you, never re-gloss just to end a contest; closure
otherwise belongs to a human.

## The correction (project lead, 2026-08-05)

Two reframings, both recorded so they steer what comes next:

- **The judge pattern is measure → judge → declare**, not contested
  slots. v0.3's statistical evaluators were tuned toward high recall;
  the judge's one job is removing false positives. Contested slots are
  the *disagreement edge* — real, but a different, smaller thing.
- **Humans do not volunteer disagreement.** Nobody parses dozens of
  columns and judges them; the UX is triage — detectors compute, the
  human is *shown what is red*, then looks. That is why v0.3
  calculated all those detectors. The reads exist
  (`SELECT subject, aspect, band, score FROM ATTEST(fin)
  WHERE band = 'red' ORDER BY score DESC` through `/query`); the
  cockpit's job is to lead with them. The consequence: red only exists
  where a detector computes it, so the detector *library* — not the
  contested mechanics — is the human-side bottleneck. `slot_entropy`
  is its first entry; the strike-invalidation rule is shared basis for
  the rest.
