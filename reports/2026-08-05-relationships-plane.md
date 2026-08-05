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
- **Composite rescue** (added same day on the lead's flag: "we also
  had composite keys"): a to side that is no key alone can be one
  inside a scope — the multi-tenant shape, `(businessID, name)`,
  which is every one of booksql's five declared FKs. Ported as the
  reality, not the machinery: v0.3's greedy width-4 fuse becomes
  width 2 — anchor plus one co-present scoping pair, accepted only
  when the combined to side is near-unique and the two-leg join
  resolves ("DATA decides, not names", v0.3's own rule). The anchor
  is the higher-cardinality to side (the identifying leg), the scope
  the tenant leg; the candidate carries `key_columns` on the open
  remainder, exactly v0.3's semantic-model shape. The declared form
  is unchanged — cure by keyed view, then declare (the corpus rule).
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

## Porting list — realities to carry, not complexity

The lead's rule for this area: v0.3's complexity is not the thing to
keep; what could be *realities in actual data* is.

- **The stock-flow judge** — ruled 2026-08-05: *not* a function voice
  in the `behavior` slots. In v0.3 the function was typically right
  and the agent wrong, but that was before agents could explore; run
  3's agent out-judged the static rule by testing against the ledger.
  The shape is an evidence MEASUREMENT (`behavior_evidence`: tie the
  column to period movements, test summability) that helps the agent
  write the verdict — and `contested` stays what it means, human vs
  agent. A measured voice ranked against claims would smuggle back
  the calibration question the 2026-08-03 pivot dropped. RETURNS onto
  a FACT aspect stays unexercised.
- **booksql** (`../testdata/booksql`, research-use dataset): the
  SQLite (`accounting.sqlite`, 810k-row `master_txn_table`, five
  composite FKs all shaped `(businessID, X)`) is the designated test
  for the relational-source import when the ADBC executor lands; the
  badly-exported CSVs under `Tables/` are an add-source reality test
  before that — broken exports are exactly what probes and authored
  recipes exist for.

## Run 3 (2026-08-05): the plane live — the decline muscle works

Fresh workspace, the eight finance CSVs, the new skills, the
pre-composite script (the rescue landed the same day, after this
run). What the agent did:

- Landed all eight tables, 0 rows dropped, typing authored — this
  time DECIMAL(18,2) for money (exact arithmetic for a ledger that
  must balance; the agent verified Decimal128 reads as numeric in the
  column kernels before committing), DECIMAL(18,6) fx rates, BIGINT
  account numbers, BOOLEAN reconciled.
- 51 profiles, 9 outliers, 6 temporals; meaning + role on all 51
  columns, behavior + unit on the 9 measures, meaning on all 8
  tables — 203 slots current, all bands green, nothing contested.
  The trial-balance-is-turnover finding recurred, tied to the ledger
  and glossed against the column names, contradiction stated in
  prose.
- **`detect_relationships` proposed 12 candidates → 9 distinct edges
  (reverse duplicates folded) → 8 declared, 1 rejected.** Every
  declared edge anti-joins to zero orphans in the declared direction,
  and every reverse-side gap is a business population, to the row:
  the 415 unpaid invoices are exactly open 219 + overdue 56 +
  cancelled 140; the 140 unposted invoices are exactly the cancelled
  ones; the 33 never-posted accounts are the roll-up parents. One
  mutual edge (`bank_transactions.payment_id <-> payments.payment_id`).
- **The rejection is the plane's proof**: `payments.amount ↔
  invoices.amount` scores 0.99 overlap *because the business process
  causes it* (payments settle invoices in full) — a harder false
  positive than parallel sequences. The agent refused it on join
  semantics: 2,566 joined rows with 12 wrong pairings, all 31 partial
  payments missed. Left undeclared and visible in the measurement.
- `trial_balance`'s real grain surfaced as composite —
  `(account_id, period)`, no key alone. The agent glossed
  dimension/timestamp and declined to invent a key: the live argument
  for the composite rescue, which the next run's measurement will
  propose instead of leaving the judge empty-handed.

Still unexercised by this run: composite candidates live (booksql is
the designated next dataset), and the contested arc with a real human
voice.
