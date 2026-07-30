# 09 · Answer-agent served context vs GLOSS + DECLARE SERVING — SEMANTICS UNDEFINED

Source: `dataraum-context/packages/cockpit/src/tools/query-context.ts` (+
`query.ts:813-848`). Nine blocks served, byte-identical per session as a cached
prompt prefix (DAT-660): schema (prefer-enriched), dimensions, relationships,
entities, drivers, grain, vocabulary, conventions, business_concepts.

## Transcription — the serving policy §3.4 can state

```glossql
DECLARE SERVING answer_agent
  PREFER enriched
  DIMENSION BUDGET 12
  RESTRICT JOINS TO DECLARED RELATIONSHIPS
  INCLUDE (conventions, drivers, grain_caveats)
  BY USER analyst;

GLOSS (SELECT sum(amount) FROM orders GROUP BY channel)
  USING SERVING answer_agent;
```

## Gap — disclosure as serving semantics (no clause family exists)

```glossql-gap
DECLARE SERVING answer_agent
  PREFER enriched
  DISCLOSE OMITTED (dimensions, relationships)
  BY USER analyst;
```

## Findings

Covered by authored statements + derived state: table inventory and the
prefer-enriched cut, column meanings, stock/flow markers, join whitelist,
dimensions with priority, hierarchies, entity/grain/time/identity, conventions
(minus per-convention `targets`, fixture 02), drivers and readiness as derived.
The one-mechanism/one-policy architecture matches the running system's shape.

Served content with **no authored statement that could produce it**:

1. **Instructional prose** — ~40% of every block is engineer-authored teaching
   text ("Ground EVERY join on a pair listed here … abstain", "(additive) is a
   flow: SUM it…"). §3.4's clauses select *content*; nothing authors the words.
   §7 excludes prompt config — then GLOSS output is f(log, lake,
   renderer-prose), and §7 contradicts §10's "a cockpit feature that cannot be
   written this way is a grammar gap" on the single most-consumed artifact.
2. **Curation-disclosure arithmetic** — "Showing 9 of 41 catalogued dimensions —
   the other 32 were never assessed, not rejected" (DAT-622/671: silence converts
   to false abstention). Load-bearing serving semantics; no clause.
3. **Alias confirmation gate** — confirmed → "group by canonical only";
   unconfirmed → "do NOT merge". A serving rule keyed to adjudication state; no
   clause.
4. **look_values grounding loop** — serves engine-internal column ids so a tool
   can be called; ids are §1.2(5)-banned surrogate identity.
5. Relation schemas for everything in §3.5 are unspecified (spec-flagged), so
   whether e.g. fan-out caution (`introduces_duplicates`) is reachable is open.

Verdict: **SEMANTICS UNDEFINED** (serving clause list is a sketch by the spec's
own flag) + **INFORMATION LOST** (instructional prose unproducible from the log).
