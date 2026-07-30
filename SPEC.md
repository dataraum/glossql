# glossql — v0 draft specification

Status: **draft for review**. The name: a *gloss* is a marginal annotation
explaining a text's meaning; a glossary is a collection of them. In the
language the word has one role: `GLOSS` is the reading statement — it asks for
the glossed reading of a subject or a query. What is authored is a
`GROUNDING` (§3.2): the grounding is what you write, the gloss is what you get.
Scope of this document: the language only. One document; no satellite docs.

Status by section — the iteration loop works the flagged items (§9.1):

- **Ready** (reads unambiguously for practitioners and agents): §1 · §3.0 ·
  §3.6 · §4 · §5 · §7 · §10.
- **Under iteration** (statement forms hold; clause details do not):
  - §3.1 — the `PARAMETER` clause shape (type, options, grain derivation) is a
    sketch; the `RECONCILES WITH` right-hand side is open (§8.2).
  - §3.0 — the semantic admission checklist is unwritten; the log envelope is
    deliberately last (§1.1).
  - §3.2 — typing/null/expectation teaches (§8.3) have no statement form yet;
    the `DECLARE VIEW` admission contract (join matching) and the recipe
    clause (`CONNECTION`/`VIA` shape, dialect boundary) are sketches; the
    rest transcribes cleanly.
  - §3.3 — the core aspect library and its label sets are open (§8.1); the
    aspect model itself (declared spaces, function-shaped application) is
    settled.
  - §3.4 — `WEIGHT` semantics, the contract policy, the interpretation policy
    key, and the serving clause list are sketches.
  - §3.5 — relation schemas for `CONTEXT()` / `READINESS` / `WHY()` /
    `GROUNDINGS()`, the `METRIC()` grain argument, the `GLOSS` document
    shape, and the `AS PACK` closure scope are unspecified; `AT` syntax
    alignment with lake time travel is open.

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
at any historical point. Replay re-executes nothing at the lake boundary:
acquisitions (recipes) and observations re-bind to results the lake already
holds.

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
  writing statements). Direction for the envelope, noted not designed: statement
  identity by content hash (today's `recipe_hash` generalized — re-admitting an
  identical statement is a no-op supersede) and a hash-chained log for tamper
  evidence (the git property). Both are audit properties of a single-writer log.
  Consensus, anchoring, and distributed trust are explicitly out of scope;
  tamper evidence is not governance (held open above).

### 1.2 Design principles

1. **Four planes, one grammar.** Declarations, observations, policies, consumption
   share one statement skeleton and one provenance model.
2. **The concept/data split.** Vocabulary (concepts, metrics, validations,
   conventions) is written in *concept space* — dataset-independent, portable.
   Assertions about actual data (aspect applications, relationships,
   groundings) are written in *data space*. The grounding statement is the
   only bridge (§3.2). This makes the analytical layer portable across
   datasets by construction.
3. **Judgment lives in policy, never in results.** Derived state carries numbers;
   bands, severities, and verdicts are policy applied at read time.
4. **Authored prose is opaque.** Meanings, conventions, guidance are string literals
   the engine transports but never parses. The grammar formalizes the envelope, not
   the prose.
5. **No surrogate identity in the language.** Subjects are structural paths
   (`orders.amount`), pairs (`orders.customer_id REFERENCES customers.id`), or
   declared names (`metric dso`). Cross-time identity is textual, by construction.
6. **Mechanism in grammar, vocabulary in declarations.** The grammar never
   enumerates domain specifics (the SQL-inventor test: SQL has no INVOICE
   statement; domain lives in tables): claim spaces, concept vocabularies, and
   their groupings are declarations — importable, supersedable — never keywords. The
   same holds for detectors: the grammar knows the actor class, never a roster.
7. **Ride SQL.** glossql adds statements only where SQL has no construct: the
   authored context. Querying, data CRUD, and functions stay plain SQL against
   the attached lake (the DuckLake posture: attach, then little syntax);
   context reading is one statement (`GLOSS`) over derived relations, not a
   verb set. The writing plane is the language.

---

## 2. What the current system says — the map

Grouped by the four verbs. Each entry names the construct (§3) that expresses it.
This map is the completeness check for the grammar: every row is either covered,
reserved (§6), or deliberately excluded (§7).

### 2.1 We DECLARE (asserted, carries provenance)

| Today | Content | glossql construct |
|---|---|---|
| concepts / concept edges | vocabulary: name, description, indicators, kind, relations | `DECLARE CONCEPT`; edges `DECLARE RELATIONSHIP` (§3.1) |
| conventions | opaque prose rules served verbatim | `DECLARE CONVENTION` |
| metrics + parameters + dependency DAG | expression over concepts, unit, output, parameters, interpretation ranges | `DECLARE METRIC`; interpretation is policy (§3.4) |
| validations | check, tolerance, severity, guidance, cycle scope | `DECLARE VALIDATION` |
| cycle families | closed families + directions | `DECLARE CYCLE FAMILY` |
| workspace calendar / vertical binding | fiscal year start; active vertical | `DECLARE calendar(workspace, …)`; vertical binding has no construct — importing a pack is replaying its statements (§3.1) |
| column annotations (LLM) | role, business name/description, behavior claim, null tokens | `DECLARE <aspect>(column)` |
| column concepts (LLM) | meaning (prose), temporal behavior, unit source, derived-formula hypothesis | `DECLARE <aspect>(column)` |
| table entities (LLM) | entity type, role (fact/dimension/snapshot), grain, time axes, identity columns | `DECLARE <aspect>(table)` |
| relationships — confirmation half | type, cardinality, confirmed-by | `DECLARE RELATIONSHIP` |
| hierarchies — judged half | drilldown/alias/role kinds, levels | `DECLARE HIERARCHY` |
| slice definitions — ranked half | priority, business context | `DECLARE dimension(…)` — an aspect |
| enrichment selection | which neighbours enrich a fact table, exposed columns | `DECLARE VIEW` |
| business cycles (LLM) | cycle assertion, stages, status column, completion semantics | decomposed: stage concepts + `PART OF`, ordered stage aspect with `TERMINAL`, per-value applications (§3.3) |
| surrogate key confirmation | composite-key intent confirmed/declined | `DECLARE key(table, …)` — an aspect |
| groundings (snippet parts + provenance basis) | concept → relation, expression, filters | `DECLARE GROUNDING` |
| teach payloads (all 8 types, today free JSON) | type patterns, null tokens, units, plus the families above | the same statements, `BY user` — type/null/expectation teaches pending §8.3 |
| sources / tables | where data lives | `DECLARE SOURCE`, `DECLARE TABLE` |
| db recipes (verbatim query per connection) | source name, credential reference, backend, query | `DECLARE SOURCE … CONNECTION` + `DECLARE TABLE … AS` |

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
| six-block answer-agent context (schema+meanings, entities, curated dimensions, relationship whitelist, drivers, grain caveats) + conventions + snippet vocabulary | `GLOSS (query)`, shaped by `DECLARE SERVING` policy |
| engine GraphAgent served context | same `GLOSS`, different serving policy — one mechanism, two policies |
| readiness surfaces (bands, drivers, coverage) | `READINESS()` relation |
| why-tools (adjudication audit) | `WHY(subject, aspect)` |
| look-tools (values, profiles, metrics, validations) | context relations and `GLOSS <subject>` (§3.5) |
| property-graph projections (`og_*`) | engine-internal; traversal served via context relations, no PGQ dependency |
| validation verdicts (computed on demand) | derived: `deviation <= tolerance` at read, tolerance from the declaration |

### 2.5 Who writes, who reads

The grammar's primary author is not human. In steady state:

| statements | produced by | at | consumed by |
|---|---|---|---|
| `DECLARE SOURCE` / `DECLARE TABLE` | onboarding code | connection time | materialization, everything downstream |
| concept-space `DECLARE` (concept, aspect, convention, metric, validation, cycle family) | pack replay (`SEED`); inductor agents; users | import · post-catalog induction · teach | catalog steering, execution, serving |
| aspect applications (`DECLARE <aspect>(…)`), `DECLARE RELATIONSHIP/HIERARCHY/VIEW` | cataloguer/judge/slicer/enricher agents; users | pipeline phases · teach moments | serving, adjudication, view rendering |
| `DECLARE GROUNDING` | grapher agent; rarely users | lazily, at first metric execution needing (concept, relation) | metric composition, drill-down, serving |
| `OBSERVE` | orchestration code | phase boundaries, re-runs | engine execution → lake |
| `WITNESS` | detectors; external producers | measurement completion | pooling |
| `DECLARE RELIABILITY` | calibration job | calibration release | pooling weights |
| `DECLARE POLICY/SERVING`, `RETRACT` | users, ops | setup · correction | read-time judgment, rendering |

Reading has two consumer kinds: answer agents (curated context, then plain SQL
over data; they write nothing — their teach suggestions surface to users, who
write) and audit/UI (why-traces, readiness, history). Humans author corrections
and policy; mostly they *review* the log. The text form exists for diffability
and audit, not authoring ergonomics — statements are written by agents and code,
so shapes must be regular enough for constrained decoding.

### 2.6 Derived (computed, never authored, never in the log)

Pooled posteriors and conflict flags · readiness bands · validation verdicts ·
additivity conclusions · graph projections · rendered view SQL · rendered
grounding SQL. All queryable (§3.5); none writable.

---

## 3. The grammar

Notation: lowercase = nonterminal, `UPPER` = keyword, `[...]` optional, `{...}` repeated,
`|` alternatives. Sketch-level: statement forms are normative, clause details are
illustrative pending review. Token-level grammar (identifier quoting, string
literals, comments, keyword case) is inherited from the engine substrate's SQL
dialect — DataFusion's PostgreSQL-style parser. glossql adds statement forms,
not a lexer. One token rule is glossql's own: clause-head keywords are
reserved inside glossql statement bodies — an identifier that collides is
double-quoted, as in SQL; transported SQL strings are unaffected.

### 3.0 Shared skeleton

```
statement   := writing | reading
writing     := declaration | witness | observation | lifecycle
reading     := [ AT timestamp ] ( gloss | sql_query )       -- §3.5
gloss       := GLOSS ( subject | '(' query ')' ) [ USING SERVING name ] [ AS PACK ]
declaration := DECLARE class name clauses provenance ';'
             | DECLARE aspect '(' subject { ',' arg ':=' value } ')' provenance ';'
witness     := WITNESS aspect '(' subject { ',' arg ':=' value } ')' provenance ';'
provenance  := BY actor [ CONFIDENCE number ] [ EVIDENCE ref ]
actor       := USER name | AGENT name | DETECTOR name [ WITNESS name ]
             | SEED name | CALIBRATION name
subject     := workspace | table | table '.' column | pair | declared_name
pair        := table '.' column REFERENCES table '.' column
             | table '(' column {',' column} ')' REFERENCES table '(' column {',' column} ')'
```

- **One verb per replay semantics.** Four verbs write: `DECLARE` asserts — the
  latest statement per claim slot wins; `WITNESS` evidences — statements pool
  under declared reliabilities and never occupy the slot they inform;
  `OBSERVE` requests — every occurrence fires, nothing supersedes; `RETRACT`
  removes — the slot is vacated. Merging any two would force one replay rule
  to serve two speech acts. `GLOSS` is the only reading statement (§3.5). No
  word holds two roles: verbs are speech acts — four writing, one reading —
  and every other keyword is a noun (a class after `DECLARE`) or a clause.
- Only **writing** statements enter the log; **reading** statements (§3.5) are
  session-ephemeral — never logged, never part of replay.
- **A declared name is a name.** `DECLARE <CLASS> <name>` follows SQL's
  `CREATE <CLASS> <name>` convention: the token after the class is the name,
  usable in any later statement. References resolve against the declaring
  namespace at admission — never joined by naming convention, as today's
  `standard_field` strings are.
- **The aspect-application form declares no name.** `DECLARE
  behavior(orders.amount, value := flow)` occupies a claim slot *(subject,
  aspect[, argument])*: the class form defines names, the application form
  asserts about subjects. Both are declarations — the assertions adjudication
  compares witnesses against (§5).
- **Three declaration shapes.** *Named* classes define names (`SOURCE`,
  `TABLE`, `VIEW`, `CONCEPT`, `CONVENTION`, `METRIC`, `VALIDATION`,
  `CYCLE FAMILY`, `ASPECT`, `HIERARCHY`, `SERVING`); *keyed* classes carry no
  name — their identity is their key (`RELATIONSHIP` by its edge, `GROUNDING`
  by concept — one active grounding per concept, `RELIABILITY` by (detector, witness,
  aspect) — a detector pools one or more named witnesses, each with its own
  declared reliability, and bare `DETECTOR x` is shorthand for `DETECTOR x
  WITNESS x`); *aspect applications* are keyed by their claim slot. Policy keys are
  mechanism-defined (a readiness singleton, a named contract, an
  interpretation keyed `FOR` a metric). A proposed class that would carry no
  name and no principled key is an aspect wearing a costume.
- Every authored statement carries `BY`. The actor classes generalize today's
  `source`/`confirmation_source`/`detection_source` vocabularies into one clause.
- `CONFIDENCE` is logged provenance metadata in [0, 1] — never an adjudication
  input. The pooling plane ignores it; serving decides whether and how consumers
  see it (prompts govern what an LLM must do with it, as today).
- **Supersession:** a declaration's natural key is *(subject, aspect[, argument])*. Re-declaring
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
-- dimension concepts may declare ORDERING (ordered | nominal)

DECLARE RELATIONSHIP net_revenue PART OF revenue BY SEED finance;
DECLARE RELATIONSHIP receivables RECONCILES WITH (revenue - collections)
  TOLERANCE 0.01 BY AGENT inductor;           -- right-hand-side shape: §8.2
DECLARE RELATIONSHIP DISJOINT (product_revenue, service_revenue, other_revenue)
  BY SEED finance;

DECLARE CONVENTION accrual_basis
  STATEMENT 'amounts are recognized when earned, not when paid'
  BY USER analyst;

DECLARE METRIC dso
  AS 90 * avg(receivables) / sum(revenue)      -- identifiers denote concepts here
  UNIT 'days'
  PARAMETER period GRAIN month DEFAULT last_complete
  BY SEED finance;
-- interpretation bands are read-time policy (§3.4), never metric declaration

DECLARE VALIDATION receivables_roll_forward
  KIND balance                      -- balance | comparison | constraint | aggregate
  ON CYCLE order_to_cash
  OVER (receivables, revenue, collections)
  TOLERANCE 0.01
  SEVERITY error
  GUIDANCE 'opening receivables plus revenue minus collections reconciles with
            closing receivables; a gap usually indicates unposted collections'
  BY AGENT inductor;

DECLARE CYCLE FAMILY settlement
  DIRECTIONS (incoming accounts_receivable, outgoing accounts_payable)
  BY SEED finance;
-- directions bind concepts, never bare labels
DECLARE calendar(workspace, fiscal_year_starts := april) BY USER analyst;
-- workspace is a subject: the home of workspace-scoped facts (calendar now;
-- §8.3's null-token vocabulary when it lands)
```

A context's **glossary** is implicit: it is the set of concept-space declarations
currently active in the log — what is defined. There is no pack construct and no
binding statement (`USE VERTICAL` does not survive). A vocabulary pack is a file
of statements; importing it is replaying it, and provenance (`BY SEED finance`)
records where each declaration came from. Collisions between packs resolve by
supersession like everything else. Exporting is the same mechanism in reverse:
a pack is a saved gloss (`GLOSS … AS PACK`, §3.5). Distribution stays reserved
(§6).

`DECLARE RELATIONSHIP` is the single head for authored edges in both spaces:
`REFERENCES` operates on column pairs (§3.2); `PART OF`, `RECONCILES WITH`,
and `DISJOINT` operate on concepts. An edge is its own statement — its own
supersession slot, provenance, and retraction — never a clause on an
endpoint's declaration (tolerance is edge payload; disjointness is n-ary over
a group, not a property of one concept). The operator set is closed and
mechanism-backed — §1.2(6) applied to edges: `PART OF` feeds the serving
spine's closure traversal (§3.5), `RECONCILES WITH` carries the tolerance
balance checks derive from, `DISJOINT` feeds admission-time contradiction
checks. Domain edges (feeds-into, stage order) stay pack vocabulary: no
operator without a named mechanism. Admission rejects cross-space edges — a
`PART OF` between a concept and a column is an error — so
grounding-as-only-bridge is enforced, not just stated.

Rule: inside `DECLARE METRIC` / `DECLARE VALIDATION` expressions, bare identifiers
resolve against a single concept-space namespace: concepts, declared metrics, and
parameters. A name that would denote more than one of these is a declaration-time
error. Columns are unreachable here by construction — the grounding is the
only bridge (§3.2) — so the expressions stay portable. Metric composition is plain reference:
`operating_income` in another metric's body denotes the declared metric.

Validations carry **no formal check expression** — principle 4 taken fully. The
authored surface is the typed envelope (kind, cycle scope, tolerance, severity)
plus opaque guidance prose; the executed SQL is derived at run time and auditable
via `WHY()`. `OVER` is the membership contract: every name it lists must
resolve in the glossary, so a fabricated reference fails at declaration time.
Validation *induction* is not a language mechanism — it is an agent authoring
these statements (`BY AGENT inductor`), grounded in served context.

### 3.2 Data statements (data space)

```sql
DECLARE SOURCE erp_export FROM 'lake/erp/*.parquet' BY USER analyst;
DECLARE TABLE orders FROM erp_export BY USER analyst;

DECLARE SOURCE crm CONNECTION postgres VIA 'crm_prod' BY USER analyst;
-- VIA references engine-configured credentials; secrets never enter the log

DECLARE TABLE active_customers FROM crm
  AS 'SELECT id, name, segment FROM customers WHERE status = ''active'''
  BY USER analyst;
-- a recipe: verbatim SQL in the source's dialect, executed at the origin,
-- its result materialized into the lake. A transported payload (principle 4),
-- not glossql SQL — parsed bodies are bare, transported bodies are strings.
-- Cross-source SQL decomposes: one recipe table per source, a view to join

DECLARE entity(orders, value := 'sales order') BY AGENT cataloguer CONFIDENCE 0.9;
DECLARE role(orders, value := fact) BY AGENT cataloguer CONFIDENCE 0.9;
DECLARE grain(orders, columns := (order_id, line_no)) BY AGENT cataloguer;
DECLARE time_axis(orders, column := order_date, anchor := true) BY AGENT cataloguer;

DECLARE meaning(orders.amount, value := 'gross invoiced amount per order line')
  BY AGENT cataloguer CONFIDENCE 0.92;
DECLARE unit(orders.amount, value := 'EUR') BY AGENT cataloguer CONFIDENCE 0.92;
DECLARE behavior(orders.amount, value := flow) BY AGENT cataloguer CONFIDENCE 0.92;
DECLARE null_token(orders.amount, token := 'n/a', value := is_null) BY USER analyst;
-- one aspect application per statement: exactly one claim slot
-- (subject, aspect, argument), the supersession unit. An aspect's payload is
-- typed by its declaration: a closed label set (VALUES), prose, or structure.

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

DECLARE dimension(orders.channel, priority := 0.8,
  context := 'primary go-to-market split') BY AGENT slicer;

DECLARE dimension(orders, via := customer_id, to := customers.segment,
  priority := 0.7) BY AGENT slicer;
-- a reachable attribute as a slice: the path rides the argument surface
-- (replacing a bespoke VIA … TO clause form); the arguments are the claim slot

DECLARE VIEW orders_enriched AS
  SELECT o.order_id, o.line_no, o.amount, c.region, c.segment
  FROM orders o JOIN customers c ON o.customer_id = c.id
  BY AGENT enricher;
-- SQL-bodied derivation: enrichment, cleaning, dedup, typing transforms share
-- this head. The select list is the exposure; joins-used derive from the AST;
-- every join equality must match a declared relationship at admission (the
-- OVER membership contract, applied to data space); grain preservation is
-- verified by observation, not by construction

DECLARE key(orders, columns := (order_id, line_no), value := confirmed)
  BY AGENT judge;
-- composite-key intent is an aspect (VALUES (confirmed, declined)), not a class

DECLARE GROUNDING revenue IN orders
  AS amount
  WHERE doc_type = 'invoice'
  BY AGENT grapher CONFIDENCE 0.9;
-- row-level: the metric aggregates (sum(revenue)); the grounding only reads
```

`DECLARE` creates nothing. SQL's `CREATE TABLE` makes an object; `DECLARE
TABLE` makes an assertion — the files already exist in the lake, and the
statement records, attributed, that they carry this name here. The binding
must be log content, not engine configuration: subjects resolve against it at
admission, and state = f(log, lake) fails if a third input holds the names.
The engine's material work (scanning, typing, layering, perhaps an internal
`CREATE EXTERNAL TABLE`) is the *effect* of replaying the declaration, never
the record of it. The same reading applies to `DECLARE VIEW` against `CREATE
VIEW`: same body, plus attribution, supersession, and the join-admission
check.

`GROUNDING` — the bridge class: how a concept reads in this dataset, the one
statement family that names both a concept and columns. The grounding is what
is authored; the gloss is what is rendered (§3.5) — the read verb assembles
groundings with everything else known about a subject. It is the successor of
the snippet parts + provenance
contract: concept, relation, expression, filter — as grammar rather than JSON.
Columns-used, filter members, and rendered SQL are all derived from the
statement's AST by the engine; the statement is the single typed source. A
grounding's supersession key is the *concept*: one active grounding per
concept, and re-grounding supersedes. A grounding is a **row-level reading** —
relation, expression, filter; aggregation belongs to the metric expression,
never the grounding body. A differently-filtered or differently-axised reading
is its own declared concept: the statement axis (`balance_sheet`,
`income_statement`) is `PART OF` structure, and a filtered variant
(`reconciled_count`) earns a name rather than riding a filter. Admission
rejects byte-identical grounding bodies for concepts related by `DISJOINT`.

A human teach is not a separate mechanism: it is any of these statements with
`BY USER`. Precedence between actor classes is policy (§3.4), not syntax.

The language knows **one logical table name**. Layered materializations
(raw/typed/quarantine/enriched) are engine-internal; aspect applications attach
to the logical name, and layer resolution happens at the engine's SQL-emission
boundary. Views are the derived relations with language-level identity — each
gets its own declared name (`orders_enriched` above); whether a view is
materialized is engine-internal.

### 3.3 Observation statements

```sql
OBSERVE profile, temporal, quality ON orders BY AGENT onboarding;
OBSERVE overlap ON (orders.customer_id, customers.id) BY AGENT relationships;
OBSERVE validation receivables_roll_forward BY AGENT scheduler;
```

An `OBSERVE` statement is the authored *request*; execution and storage are engine
concerns. Like every writing statement it carries `BY` — orchestration code
writes as an `AGENT` actor named for its workflow. Results land in the lake, keyed to the request. `OBSERVE` returns no
rows, and replay re-runs nothing: requests re-bind to results already in the
lake, which is what keeps replay deterministic. Two result channels:

- **Bulk results** → lake, referenced. Queryable via observation relations
  (`PROFILE(orders.amount)`, `OBSERVATIONS(subject)`).
- **Witnesses** → the log, as `WITNESS` statements emitted by detectors (§4):

```sql
WITNESS behavior(orders.amount, flow := 0.83, stock := 0.11, point_in_time := 0.06)
  BY DETECTOR aggregation_lineage
  EVIDENCE 'obs://run-342/aggregation_lineage/orders.amount';
```

A witness is a claim distribution over a **closed claim space** plus an evidence
reference. Claim spaces are themselves declarations — §1.2(6), no aspect
vocabulary is fixed by the grammar:

```sql
DECLARE ASPECT behavior VALUES (flow, stock, point_in_time) BY SEED core;
DECLARE ASPECT null_token ARGUMENTS (token) VALUES (is_null, is_value) BY SEED core;
DECLARE ASPECT ar_stage VALUES (created < sent < due < paid) TERMINAL (paid)
  BY SEED finance;
```

`ARGUMENTS` declares the per-instance argument names; `VALUES` may declare a
total order (`<`) — ordered label sets are generic to data processing (stages,
severity ladders, bands) and back progression checks and ordered rendering;
`TERMINAL` marks absorbing labels, and completion semantics — with the
validations that check them — derive from it. Cycle types need no statement of
their own: stage concepts in a pack, `PART OF` edges to the cycle concept, an
ordered stage aspect binding the status column's values per instance
(`DECLARE ar_stage(invoices.status, token := 'delivered', value := sent) BY
AGENT cataloguer`), `TERMINAL` for completion. `feeds_into` and stage
indicator prose stay pack description — a deliberate drop.

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

An aspect may take arguments where claims are per-instance rather than
per-subject (today: per null token, per candidate formula). Arguments and
distribution labels share the call's named-argument surface; admission tells
them apart by the aspect's declaration (names in `ARGUMENTS` vs labels in
`VALUES`):

```sql
WITNESS null_token(orders.amount, token := 'n/a', is_null := 0.91, is_value := 0.09)
  BY DETECTOR null_semantics
  EVIDENCE 'obs://run-342/null_semantics/orders.amount';
```

The claim slot — the supersession key for witnesses — is *(subject, aspect,
argument)*. Witness reliability comes from `DECLARE RELIABILITY` (§3.4), not from
the witness itself.

`EVIDENCE` is the witness's citation, not its substance: an opaque ref resolved
at read time (`WHY()`, drill-down), and optional — the engine keys its own
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
DECLARE RELIABILITY DETECTOR null_semantics WITNESS null_vocabulary
  FOR null_token 0.944 BY CALIBRATION '2026-07';
-- reliability is per witness within a detector; a bare DETECTOR name is the
-- single-witness shorthand (DETECTOR x ≡ DETECTOR x WITNESS x)

DECLARE POLICY readiness
  BANDS (ready < 0.30, investigate < 0.70, blocked)
  WEIGHT behavior FOR aggregation_intent (conflict 0.8, ignorance 0.4)
  BY USER analyst;

DECLARE POLICY interpretation FOR dso
  BANDS (ok < 45, warn < 75, critical)
  BY SEED finance;

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
over seed) is itself a policy default, overridable. Metric interpretation is
the same move applied to `DECLARE METRIC`: bands over metric values are
read-time judgment (principle 3), keyed to the metric name under the same
`BANDS` mechanism as readiness.

### 3.5 Consumption

Consumption adds almost no grammar — principle 7. Context relations are table
functions taking **subject strings**, so every one of them is standard SQL, and
they compose with data queries in the same session. The grammar additions are
exactly two: `GLOSS` — the reading statement, rendering the context document
for a subject or a query — and the `AT` pin (time travel over the whole
context, not one table):

```sql
SELECT aspect, value, posterior, contested
FROM CONTEXT('orders.amount');

SELECT subject, aspect FROM DECLARATIONS WHERE contested;

SELECT target, band, top_driver FROM READINESS WHERE band <> 'ready';

SELECT id, version, aspects FROM MEASUREMENTS;  -- what the engine itself can run

SELECT month, value FROM METRIC('dso', grain => 'month');  -- groundings + data, composed

SELECT * FROM WHY('orders.amount', 'behavior');
-- declaration, witnesses, reliabilities, pooling trace, posterior — the
-- why-audit as a relation (today's why-tools; EXPLAIN stays SQL's keyword)

SELECT concept, relation, sql FROM GROUNDINGS('revenue');
-- the grounding library searched by concept — KB-first composition starts here

GLOSS orders.amount;        -- subject document: aspects, posteriors, contested
GLOSS orders;               -- table document: entity header, per-column bands
GLOSS revenue;              -- concept document: its groundings and neighbourhood

GLOSS (SELECT sum(amount) FROM orders GROUP BY channel)
  USING SERVING answer_agent;
-- query-anchored: the curated context document relevant to a query,
-- rendered per serving policy

GLOSS revenue AS PACK;
-- statements form: the active authored statements in revenue's closure,
-- replayable as-is — a pack is a saved gloss

AT '2026-07-01' SELECT * FROM CONTEXT('orders.amount');  -- log replay, time travel
-- context-wide pin: state = f(log ≤ t, lake ≤ t); syntax alignment with lake
-- time travel (AT (TIMESTAMP => ...)) is a flag item
```

`GLOSS` replaces bespoke prompt assembly: the engine selects and renders the
declarations, posteriors, and caveats relevant to a subject or a query, bounded
by a named serving policy. Row-shaped access stays in the relations above;
`GLOSS` renders the document. `USING SERVING` is its only shaping knob —
document shape is policy, never per-call arguments. Fieldwork: the running
system serves a byte-identical context block per session so it rides as a
cached prompt prefix; per-call include/exclude would break that and open a
second shaping surface beside `DECLARE SERVING`. One mechanism serves every
agent; policies differ, code does not — and calls do not either.

`AS PACK` is the closure move. SQL closes over tables — `SELECT` consumes
them and produces them. glossql closes over context: statements produce it,
and `GLOSS … AS PACK` reproduces the statements — the active (unsuperseded)
writing statements in the subject's closure, scope bounded by serving policy
like the document form. The document form is the rendered page — authored
plus derived, for prompts and screens; its posteriors are computed, so it
cannot replay. The pack form is the source: `SHOW CREATE TABLE`, generalized
to a context neighbourhood. Replaying a pack elsewhere reproduces the context;
witnesses travel (they are log statements), derived state is recomputed,
never exported. Export, dump/restore, and vocabulary-pack publishing are one
mechanism. How a whole vertical is addressed for export (subject closure vs
actor scope) is a flag item; distribution stays reserved (§6).

The graph is the serving **spine**, not a consumer verb set. `GLOSS`'s
renderer traverses the derived graph projection to assemble each concept's
neighbourhood (part-of closure, confirmed relationships, drivers, additivity) —
fieldwork: when the running system made the graph its one serving spine, neither
LLM agent gained a traversal tool; the graph arrived as pushed structure. Code
consumers (drill UIs) query the same projection as derived relations with plain
SQL — recursive CTEs are SQL (principle 7). No graph grammar exists.

The projection's vocabulary is not new vocabulary: nodes are the language's own
names (tables, columns, concepts, groundings, metrics, parameters, plus derived
verdicts); edges are the references statements already carry or that the engine
derives from their ASTs (`REFERENCES` pairs, concept relations, grounded-by,
uses, rolls-up-to, derives-from). The derived plane exposes it as relations;
the exact shape is engine-defined, not grammar. Two fieldwork notes: today's
projection outruns consumption (9 of 25 element views have no production
reader — project on demand, not by completeness), and its one tuning constant
(part-of closure depth, hand-mirrored across two languages today) is exactly
the kind of number that becomes declared serving policy here.

Rendering extensions (ggsql-style trailing `VISUALISE` clauses) are compatible with
this grammar and out of scope for v0.

### 3.6 Lifecycle

```sql
RETRACT unit(orders.amount) BY USER analyst;          -- removes, no replacement
RETRACT null_token(orders.amount, token := 'n/a')
  BY USER analyst;                                    -- the argument addresses the slot
```

Supersession needs no statement (re-apply the same (subject, aspect[, argument])
slot). There is
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
and the trace (`WHY()`). Readiness aggregates contested/ignorant aspects per
target under the readiness policy. Verdicts for validations apply declared tolerance
to observed deviation at read time.

Nothing in this plane is authored. In particular, resolution does **not** write back
into declarations (a change from today's resolve write-back): if an agent or user
accepts a posterior, that acceptance is a new aspect-application
`DECLARE ... BY ...` in the log — authored, attributed, and auditable like
everything else.

---

## 6. Reserved statement space (not yet covered, room held)

One line each; none designed in v0:

- **Synonyms** on any subject (`SYNONYMS ('turnover', ...)`) — we don't do this systematically yet.
- **Verified example queries** — question + query + verified-by; today only half-exists as saved snippets.
- **Agent instructions per subject** — prose guidance scoped to a table/metric (beyond global conventions).
- **Expectations on incoming data** — schema stability, freshness SLAs, arrival contracts.
- **Unit conversion** — FX, unit algebra; today units are labels only.
- **Entity resolution** — same-entity assertions across sources.
- **Vocabulary sharing** — the format is resolved: a pack is a saved gloss (`AS PACK`, §3.5) and import is replay. Reserved here: distribution (registries, versioning) and addressing a whole vertical for export.
- **Visualization clauses** — ggsql-compatible rendering tail.
- **Visibility/governance** — who may see which context.

## 7. Deliberately excluded

- **Orchestration and scheduling** — the language requests observations; it never sequences them.
- **Prompt configuration** — LLM prompts/versions are operational engine config, not context.
- **Storage layout** — log/lake encodings are implementation.
- **Interchange formats** — an Ossie mapping is possible for the vocabulary tier and is not part of the language.

---

## 8. Open questions for review

Items carry direction from review, not final decisions; "shaped later" is a
legal resolution.

1. **Universal-core aspects** — direction: the built-in-function model. Like
   `md5()`, a core library of data-processing aspects (null semantics, type
   parsing) is standardized and *named* by the spec, shipped as a seed pack —
   product-standard, never grammar. Aspects are extensible the way engine UDFs
   are (an extension point, not a first concern). Domain aspects (behavior,
   cycle stages) stay pack vocabulary regardless. Remaining: the core list and
   its label sets.
2. **Concept relations** — decided and folded into §3.1: authored edges are
   statements under the `DECLARE RELATIONSHIP` head (`PART OF`,
   `RECONCILES WITH … TOLERANCE`, `DISJOINT`), one closed, mechanism-backed
   operator set across both spaces. Remaining open: the `RECONCILES WITH`
   right-hand side (bare concept vs concept-space expression), and whether a
   `RECONCILES WITH` edge subsumes `KIND balance` validations or only feeds
   them. The statement axis on metric extracts is decided: `PART OF` structure
   (§3.2), not a grounding key member.
3. **Typing vocabulary and DECLARE proliferation** — type-pattern teaches,
   workspace-level null-token vocabulary, and expected-dependency assertions
   still have no statement form. Direction: resist new `DECLARE` families;
   prefer SQL-bodied forms under few heads. `DECLARE VIEW … AS` (§3.2) now
   hosts the transform-shaped half (enrichment, cleaning, typing transforms);
   type patterns are expressions and likely ride an existing head. Start
   small and see how it works out.

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

Transcription covers artifacts; **walkthroughs** cover flows. For each producing
moment in §2.5 (pack import, onboarding, a catalog run, a calibration release, a
metric execution, a user teach, an agent answering a question) write the exact
statement sequence produced and consumed, end to end. §2's rows prove every
artifact has a home; walkthroughs prove every moment has a script. §10 is the
canonical walkthrough; the corpus extends it to every artifact.

Both checks may be tooled: a disposable validation harness — grammar parser,
log replay and pooling simulator, and a constrained-decoding authoring test
(§2.5's regularity claim, actually tested against an LLM) — whose only
outputs are bucket verdicts and SPEC.md diffs. The harness is not the
implementation and does not survive it; §9.2 stays gated on the cleared
flags.

### 9.2 First implementation slice (v0.1 PoC)

Scope for the first implementation, gated on §9.1 — the adjudication slice on
DataFusion, chosen because it exercises the two riskiest bets at once: the
DataFusion extension path and the witness/adjudication plane. Log and lake
encodings below are placeholders; the persistence decision (§1.1) stays open.

- **Statements:** `DECLARE CONCEPT`, `DECLARE ASPECT`, aspect-application
  `DECLARE`, `OBSERVE`, `WITNESS`, `DECLARE RELIABILITY`, `DECLARE POLICY`
  (readiness bands only), `RETRACT`.
- **Consumption:** `CONTEXT()`, `DECLARATIONS`, `WHY()`, `AT`.
- **Substrate:** DataFusion custom statements; log = one plain-text statement
  file; lake = a parquet directory.
- **Loop closure:** one built-in detector (null semantics over `profile`
  results), so `OBSERVE → lake → WITNESS → pooling → CONTEXT` runs end to end.
- **Acceptance:** replay determinism (identical derived state from log + lake),
  contested detection under declared reliabilities, a faithful `WHY()` trace,
  `AT` time travel by prefix replay.
- **Excluded:** `DECLARE GROUNDING`, metrics, validations, views, hierarchies,
  serving policy, the `GLOSS` document renderer.

---

## 10. Happy path — one workspace, end to end

The canonical walkthrough (§9.1): the producing moments of §2.5 in the order a
real workspace meets them, as well-formed statements (abridged where marked).
Two boundaries hold throughout. Orchestration appears nowhere: workflows
*emit* statements and *read* derived state — the language says what, never
when (§7). And the cockpit is a pure consumer: every inspection surface is
`GLOSS` or a relation; every teach control emits a `DECLARE`. A cockpit
feature that cannot be written this way is a grammar gap (§9.1, bucket two).

**1 · Frame.** Choosing a vertical replays its pack — framing is not a
mechanism, it is provenance:

```sql
DECLARE ASPECT behavior VALUES (flow, stock, point_in_time) BY SEED core;
DECLARE CONCEPT revenue KIND measure UNIT FROM currency
  DESCRIPTION 'income from sales of goods and services' BY SEED finance;
DECLARE RELATIONSHIP net_revenue PART OF revenue BY SEED finance;
DECLARE METRIC dso AS 90 * avg(receivables) / sum(revenue) UNIT 'days'
  BY SEED finance;
-- abridged: receivables, collections, conventions, validations elided
```

**2 · Connect.** The add-source workflow authors bindings, then requests
evidence; its progress widget polls a derived relation, never the log:

```sql
DECLARE SOURCE erp_export FROM 'lake/erp/*.parquet' BY USER analyst;
DECLARE TABLE orders FROM erp_export BY USER analyst;
DECLARE TABLE customers FROM erp_export BY USER analyst;
DECLARE SOURCE crm CONNECTION postgres VIA 'crm_prod' BY USER analyst;
DECLARE TABLE segments FROM crm
  AS 'SELECT id, segment FROM customer_segments' BY USER analyst;  -- a recipe
OBSERVE profile, typing, temporal ON orders BY AGENT onboarding;
```

**3 · Witness.** Detectors return claim distributions; bulk results land in
the lake behind the `EVIDENCE` ref:

```sql
WITNESS behavior(orders.amount, flow := 0.83, stock := 0.11, point_in_time := 0.06)
  BY DETECTOR aggregation_lineage
  EVIDENCE 'obs://run-342/aggregation_lineage/orders.amount';
```

**4 · Catalog.** Agents declare — including, here, a mistake:

```sql
DECLARE entity(orders, value := 'sales order') BY AGENT cataloguer CONFIDENCE 0.9;
DECLARE behavior(orders.amount, value := stock) BY AGENT cataloguer CONFIDENCE 0.7;
DECLARE RELATIONSHIP orders.customer_id REFERENCES customers.id
  CARDINALITY many_to_one BY AGENT judge CONFIDENCE 0.97;
```

**5 · Contest.** Nothing is authored; the pooled posterior disagrees with the
declaration, and the cockpit shows the badge:

```sql
SELECT aspect, value, posterior, contested FROM CONTEXT('orders.amount');
--  behavior | stock | {flow: 0.79, …} | true
SELECT * FROM WHY('orders.amount', 'behavior');  -- the full trace behind the badge
```

**6 · Teach.** The cockpit's accept control emits one statement: same slot,
new declaration, contested clears. The log remembers:

```sql
DECLARE behavior(orders.amount, value := flow) BY USER analyst;
AT '2026-07-29' SELECT contested FROM CONTEXT('orders.amount');  -- true, back then
```

**7 · Derive and ground.** The enricher writes a view; the grapher bridges
concept to columns:

```sql
DECLARE VIEW orders_enriched AS
  SELECT o.order_id, o.amount, c.region, c.segment
  FROM orders o JOIN customers c ON o.customer_id = c.id
  BY AGENT enricher;

DECLARE GROUNDING revenue IN orders
  AS amount WHERE doc_type = 'invoice'
  BY AGENT grapher CONFIDENCE 0.9;
```

**8 · Execute.** With `receivables` and `collections` grounded the same way,
the concept-space metric runs against data:

```sql
SELECT month, value FROM METRIC('dso', grain => 'month');
```

**9 · Answer.** The answer agent reads the rendered document, then plain SQL;
it writes nothing — its teach suggestions surface to users, who write:

```sql
GLOSS (SELECT sum(amount) FROM orders GROUP BY channel)
  USING SERVING answer_agent;
```

**10 · Share.** Closure: context leaves the same way it arrived —

```sql
GLOSS revenue AS PACK;  -- the active authored statements in revenue's closure
```

Ten stations, five verbs, one log. §9.1's corpus is this path widened to
every artifact and every moment.
