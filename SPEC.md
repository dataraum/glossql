# glossql — v0 draft specification

Status: **draft for review**. The name: a *gloss* is a marginal annotation
explaining a text's meaning; a glossary is a collection of them.
Scope of this document: the language only. One document; no satellite docs.

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

### 1.2 Design principles

1. **Four planes, one grammar.** Declarations, observations, policies, consumption
   share one statement skeleton and one provenance model.
2. **The concept/data split.** Vocabulary (concepts, metrics, validations,
   conventions) is written in *concept space* — dataset-independent, portable.
   Assertions about actual data (annotations, relationships, groundings) are written
   in *data space*. `GROUND` is the only bridge. This makes the analytical layer
   portable across datasets by construction.
3. **Judgment lives in policy, never in results.** Derived state carries numbers;
   bands, severities, and verdicts are policy applied at read time.
4. **Authored prose is opaque.** Meanings, conventions, guidance are string literals
   the engine transports but never parses. The grammar formalizes the envelope, not
   the prose.
5. **No surrogate identity in the language.** Subjects are structural paths
   (`orders.amount`), pairs (`orders.customer_id REFERENCES customers.id`), or
   declared names (`metric dso`). Cross-time identity is textual, by construction.

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
| workspace calendar / vertical binding | fiscal year start; active vertical | `DECLARE CALENDAR`, `USE VERTICAL` |
| column annotations (LLM) | role, business name/description, behavior claim, null tokens | `ANNOTATE <column>` |
| column concepts (LLM) | meaning (prose), temporal behavior, unit source, derived-formula hypothesis | `ANNOTATE <column>` |
| table entities (LLM) | entity type, role (fact/dimension/snapshot), grain, time axes, identity columns | `ANNOTATE <table>` |
| relationships — confirmation half | type, cardinality, confirmed-by | `DECLARE RELATIONSHIP` |
| hierarchies — judged half | drilldown/alias/role kinds, levels | `DECLARE HIERARCHY` |
| slice definitions — ranked half | priority, business context | `DECLARE DIMENSION` |
| enrichment selection | which neighbours enrich a fact table, exposed columns | `DECLARE ENRICHMENT` |
| business cycles (LLM) | cycle assertion, stages, status column, completion semantics | `DECLARE CYCLE` |
| surrogate key confirmation | composite-key intent confirmed/declined | `DECLARE KEY` |
| groundings (snippet parts + provenance basis) | concept → relation, expression, filters | `GROUND` |
| teach payloads (all 8 types, today free JSON) | type patterns, null tokens, units, plus the families above | the same statements, `BY user` |
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
grounding SQL. All queryable (§3.5); none writable.

---

## 3. The grammar

Notation: lowercase = nonterminal, `UPPER` = keyword, `[...]` optional, `{...}` repeated,
`|` alternatives. Sketch-level: statement forms are normative, clause details are
illustrative pending review.

### 3.0 Shared skeleton

```
statement   := declaration | observation | witness | policy | consumption | lifecycle
declaration := DECLARE family subject clauses provenance ';'
provenance  := BY actor [ CONFIDENCE number ] [ EVIDENCE ref ]
actor       := USER name | AGENT name | DETECTOR name | SEED name | CALIBRATION name
subject     := table | table '.' column | pair | declared_name
pair        := table '.' column REFERENCES table '.' column
```

- Every authored statement carries `BY`. The actor classes generalize today's
  `source`/`confirmation_source`/`detection_source` vocabularies into one clause.
- **Supersession:** a declaration's natural key is *(subject, aspect)*. Re-declaring
  the same key supersedes; history remains in the log. `RETRACT` removes without
  replacement. No in-place mutation exists.
- Prose payloads are single-quoted string literals, opaque to the engine.

### 3.1 Vocabulary statements (concept space)

```sql
DECLARE CONCEPT revenue
  KIND measure
  DESCRIPTION 'income from sales of goods and services'
  INDICATORS ('revenue', 'sales', 'turnover')
  BY SEED finance;

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
  ON CYCLE order_to_cash
  CHECK opening(receivables) + sum(revenue) - sum(collections)
        RECONCILES WITH closing(receivables)
  TOLERANCE 0.01
  SEVERITY error
  GUIDANCE 'a gap usually indicates unposted collections or write-offs'
  BY SEED finance;

DECLARE CYCLE FAMILY conversion DIRECTIONS (forward, reverse) BY SEED finance;
DECLARE CALENDAR FISCAL YEAR STARTS april BY USER analyst;
USE VERTICAL finance BY USER analyst;
```

Rule: inside `DECLARE METRIC` / `DECLARE VALIDATION` expressions, bare identifiers
denote **concepts**, never columns. The expressions are portable; only `GROUND`
binds them to data.

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

DECLARE HIERARCHY geo IN customers
  LEVELS (country > region > city)
  KIND drilldown
  BY AGENT judge;

DECLARE DIMENSION orders.channel
  PRIORITY 0.8
  CONTEXT 'primary go-to-market split'
  BY AGENT slicer;

DECLARE ENRICHMENT orders_enriched FROM orders
  JOIN customers VIA (orders.customer_id REFERENCES customers.id)
  EXPOSE (customers.region, customers.segment)
  BY AGENT enricher;
-- the engine renders the grain-preserving SQL; grain verification is an observation

DECLARE KEY orders (order_id, line_no) CONFIRMED BY AGENT judge;

GROUND revenue IN orders
  AS sum(amount)
  WHERE doc_type = 'invoice'
  BY AGENT grapher CONFIDENCE 0.9;
```

`GROUND` is the successor of the snippet parts + provenance contract: concept,
relation, expression, filter — as grammar rather than JSON. Columns-used, filter
members, and rendered SQL are all derived from the statement's AST by the engine;
the statement is the single typed source.

A human teach is not a separate mechanism: it is any of these statements with
`BY USER`. Precedence between actor classes is policy (§3.4), not syntax.

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
WITNESS orders.amount BEHAVIOR
  DISTRIBUTION (flow 0.83, stock 0.11, point_in_time 0.06)
  BY DETECTOR aggregation_lineage
  EVIDENCE 'obs://run-342/aggregation_lineage/orders.amount';
```

A witness is a claim distribution over a **closed claim space** plus an evidence
reference. Claim spaces are declared per aspect (the enum homes of today, in the
language). Witness reliability comes from `DECLARE RELIABILITY` (§3.4), not from
the witness itself.

### 3.4 Policy statements

```sql
DECLARE RELIABILITY DETECTOR aggregation_lineage FOR BEHAVIOR 0.72
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

SELECT month, value FROM METRIC dso BY month;   -- engine composes grounding + data

EXPLAIN orders.amount BEHAVIOR;
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

## 4. Expressing measured evidence — the decision

The atypical part of this language: formal grammars usually carry assertions, not
measurements. Three options considered:

- **A — evidence outside the language.** Only `OBSERVE` requests and policies are
  statements; all results live in the lake, queryable but never in the log.
  Cheapest; but adjudication inputs (witnesses) become invisible to replay — state
  stops being f(log, lake) unless the lake carries witness semantics too.
- **B — all evidence as statements.** Every profile and distribution inlined into
  the log. Uniform, but the log stops being small and diffable; bulk numerics do
  not belong in text.
- **C — witnesses in the log, bulk evidence in the lake (recommended).** The log
  carries exactly the evidence that participates in adjudication: claim
  distributions over closed spaces, attributed to detectors, referencing their bulk
  evidence. Everything else is columnar.

**Recommendation: C.** The witness layer is the load-bearing novelty of this
language — it is what lets a declaration be *contested* — and it is exactly
log-shaped: small, per-subject, attributed, supersedable. Bulk evidence is exactly
lake-shaped. The `EVIDENCE ref` clause is the join between the two worlds, and the
reproducibility invariant holds: adjudication derives from the log alone; drill-down
derives from the lake.

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

1. **Evidence model** — confirm recommendation C (§4).
2. **Resolution as re-declaration** — confirm dropping write-back in favor of purely derived adjudication (§5).
3. **Runs** — are observation batches first-class in the language (`OBSERVE ... AS RUN x`) or engine-internal grouping? Proposal: engine-internal; the log has timestamps and actors, which is what replay needs.
4. **Layer naming** — today tables exist at raw/typed/quarantine/enriched layers. Proposal: the language knows one logical table name; layers are engine-internal materializations; annotations attach to the logical name. Enrichments get their own declared names.
5. **Metric expression rule** — bare identifiers = concepts inside metric/validation bodies: acceptable, or explicit marker preferred?
6. **Claim spaces** — declared in-language (`DECLARE ASPECT behavior VALUES (flow, stock, point_in_time)`) or fixed by the spec per aspect? Proposal: core aspects fixed by the spec, extensions declarable.
