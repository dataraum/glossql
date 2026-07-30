# Sprint 2 · The GROUNDING supersession key — decision forks

**DECIDED 2026-07-30: Fork A** (project lead). Applied: SPEC §3.0 keyed-class
key, §3.2 grounding prose + examples (row-level, aggregation to metrics), §8.2
statement-axis sentence closed, §10 example, `grammar.ebnf` (U3 closed),
fixtures 03/07 updated. Open follow-ups spun out: per-assumption confidence
records; a negative GROUNDING form (retained failures).

Gap (fixture `corpus/07`, grammar.ebnf U3): the spec's key is (concept,
relation, parameter); the parameter member has no surface syntax; the real key
is `UNIQUE (snippet_type, standard_field, statement, aggregation, predicate,
schema_mapping_id, parameter_value)`.

## Evidence (verified 2026-07-30)

- `schema_mapping_id` ≈ the workspace (DAT-506: "execute is called with
  `workspace_id=schema_mapping_id` at all call sites", `graphs/agent.py:194-196`)
  — free in glossql's workspace-scoped log. Dissolves.
- `parameter_value` is **constants-only** (`snippet_library.py:147` "(for
  constants)"; extract lookups pass None). Constants are parameter-derived
  values — they follow from `PARAMETER` declarations, they are not groundings.
  The spec's *parameter* key member has no extract-shaped referent. Dissolves.
- The `statement` axis takes exactly **two values** across all 16 finance
  metrics: `income_statement`, `balance_sheet`. §8.2's "plausibly just part-of"
  holds up empirically.
- `relation` is **not** a member of the real key at all (it lives inside
  `parts.from`). Grounding is one LLM call per concept
  (`grounding_collision.py:3-5`); one concept holds one extract per run. The
  spec's "a concept may hold several groundings — per relation" is invented.
- Aggregation is owned **twice**: the metric extract step declares it
  (`aggregation: sum`) *and* §3.2's example puts it in the grounding body
  (`AS sum(amount)`), while §3.1's metric example also writes `sum(revenue)`.
  Incoherent as specified.

## Fork A — One grounding per concept; axes become concepts — recommended

Key = **(concept)**. Every real distinguisher moves where the language already
has a home for it:

```sql
-- statement axis: concept space, per §8.2's own direction (edges exist in the
-- finance pack's compositions already)
DECLARE RELATIONSHIP accounts_receivable PART OF balance_sheet BY SEED finance;

-- grounding: row-level reading — relation, expression, filter. No aggregate.
DECLARE GROUNDING accounts_receivable IN journal_lines_enriched
  AS debit_amount - credit_amount
  WHERE account_type = 'asset'
  BY AGENT grapher CONFIDENCE 0.9;

-- aggregation: owned by the metric expression, where §3.1 already writes it
DECLARE METRIC dso AS 90 * avg(accounts_receivable) / sum(revenue) UNIT 'days'
  BY SEED finance;
```

- A differently-filtered reading is a **different concept** (`reconciled_count`,
  not revenue-with-a-predicate) — DAT-838's predicate case becomes vocabulary,
  which is arguably what it always was: "reconciled count" is a distinct
  business meaning deserving a name.
- Fixes the aggregation double-ownership: grounding maps concept → row
  expression; metrics aggregate concepts.
- Re-grounding supersedes (correction is natural); DISJOINT concepts with
  byte-identical grounding bodies become an admission check (the collision
  guard's post-hoc detection, moved to declaration time — §3.1 already admits
  DISJOINT-based admission checks).
- Cost: concept proliferation for filtered variants — the grapher must *name*
  a concept where today it silently mints a snippet row. That is a feature
  (vocabulary is explicit) and a burden (naming is work). Migration: dso.yaml's
  `aggregation:` fields fold into metric expressions.

## Fork B — Mirror the real key: (concept, statement, aggregation, predicate)

`FOR STATEMENT balance_sheet` clause; aggregation and normalized predicate
hashed into the key. Faithful, but **payload-in-key breaks correction**: editing
a grounding's WHERE creates a sibling instead of superseding — you can never fix
a filter without RETRACT-by-full-payload. Supersession keys must be addressable.

## Fork C — Named groundings

`DECLARE GROUNDING ar_bs FOR accounts_receivable IN … AS …` — a named class;
engine picks among named groundings by policy. Naming burden without the
vocabulary payoff of A; metric→grounding selection becomes policy soup;
portability muddied. Included for completeness.

## Recommendation

**A.** It is the radical simplification, it matches the measured reality on
every dissolved member, it takes §8.2's own direction to its conclusion, and it
repairs an incoherence (aggregation ownership) the spec doesn't know it has.
If chosen: §3.2 grounding prose + example lose the aggregate; §3.0 keyed-class
key becomes (concept); §8.2 closes; fixture 07 rewrites; grammar.ebnf U3
resolves (no parameter surface needed — the member is gone).
