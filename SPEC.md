# glossql — v0 draft specification

Status: **draft for review**. The name: a *gloss* is a marginal annotation
explaining a text's meaning; a glossary is a collection of them.
Scope of this document: the language only. One document; no satellite docs.

Status by section — the iteration loop works the flagged items (§9.1):

- **Ready** (reads unambiguously for practitioners and agents): §1 · §3.0 ·
  §3.6 · §4 · §5 · §7.
- **Under iteration** (statement forms hold; clause details do not):
  - §3.1 — `INTERPRET` ranges and per-step sanity ranges sit awkwardly against
    principle 3 (judgment belongs in policy, not declarations); concept
    relations are open (§8.3); cycle vocabulary is open (§8.5); the `PARAMETER`
    clause shape (type, options, grain derivation) is a sketch.
  - §3.0 — the semantic admission checklist is unwritten; the log envelope is
    deliberately last (§1.1).
  - §3.2 — typing/null/expectation teaches (§8.4) have no statement form yet;
    the rest transcribes cleanly.
  - §3.3 — aspect/claim-space model under review (§8.1, §8.2).
  - §3.4 — `WEIGHT` semantics, the contract policy, and the serving clause list
    are sketches.
  - §3.5 — relation schemas for `CONTEXT()` / `READINESS`, the
    `METRIC ... BY grain` syntax, and the `CONTEXT FOR` output shape are
    unspecified.

---

## 1. Definition and scope

glossql is a declarative extension of SQL for the analytics context of a dataset: the
assertions made about data, the evidence gathered from data, the policies for judging
that evidence, and the serving of all three to analytics agents. A glossql context is
a pair of stores:

- **the log** — an append-only sequence of glossql statements (text). Small, diffable,
  attributable, portable.
- **the lake** — the data itself plus bulk observation results (columnar). Large,
  append-only.

**Core invariant: state = f(log, lake).** The log contains only *authored* events —
every statement names an identified actor. Everything computed (posteriors, verdicts,
readiness, projections) is derived deterministically by the engine and is never
written to the log. Replaying the log against the lake reproduces the full context,
at any historical point.

### 1.1 Held-open decisions (explicitly not made here)

- Persistence backend for log and lake — deferred. Leading candidate: the DuckLake
  design (parquet data files, all metadata in a transactional SQL database), with the
  catalog DB holding only rebuildable projections of the log plus derived state —
  never a second source of authored truth. Whether to conform to the DuckLake spec's
  tables on the data half (DuckDB-attach interop) or only adopt the pattern is open.
- Mapping onto DataFusion extension points — deferred until the grammar is agreed.
- Access control / governance — reserved, no v0 design.
- Orchestration (when observations run, retries, scheduling) — engine concern, not
  language. The language says *what*; never *when*.
- Log envelope (timestamps, sequencing, admission mechanics) — decided **last**,
  after statement semantics settle. Transactional mechanics (ordering, atomicity,
  isolation) are inherited from the persistence substrate — the log is
  workspace-scoped and single-writer; no distributed design is targeted. The spec
  will own only the semantic admission checklist (which contextual checks gate
  writing statements).

### 1.2 Design principles

1. **Four planes, one grammar.** Declarations, observations, policies, consumption
   share one statement skeleton and one provenance model.
2. **The concept/data split.** Vocabulary (concepts, metrics, validations,
   conventions) is written in *concept space* — dataset-independent, portable.
   Assertions about actual data (annotations, relationships, glosses) are written
   in *data space*. `GLOSS` is the only bridge. This makes the analytical layer
   portable across datasets by construction.
3. **Judgment lives in policy, never in results.** Derived state carries numbers;
   bands, severities, and verdicts are policy applied at read time.
4. **Authored prose is opaque.** Meanings, conventions, guidance are string literals
   the engine transports but never parses. The grammar formalizes the envelope, not
   the prose.
5. **No surrogate identity in the language.** Subjects are structural paths
   (`orders.amount`), pairs (`orders.customer_id REFERENCES customers.id`), or
   declared names (`metric dso`). Cross-time identity is textual, by construction.
6. **Mechanism in grammar, vocabulary in declarations.** The grammar never
   enumerates domain specifics: claim spaces, concept vocabularies, and their
   groupings are declarations — importable, supersedable — never keywords. The
   same holds for detectors: the grammar knows the actor class, never a roster.

---

## 2. What the current system says — the map

Grouped by the four verbs. Each entry names the construct (§3) that expresses it.
This map is the completeness check for the grammar: every row is either covered,
reserved (§6), or deliberately excluded (§7).

### 2.1 We DECLARE (asserted, carries provenance)

| Today | Content | glossql construct |
|---|---|---|
| concepts / concept edges | vocabulary: name, description, indicators, kind, relations | `DECLARE CONCEPT` |
| conventions | opaque prose rules served verbatim | `DECLARE CONVENTION` |
| metrics + parameters + dependency DAG | expression over concepts, unit, output, parameters, interpretation ranges | `DECLARE METRIC` |
| validations | check, tolerance, severity, guidance, cycle scope | `DECLARE VALIDATION` |
| cycle families | closed families + directions | `DECLARE CYCLE FAMILY` |
| workspace calendar / vertical binding | fiscal year start; active vertical | `DECLARE CALENDAR`; vertical binding has no construct — importing a pack is replaying its statements (§3.1) |
| column annotations (LLM) | role, business name/description, behavior claim, null tokens | `ANNOTATE <column>` |
| column concepts (LLM) | meaning (prose), temporal behavior, unit source, derived-formula hypothesis | `ANNOTATE <column>` |
| table entities (LLM) | entity type, role (fact/dimension/snapshot), grain, time axes, identity columns | `ANNOTATE <table>` |
| relationships — confirmation half | type, cardinality, confirmed-by | `DECLARE RELATIONSHIP` |
| hierarchies — judged half | drilldown/alias/role kinds, levels | `DECLARE HIERARCHY` |
| slice definitions — ranked half | priority, business context | `DECLARE DIMENSION` |
| enrichment selection | which neighbours enrich a fact table, exposed columns | `DECLARE ENRICHMENT` |
| business cycles (LLM) | cycle assertion, stages, status column, completion semantics | `DECLARE CYCLE` |
| surrogate key confirmation | composite-key intent confirmed/declined | `DECLARE KEY` |
| groundings (snippet parts + provenance basis) | concept → relation, expression, filters | `GLOSS` |
| teach payloads (all 8 types, today free JSON) | type patterns, null tokens, units, plus the families above | the same statements, `BY user` — type/null/expectation teaches pending §8.4 |
| sources / tables | where data lives | `DECLARE SOURCE`, `DECLARE TABLE` |

### 2.2 We MEASURE (computed against data; requests are authored, results are not)

| Today | glossql measurement id |
|---|---|
| statistical profiles (counts, nulls, cardinality, top values, min/max) | `profile` |
| statistical quality (Benford, outliers) | `quality` |
| temporal profiles (span, granularity, gaps, staleness) | `temporal` |
| type candidates (parse rates, patterns, detected units, quarantine) | `typing` |
| column eligibility gates | `eligibility` |
| derived-column detection (formula match rates) | `derivation` |
| relationship candidates (value overlap) | `overlap` |
| functional dependencies (g3) for hierarchies | `fd` |
| aggregation lineage (stock/flow witness) | `aggregation_lineage` |
| driver rankings | `drivers` |
| additivity resolution inputs | `additivity` |
| validation execution (deviation, magnitude) | `validation` |
| 17 entropy detectors across 4 layers | detector ids (open vocabulary) |

The measurement vocabulary is open and versioned; detectors register into it.

### 2.3 We COLLECT (stored evidence)

Two shapes, and the distinction drives §4:

- **Witnesses** — claim distributions over closed spaces with reliability
  (today: `claim_witnesses`). Small, per-subject, the substrate of adjudication.
- **Bulk evidence** — profiles, distributions, per-row artifacts, execution results
  (today: JSONB `evidence` columns, profile tables, validation results). Large,
  columnar, referenced not inlined.

### 2.4 We PROVIDE (served to agents and humans)

| Today | glossql construct |
|---|---|
| six-block answer-agent context (schema+meanings, entities, curated dimensions, relationship whitelist, drivers, grain caveats) + conventions + snippet vocabulary | `CONTEXT FOR`, shaped by `DECLARE SERVING` policy |
| engine GraphAgent served context | same `CONTEXT FOR`, different serving policy — one mechanism, two policies |
| readiness surfaces (bands, drivers, coverage) | `READINESS()` relation |
| why-tools (adjudication audit) | `EXPLAIN <claim>` |
| look-tools (values, profiles, metrics, validations) | context relations (§3.5) |
| property-graph projections (`og_*`) | engine-internal; traversal served via context relations, no PGQ dependency |
| validation verdicts (computed on demand) | derived: `deviation <= tolerance` at read, tolerance from the declaration |

### 2.5 Derived (computed, never authored, never in the log)

Pooled posteriors and conflict flags · readiness bands · validation verdicts ·
additivity conclusions · graph projections · rendered enrichment SQL · rendered
gloss SQL. All queryable (§3.5); none writable.

---

## 3. The grammar

Notation: lowercase = nonterminal, `UPPER` = keyword, `[...]` optional, `{...}` repeated,
`|` alternatives. Sketch-level: statement forms are normative, clause details are
illustrative pending review. Token-level grammar (identifier quoting, string
literals, comments, keyword case) is inherited from the engine substrate's SQL
dialect — DataFusion's PostgreSQL-style parser. glossql adds statement forms,
not a lexer.

### 3.0 Shared skeleton

```
statement   := writing | reading
writing     := declaration | observation | witness | policy | lifecycle
reading     := consumption
declaration := DECLARE family subject clauses provenance ';'
provenance  := BY actor [ CONFIDENCE number ] [ EVIDENCE ref ]
actor       := USER name | AGENT name | DETECTOR name | SEED name | CALIBRATION name
subject     := table | table '.' column | pair | declared_name
pair        := table '.' column REFERENCES table '.' column
             | table '(' column {',' column} ')' REFERENCES table '(' column {',' column} ')'
```

- Only **writing** statements enter the log; **reading** statements (§3.5) are
  session-ephemeral — never logged, never part of replay.
- Every authored statement carries `BY`. The actor classes generalize today's
  `source`/`confirmation_source`/`detection_source` vocabularies into one clause.
- `CONFIDENCE` is logged provenance metadata in [0, 1] — never an adjudication
  input. The pooling plane ignores it; serving decides whether and how consumers
  see it (prompts govern what an LLM must do with it, as today).
- **Supersession:** a declaration's natural key is *(subject, aspect)*. Re-declaring
  the same key supersedes; history remains in the log. `RETRACT` removes without
  replacement. No in-place mutation exists.
- Prose payloads are single-quoted string literals, opaque to the engine.

Three kinds of names cross statements, and only one touches code:

- **Defined names** — the declaration is the complete definition: concepts,
  metrics, validations, aspects, conventions, hierarchies, enrichments,
  dimensions, tables. The log carries everything there is to know.
- **Attribution names** — actor names (`USER analyst`, `DETECTOR
  aggregation_lineage`, `SEED finance`). Never defined, never resolved: they
  attribute. Statements join to each other by these names entirely within the
  log — a witness `BY DETECTOR x` meets `DECLARE RELIABILITY DETECTOR x` by
  string equality, no code required. A replayed log therefore reproduces full
  adjudicated state with no detector installed; detector code only *produces*
  new evidence. Identity behind actor names is governance (held open).
- **Implementation names** — measurement ids in `OBSERVE`, the one place the
  language references executable behavior. The engine's own registry is
  inspectable through the `MEASUREMENTS` relation (§3.5), but it is not the
  boundary of the detector world (§3.3 — the witness statement is the detector
  interface): fulfillment by the engine, by an external worker, or by nobody is
  an orchestration outcome (§7), never a language error.

Policy kinds (`POLICY readiness`, `POLICY contract`, `SERVING`) are none of the
three: they name engine **mechanisms** that the declaration parameterizes. A
policy clause is only as defined as the mechanism contract behind it (§5 for
pooling and banding). Mechanisms are finite and spec-defined; vocabulary is not
— the grammar enumerates mechanisms, never domains (§1.2(6)).

### 3.1 Vocabulary statements (concept space)

```sql
DECLARE CONCEPT revenue
  KIND measure
  DESCRIPTION 'income from sales of goods and services'
  INDICATORS ('revenue', 'sales', 'turnover')
  EXCLUDE ('cost', 'expense')
  UNIT FROM currency
  BY SEED finance;
-- dimension concepts may declare ORDERING (ordered | nominal);
-- concept relations (part-of, reconciles-with, disjointness) are under design (§8.3)

DECLARE CONVENTION accrual_basis
  STATEMENT 'amounts are recognized when earned, not when paid'
  BY USER analyst;

DECLARE METRIC dso
  AS 90 * avg(receivables) / sum(revenue)      -- identifiers denote concepts here
  UNIT 'days'
  PARAMETER period GRAIN month DEFAULT last_complete
  INTERPRET (ok 0..45, warn 45..75, critical 75..)
  BY SEED finance;

DECLARE VALIDATION receivables_roll_forward
  KIND balance                      -- balance | comparison | constraint | aggregate
  ON CYCLE order_to_cash
  OVER (receivables, revenue, collections)
  TOLERANCE 0.01
  SEVERITY error
  GUIDANCE 'opening receivables plus revenue minus collections reconciles with
            closing receivables; a gap usually indicates unposted collections'
  BY AGENT inductor;

DECLARE CYCLE FAMILY conversion DIRECTIONS (forward, reverse) BY SEED finance;
DECLARE CALENDAR FISCAL YEAR STARTS april BY USER analyst;
```

A context's **glossary** is implicit: it is the set of concept-space declarations
currently active in the log — what is defined. There is no pack construct and no
binding statement (`USE VERTICAL` does not survive). A vocabulary pack is a file
of statements; importing it is replaying it, and provenance (`BY SEED finance`)
records where each declaration came from. Collisions between packs resolve by
supersession like everything else. Publishing/distribution format stays reserved
(§6).

Rule: inside `DECLARE METRIC` / `DECLARE VALIDATION` expressions, bare identifiers
resolve against a single concept-space namespace: concepts, declared metrics, and
parameters. A name that would denote more than one of these is a declaration-time
error. Columns are unreachable here by construction — `GLOSS` is the only bridge —
so the expressions stay portable. Metric composition is plain reference:
`operating_income` in another metric's body denotes the declared metric.

Validations carry **no formal check expression** — principle 4 taken fully. The
authored surface is the typed envelope (kind, cycle scope, tolerance, severity)
plus opaque guidance prose; the executed SQL is derived at run time and auditable
via `EXPLAIN`. `OVER` is the membership contract: every name it lists must
resolve in the glossary, so a fabricated reference fails at declaration time.
Validation *induction* is not a language mechanism — it is an agent authoring
these statements (`BY AGENT inductor`), grounded in served context.

### 3.2 Data statements (data space)

```sql
DECLARE SOURCE erp_export FROM 'lake/erp/*.parquet' BY USER analyst;
DECLARE TABLE orders FROM erp_export BY USER analyst;

ANNOTATE orders
  ENTITY 'sales order'
  ROLE fact
  GRAIN (order_id, line_no)
  TIME AXIS order_date ANCHOR
  BY AGENT cataloguer CONFIDENCE 0.9;

ANNOTATE orders.amount
  MEANING 'gross invoiced amount per order line'
  UNIT 'EUR'
  BEHAVIOR flow
  NULL TOKENS ('', 'n/a')
  BY AGENT cataloguer CONFIDENCE 0.92;

DECLARE RELATIONSHIP orders.customer_id REFERENCES customers.id
  CARDINALITY many_to_one
  BY AGENT judge CONFIDENCE 0.97;

DECLARE RELATIONSHIP txn (account, business_id) REFERENCES coa (account_name, business_id)
  CARDINALITY many_to_one
  BY AGENT judge CONFIDENCE 0.9;
-- composite keys are the multi-column pair form; today's surrogate-key intents

DECLARE RELATIONSHIP orders.customer_id REFERENCES customers.id
  REJECTED
  BY USER analyst;
-- a negative declaration: asserts absence, occupies the same (subject, aspect)
-- key, so a later confirming declaration supersedes it — and vice versa

DECLARE HIERARCHY geo IN customers
  LEVELS (country > region > city)
  KIND drilldown
  BY AGENT judge;

DECLARE DIMENSION orders.channel
  PRIORITY 0.8
  CONTEXT 'primary go-to-market split'
  BY AGENT slicer;

DECLARE DIMENSION orders VIA customer_id TO customers.segment
  PRIORITY 0.7
  BY AGENT slicer;
-- a dimension subject may be a path: fact table, FK role, reachable attribute

DECLARE ENRICHMENT orders_enriched FROM orders
  JOIN customers VIA (orders.customer_id REFERENCES customers.id)
  EXPOSE (customers.region, customers.segment)
  BY AGENT enricher;
-- the engine renders the grain-preserving SQL; grain verification is an observation

DECLARE KEY orders (order_id, line_no) CONFIRMED BY AGENT judge;

GLOSS revenue IN orders
  AS sum(amount)
  WHERE doc_type = 'invoice'
  BY AGENT grapher CONFIDENCE 0.9;
```

`GLOSS` — the namesake statement: the in-context explanation of a concept, how
it reads in this dataset. It is the successor of the snippet parts + provenance
contract: concept, relation, expression, filter — as grammar rather than JSON.
Columns-used, filter members, and rendered SQL are all derived from the
statement's AST by the engine; the statement is the single typed source. A
gloss's supersession key is *(concept, relation, parameter)*: a concept may hold
several glosses — per relation, per parameter — and re-glossing the same key
supersedes.

A human teach is not a separate mechanism: it is any of these statements with
`BY USER`. Precedence between actor classes is policy (§3.4), not syntax.

The language knows **one logical table name**. Layered materializations
(raw/typed/quarantine/enriched) are engine-internal; annotations attach to the
logical name, and layer resolution happens at the engine's SQL-emission boundary.
Enrichments are the one layered artifact with language-level identity, and they
get their own declared name (`orders_enriched` above).

### 3.3 Observation statements

```sql
OBSERVE profile, temporal, quality ON orders;
OBSERVE overlap ON (orders.customer_id, customers.id);
OBSERVE validation receivables_roll_forward;
```

An `OBSERVE` statement is the authored *request*; execution and storage are engine
concerns. Results land in the lake, keyed to the request. Two result channels:

- **Bulk results** → lake, referenced. Queryable via observation relations
  (`PROFILE(orders.amount)`, `OBSERVATIONS(subject)`).
- **Witnesses** → the log, as `WITNESS` statements emitted by detectors (§4):

```sql
WITNESS orders.amount behavior
  DISTRIBUTION (flow 0.83, stock 0.11, point_in_time 0.06)
  BY DETECTOR aggregation_lineage
  EVIDENCE 'obs://run-342/aggregation_lineage/orders.amount';
```

A witness is a claim distribution over a **closed claim space** plus an evidence
reference. Claim spaces are themselves declarations — §1.2(6), no aspect
vocabulary is fixed by the grammar:

```sql
DECLARE ASPECT behavior VALUES (flow, stock, point_in_time) BY SEED core;
DECLARE ASPECT null_token VALUES (is_null, is_value) BY SEED core;
```

**There are no fixed detectors — the witness statement is the detector
interface.** A detector is anything that produces attributed witnesses and their
lake evidence: an engine-registered measurement, an external worker, an LLM
agent. Engine-registered measurements are only the subset that `OBSERVE` can ask
the engine itself to run (`MEASUREMENTS`, §3.5); an external detector
orchestrates itself and appends witnesses directly. Two data-driven checks keep
the open door sound: a witness's distribution is validated at admission against
the declared aspect (the claim vocabulary lives in declarations, never in
compiled code), and a producer with no declared reliability pools at whatever
weight the reliability policy grants — by default, none. Writing to the log is
open; influence is earned through `DECLARE RELIABILITY` (§3.4).

Aspects arrive the way all vocabulary does — as declarations, normally replayed
from a vocabulary pack (§3.1). An aspect's declaration is its single home: one
label set per aspect, ending today's per-layer spellings of the same claim
space. Whether a small core of aspects is universal enough to standardize —
bound to data processing itself rather than to any domain — is held open (§8.1).

An aspect may take an argument where claims are per-instance rather than
per-subject (today: per null token, per candidate formula):

```sql
WITNESS orders.amount null_token 'n/a'
  DISTRIBUTION (is_null 0.91, is_value 0.09)
  BY DETECTOR null_semantics
  EVIDENCE 'obs://run-342/null_semantics/orders.amount';
```

The claim slot — the supersession key for witnesses — is *(subject, aspect,
argument)*. Witness reliability comes from `DECLARE RELIABILITY` (§3.4), not from
the witness itself.

`EVIDENCE` is the witness's citation, not its substance: an opaque ref resolved
at read time (`EXPLAIN`, drill-down), and optional — the engine keys its own
measurements' bulk results to the request; an external detector that omits the
ref simply cannot be drilled into. A dangling ref degrades drill-down, never
adjudication.

Observation batching, run identity, and result promotion are engine-internal: no
consumer addresses a run, and the log's timestamps and actors are what replay
needs. Opaque run tokens may appear inside `EVIDENCE` refs (as above) without
being addressable in the language.

### 3.4 Policy statements

```sql
DECLARE RELIABILITY DETECTOR aggregation_lineage FOR behavior 0.72
  BY CALIBRATION '2026-07';

DECLARE POLICY readiness
  BANDS (ready < 0.30, investigate < 0.70, blocked)
  WEIGHT behavior FOR aggregation_intent (conflict 0.8, ignorance 0.4)
  BY USER analyst;

DECLARE POLICY contract exploratory_analysis
  OVERALL THRESHOLD 0.6
  BLOCK ON (structural.parse_failure)
  BY SEED defaults;

DECLARE SERVING answer_agent
  PREFER enriched
  DIMENSION BUDGET 12
  RESTRICT JOINS TO DECLARED RELATIONSHIPS
  INCLUDE (conventions, drivers, grain_caveats)
  BY USER analyst;
```

Policies are declarations like any other — supersedable, attributed. Today's
hardcoded curation constants (the dimension budget, prefer-enriched, the join
whitelist rule) become declared serving policy. Actor-precedence (user over agent
over seed) is itself a policy default, overridable.

### 3.5 Consumption

Context is queryable as relations, composable with data in the same SQL session:

```sql
SELECT aspect, value, posterior, contested
FROM CONTEXT(orders.amount);

SELECT subject, aspect FROM DECLARATIONS WHERE contested;

SELECT target, band, top_driver FROM READINESS WHERE band <> 'ready';

SELECT id, version, aspects FROM MEASUREMENTS;  -- what the engine itself can run

SELECT month, value FROM METRIC dso BY month;   -- engine composes glosses + data

EXPLAIN orders.amount behavior;
-- declaration, witnesses, reliabilities, pooling trace, posterior — the why-audit

CONTEXT FOR (SELECT sum(amount) FROM orders GROUP BY channel)
  USING SERVING answer_agent;
-- the curated context document relevant to a query, rendered per serving policy

AT '2026-07-01' SELECT * FROM CONTEXT(orders.amount);   -- log replay, time travel
```

`CONTEXT FOR` replaces bespoke prompt assembly: the engine selects and renders the
declarations, posteriors, and caveats relevant to a query, bounded by a named
serving policy. One mechanism serves every agent; policies differ, code does not.

Rendering extensions (ggsql-style trailing `VISUALISE` clauses) are compatible with
this grammar and out of scope for v0.

### 3.6 Lifecycle

```sql
RETRACT ANNOTATE orders.amount UNIT BY USER analyst;   -- removes, no replacement
```

Supersession needs no statement (re-declare the same (subject, aspect)). There is
no UPDATE and no DELETE-of-history; the log is append-only.

---

## 4. Expressing measured evidence — decided

The atypical part of this language: formal grammars usually carry assertions, not
measurements. **Decided: witnesses live in the log, bulk evidence in the lake.**
The log carries exactly the evidence that participates in adjudication — claim
distributions over closed spaces, attributed to detectors, referencing their bulk
evidence. Everything else is columnar.

The witness layer is the load-bearing novelty of this language — it is what lets
a declaration be *contested* — and it is exactly log-shaped: small, per-subject,
attributed, supersedable. Bulk evidence is exactly lake-shaped. The `EVIDENCE ref`
clause is the join between the two worlds (today that join is implicit — inline
evidence keyed by target, run, and detector; the ref makes it explicit and typed),
and the reproducibility invariant holds: adjudication derives from the log alone;
drill-down derives from the lake.

Rejected: evidence fully outside the language (adjudication inputs become
invisible to replay — state stops being f(log, lake)); all evidence as statements
(the log stops being small and diffable; bulk numerics do not belong in text).

---

## 5. Adjudication semantics (derived plane)

For each (subject, aspect): the engine pools witnesses (weighted by declared
reliabilities) into a posterior over the claim space, compares it with the current
declaration, and exposes: `posterior`, `agreement`, `contested` (policy-thresholded),
and the trace (`EXPLAIN`). Readiness aggregates contested/ignorant aspects per
target under the readiness policy. Verdicts for validations apply declared tolerance
to observed deviation at read time.

Nothing in this plane is authored. In particular, resolution does **not** write back
into declarations (a change from today's resolve write-back): if an agent or user
accepts a posterior, that acceptance is a new `ANNOTATE ... BY ...` in the log —
authored, attributed, and auditable like everything else.

---

## 6. Reserved statement space (not yet covered, room held)

One line each; none designed in v0:

- **Synonyms** on any subject (`SYNONYMS ('turnover', ...)`) — we don't do this systematically yet.
- **Verified example queries** — question + query + verified-by; today only half-exists as saved snippets.
- **Agent instructions per subject** — prose guidance scoped to a table/metric (beyond global conventions).
- **Derivation declarations** — cleaning/union/dedup transformations beyond enrichment (the Databricks-LDP-shaped territory).
- **Expectations on incoming data** — schema stability, freshness SLAs, arrival contracts.
- **Unit conversion** — FX, unit algebra; today units are labels only.
- **Entity resolution** — same-entity assertions across sources.
- **Vocabulary sharing** — publishing/importing concept packs (verticals generalized beyond seed files).
- **Visualization clauses** — ggsql-compatible rendering tail.
- **Visibility/governance** — who may see which context.

## 7. Deliberately excluded

- **Orchestration and scheduling** — the language requests observations; it never sequences them.
- **Prompt configuration** — LLM prompts/versions are operational engine config, not context.
- **Storage layout** — log/lake encodings are implementation.
- **Interchange formats** — an Ossie mapping is possible for the vocabulary tier and is not part of the language.

---

## 8. Open questions for review

1. **Universal-core aspects** — no aspect vocabulary is fixed by the grammar and
   no detector is fixed by the language, ever. The only open question is whether
   a few aspects are so obviously bound to data processing itself — null
   semantics, type parsing — that a spec-blessed seed pack should standardize
   their label sets. Domain aspects (behavior, cycle stages) are pack vocabulary
   regardless.
2. **Aspect-driven annotation clauses** — §3.3 makes claim spaces declared, not
   grammar. Follow-on: do declared aspects also drive `ANNOTATE` clauses
   (`ANNOTATE orders.amount behavior flow`, with `behavior` resolved against
   declared aspects), or do core annotation clauses remain grammar keywords?
   §1.2(6) suggests the former; the cost is that `ANNOTATE` loses its fixed
   clause shape.
3. **Concept relations** — transcription evidence: part-of is authored as
   compositions (whole + parts), reconciles-with is an edge carrying a
   tolerance, disjointness is *derived* from convention concept groups (never
   authored), and the statement axis on metric extracts (`balance_sheet`,
   `income_statement`) is plausibly just part-of. Open: clauses on
   `DECLARE CONCEPT`, edge statements, or group declarations — and whether
   derived edges stay out of the log entirely (§2.5 suggests yes).
4. **Typing vocabulary** — type-pattern teaches (pattern → type +
   standardization expression), workspace-level null-token vocabulary, and
   expected-dependency assertions (intentional conditional nulls) have no
   statement family. Candidates: `DECLARE TYPE PATTERN`, pack-level null
   vocabulary, and a small expectation form — or §6 reserved space.
5. **Cycle vocabulary (concept space)** — cycle types (stages with indicators,
   completion indicators, aliases, feeds-into) live in pack YAML with no
   glossql home, and real cycle-family directions bind *concepts*
   (`incoming accounts_receivable`), not bare labels as §3.1 sketches.

## 9. Validation and first implementation slice

### 9.1 Transcription validation (before any implementation)

The grammar is validated against the running system by transcription, not by
implementation: take real artifacts from `dataraum-context` — §2's rows, as they
exist in config YAML, overlay payloads, and stored rows — and hand-write the
glossql statement for each. Every artifact lands in one of three buckets:
transcribes cleanly · exposes a grammar gap (the grammar gets fixed) · maps to a
mechanism the language deliberately drops. The companion check runs the reverse
direction: every current mechanism the language claims to subsume is listed and
confirmed droppable **without a workaround**. Implementation starts when §2's
rows are transcribed and the section-status flags are cleared.

### 9.2 First implementation slice (v0.1 PoC)

Scope for the first implementation, gated on §9.1 — the adjudication slice on
DataFusion, chosen because it exercises the two riskiest bets at once: the
DataFusion extension path and the witness/adjudication plane. Log and lake
encodings below are placeholders; the persistence decision (§1.1) stays open.

- **Statements:** `DECLARE CONCEPT`, `DECLARE ASPECT`, `ANNOTATE`, `OBSERVE`,
  `WITNESS`, `DECLARE RELIABILITY`, `DECLARE POLICY` (readiness bands only),
  `RETRACT`.
- **Consumption:** `CONTEXT(subject)`, `DECLARATIONS`, `EXPLAIN`, `AT`.
- **Substrate:** DataFusion custom statements; log = one plain-text statement
  file; lake = a parquet directory.
- **Loop closure:** one built-in detector (null semantics over `profile`
  results), so `OBSERVE → lake → WITNESS → pooling → CONTEXT` runs end to end.
- **Acceptance:** replay determinism (identical derived state from log + lake),
  contested detection under declared reliabilities, a faithful `EXPLAIN` trace,
  `AT` time travel by prefix replay.
- **Excluded:** `GLOSS`, metrics, validations, enrichments, hierarchies,
  serving policy, `CONTEXT FOR`.
