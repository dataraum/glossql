# 09 · Answer-agent served context — DROPPED BY DESIGN (the agent experiment)

Source: `dataraum-context/packages/cockpit/src/tools/query-context.ts` (+
`query.ts:813-848`). Nine blocks served, byte-identical per session as a cached
prompt prefix (DAT-660): schema (prefer-enriched), dimensions, relationships,
entities, drivers, grain, vocabulary, conventions, business_concepts. ~40% of
every block is engineer-authored instructional prose; curation disclosure
("Showing 9 of 41 catalogued dimensions…") and alias confirmation gates are
load-bearing serving semantics (DAT-622/671).

## Transcription

None — deliberately. The language has no serving construct: reading is
`GLOSSARY()` / `ATTEST()` and plain SQL; context assembly is an agent skill,
not grammar. The old track's `DECLARE SERVING` policy (PREFER / DIMENSION
BUDGET / RESTRICT JOINS / INCLUDE) and its disclosure guarantees are gone with
the serving document.

```glossql
SELECT * FROM GLOSSARY(fin.orders);
SELECT * FROM GLOSSARY(fin.orders.amount, all => true);
SELECT subject, band FROM ATTEST(fin.trial_balance) WHERE band = 'red';
```

## Findings

- **DROPPED BY DESIGN — and it is the biggest bet in the language.** The
  nine-block served context has no grammar backing; whether agents write
  glossql directly, use skills over these reads, or need a curated layer is
  exactly the experiment the simplification runs.
- The running system's fieldwork is the benchmark the experiment must meet:
  byte-identical cached context prefix per session (DAT-660), curation
  disclosure so silence doesn't convert to false abstention (DAT-622/671),
  alias confirmation gates.
- Nothing to fix in the grammar; everything to learn in the experiment. If the
  experiment fails, the lesson returns as skills or as a read-side construct —
  not as this fixture's old `DECLARE SERVING`.
