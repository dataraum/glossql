# The relationships plane — fixture 12's judge pattern, first slice

Date: 2026-08-05. The judge pattern as the project lead defined it:
v0.3's statistical evaluators are tuned toward high recall, and the
judge's one job is removing false positives. Relationships is the
first plane (ruled 2026-08-05, relationships first), because both
acceptance runs already showed agents doing the judgment side by
instinct — anti-joins before every `DECLARE RELATIONSHIP`, orphans
grounded before trusting an edge. This slice turns that instinct into
shipped method: a measurement, a skill, and the machinery test.

## The v0.3 swipe: five judge lanes, one shape

Surveyed in `../dataraum-context` (begin-session pipeline,
`worker/workflows.py` and the analysis modules). Every LLM phase is a
judge over a deterministic measurement, and the code says so in its
own words — slicing: "a judge, not an elector"; hierarchies: "stats
decide; this judge fills the two identity questions no statistic can
settle."

- **semantic_per_table** — the archetype: consumes evaluated
  relationship candidates (overlap, cardinality, per-side uniqueness,
  referential integrity, orphan counts, served per pair) and
  confirms/declines; a measured composite-key rescue is offered and
  the judge only picks the columns — cardinality is code-computed,
  never LLM-echoed. Also reads per table: fact/dimension, grain, time
  columns (event vs attribute, exactly one anchor axis), identity
  columns — fixture 12's `entity` gloss transcribes this verdict.
- **enriched_views** — which joins extend a fact, and which columns
  are worth carrying; the grain check stays deterministic, runs after
  the agent, and overrides it.
- **slicing** — existence and relevance are measured regardless of
  the judge; the agent adds `interest: primary|supporting` (the
  ordinal priority was retired for the absolute enum) plus
  business-context prose.
- **catalogue_semantics** — the blanket business reading: per-table
  entity type and purpose; per-column meaning prose,
  `determination: determined|ambiguous`, unit source column, a
  derived-formula hypothesis, a stored-sign claim
  (`natural_balance|ledger_signed|unsure`) — fixture 02's territory.
  These extend the glossing plane (`glossql-add-source`), not a new
  flow.
- **dimension identity** — alias-vs-coincidence on statistically
  identical 1:1s, cross-fact conform with a mandatory concept label; a
  failed judge call means the stats' verdict stands, never a
  deterministic fill-in.

Two constants worth keeping: every structured verdict carries prose
grounds beside it, and v0.3 kept *removing* numeric confidence gates
so the judge sees everything — the same ruling glossql made for
collapse.

**Skill family (ruled 2026-08-05): skills follow deliverables, not
v0.3 phase names.** No begin-session skill, ever — the name is a
semantic leftover. Catalogue-semantics content extends
`glossql-add-source` when wanted; slicing and dimension identity
share a dimensions deliverable if they land; relationships stands
alone now. The judge *loop* is language-level and lives in the core
`glossql` skill ("measurements over-produce — you are the judge").

## What shipped

- `functions/relationships.rhai` — the high-recall detector, at
  dataset grain (`SELECT detect_relationships() FROM fin`), all SQL
  through the script's door: landed tables from `imports`, per-column
  filled/distinct counts, near-unique columns become `to` sides,
  every type-compatible column is tried as a `from` side, distinct-
  overlap joins score each pair; anything where half the from side
  resolves survives, sorted by overlap. Candidates carry the core
  fields (`from`, `to`, `cardinality`, `overlap`) and evidence
  (`matched`, `orphans`, `from_distinct`, `to_distinct`) on the
  aspect schema's open remainder. Same-table pairs stay in — they are
  hierarchy candidates.
- `relationship_candidates` aspect + `detect_relationships`
  declaration in the boot library — fixture 12's spellings verbatim.
- **The `relationships` relation** — the judge must see what is
  already declared; the store's RELATIONS gained
  `relationships (dataset, left_path, op, right_path)` and the
  planner, wire schema, and cap policy derived it for free.
- `.claude/skills/glossql-relationships/SKILL.md` — the plane's
  method: measure, judge every candidate (anti-join both directions,
  ground the orphans, distrust coincidence, verify cardinality),
  declare survivors (`->`/`<->`, same-table hierarchy, composite cure
  by keyed view), gloss the grounds on the pair path, read back.
  Rejects stay visible in the measurement.
- The machinery test (`crates/scripts/tests/relationships.rs`): a
  true edge with one orphan and a coincidental key/key decoy both
  arrive (recall is the contract), the declared survivor reads back
  from `relationships`, the reject stays in the measurement.

## Flagged, not fixed

A dataset-grain measurement shows up as `unassessed` rows on every
table and column in `GLOSSARY()` sweeps — nothing declares an
aspect's grain, so disclosure assumes every subject might speak to
it. Reads narrow past it (`WHERE state = 'current'`), but the noise
is real; whether aspects should declare grain is a language question
for the project lead, not something to decide in passing.

## Next: run 3

The populated run-2 workspace, a fresh agent, the new skill. The
dataset holds exactly the traps this plane is for: the dangling
`invoices.vendor_id` (20 vendors, no vendor table — an edge that must
be *declined* for a missing endpoint), unused `fx_rates` (candidates
with no business meaning), and the chart-of-accounts self-hierarchy.
The interesting outcome is the decline muscle: what the agent
declares matters less than what it leaves in the measurement.
