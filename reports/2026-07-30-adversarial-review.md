# Adversarial review of SPEC.md — 2026-07-30

Three independent passes: an internal-consistency review of SPEC.md itself, a
claim-by-claim fact-check of §2 against `../dataraum-context`, and the first-ever
execution of the spec's own §9.1 transcription test against nine real artifacts.
Non-normative; a §9.1 output (bucket verdicts) in report form.

## Verdict

The spec fails its own acceptance test. Of nine real artifacts transcribed strictly
per §3: one transcribes cleanly, seven hit grammar gaps, three hit undefined
semantics, six lose information. Separately, seven of the spec's own statement
families are not derivable from the §3.0 skeleton — every keyed class
(`RELATIONSHIP`, `GROUNDING`, `RELIABILITY`), all three `POLICY` shapes,
`CYCLE FAMILY`, and the never-defined `observation` and `lifecycle` productions.
`clauses` is an undefined nonterminal doing all load-bearing work.

Process evidence: SPEC.md grew 427 → 644 → 723 → 976 lines over four iterations in
two days while §8 holds the identical four open questions since iteration 1. The
§9.1 loop the spec prescribes had never been run before this review; the CLAUDE.md
harness carve-out was never used.

---

## Part I — Internal consistency (no repo needed)

### The §3.0 skeleton cannot produce the spec's own examples

`declaration := DECLARE class name clauses provenance ';'` cannot produce:

- `DECLARE RELATIONSHIP orders.customer_id REFERENCES customers.id …` — keyed
  class: the token after the class is a pair, not a name; no production exists for
  keyed classes at all.
- `DECLARE RELATIONSHIP net_revenue PART OF revenue` — name position holds an edge
  operand; `DISJOINT (a, b, c)` puts an operator there. Three edge operators, three
  token shapes, none derivable.
- `DECLARE GROUNDING revenue IN orders AS …` — and the stated supersession key
  (concept, relation, **parameter**) has a member with no surface syntax anywhere.
- `DECLARE RELIABILITY DETECTOR x FOR behavior 0.72` — actor after class, bare
  trailing number, keyed by (actor, aspect) in prose only.
- `DECLARE POLICY readiness` / `POLICY contract name` / `POLICY interpretation FOR
  dso` — three key shapes under one undeclared head.
- `DECLARE CYCLE FAMILY conversion` — multi-token class, nothing permits it.
- `DECLARE calendar(workspace, …)` — aspect application for an aspect never
  declared; its argument fits neither VALUES nor the argument/label rule.
- `OBSERVE` and `RETRACT` are named in the `writing` alternatives and never given
  productions.

### Parse depends on catalog state

Aspect-application arguments and claim-space labels share one named-argument
surface, disambiguated only by the aspect's declaration — a semantic form cannot be
built without replaying the log. This conflicts with §2.5's constrained-decoding
claim: the decoding grammar would need the live glossary. Similarly `GLOSS revenue`
vs `GLOSS orders`: one bare-identifier position resolving across the two namespaces
the spec everywhere else keeps separate; a table named `revenue` is ambiguous.

### The hardest problem is a one-liner

`SELECT month, value FROM METRIC('dso', grain => 'month')` requires composing three
groundings, possibly over different relations, into one query at month grain:
relation selection, join-path discovery, grain alignment, calendar application.
This is query planning across concept space — the core value proposition — and the
spec spends one line on it, flagging only the grain argument.

### Validation execution is underdetermined by construction

"No formal check expression" + principle 4 (prose never parsed) ⇒ executable SQL
must derive from KIND + OVER + tolerance alone. For `KIND balance`,
opening/closing conventions, signs, and direction do not follow from a concept
list. The mechanism pointed at (`RECONCILES WITH` edges) is itself open (§8.2).
Circular.

### One-document-rule pathology

~976 lines, of which the normative grammar is maybe 150; the rest is status
tracking, the §2 map, rationale essays, fieldwork anecdotes, process definition,
and a walkthrough. Because every discussion "becomes a SPEC.md diff," ideas are
folded in as prose commitments instead of tested; prose tolerates ambiguity
indefinitely, so nothing forces closure. Every example in the spec is invented
(orders.amount, dso) even though §9.1 demands real artifacts.

### Other

- `AT '2026-07-01' SELECT …` means glossql must intercept the entire SQL session —
  a large engine implication treated as a syntax-alignment flag.
- `GLOSS … AS PACK` scope "bounded by serving policy": serving-policy fields shape
  prompt documents, not statement closures — category mismatch, acknowledged.
- §2.5's "text form exists for diffability, not authoring ergonomics" exempts the
  language from the one usability test that would catch these failures; no artifact
  demonstrates any agent authoring a statement.

---

## Part II — §2 fact-check against dataraum-context

Method: adversarial verification of every §2 claim with file:line evidence, plus a
reverse check for what the map omits. Paths relative to
`/Users/philipp/Code/dataraum/dataraum-context/`.

### A. Declared artifacts (§2.1)

| # | Claim | Verdict | Evidence |
|---|---|---|---|
| A1 | Concepts: name, description, indicators, kind, relations | PARTLY | `packages/engine/schema.sql:60-79` — no `relations` column; relations are the separate `concept_edges` table (`:5-20`). Unlisted by the spec: `exclude_patterns`, `unit_from_concept`, `ordering`, `vertical` scoping. |
| A2 | Conventions: opaque prose served verbatim | PARTLY | `schema.sql:92-106` — `statement` is verbatim, but `concept_groups` is parsed and mechanically consumed (`concept_edge_store.py:78-90` derives disjoint edges; `induction.py:487`, `ontology.py:103-118` resolve members) and `targets` routes per consumer (`finance/ontology.yaml:264-305`). Not opaque. |
| A3 | Metrics: expression, unit, output, parameters, interpretation ranges, DAG | PARTLY | All in YAML (`finance/metrics/working_capital/dso.yaml`); DB `schema.sql:298-317`, `:278-296`, `:266-276`. But interpretation ranges are NOT persisted (graph-only, `graphs/loader.py:100-109`); unmentioned: per-step inline validations, `decimal_places`, `category`, `tags`; `extract` steps are standard_field+statement+aggregation, not expressions. |
| A4 | Validations: check, tolerance, severity, guidance, cycle scope | PARTLY | `schema.sql:383-408`. "check" = `check_type` (a kind). Unmentioned: `category`, `expected_outcome`, `relevant_conventions` (load-bearing, DAT-865), `tags`, `version`, `induced_validations` (`:169-193`), `validation_results` (`:369-381`). |
| A5 | Cycle families: closed families + directions | CONFIRMED | `schema.sql:108-120`; `finance/cycles.yaml:365-369`. Directions bind concept names, not bare labels (§8.4 admits). |
| A6 | Workspace calendar + active vertical | CONFIRMED | `schema.sql:425-432`, `:434-440`. |
| A7 | Column annotations (LLM) | CONFIRMED | `schema.sql:901-920`; `entity_type` is a 6th field the spec omits. |
| A8 | Column concepts (LLM) | PARTLY | `schema.sql:674-698`. Spec omits the entire stored-sign claim family (own detector `stored_sign.py:47-50`, own resolve write-back `resolve.py:146`, graph vertex property `schema_graph.sql:570`) and `meaning_status`. |
| A9 | Table entities (LLM) | CONFIRMED | `schema.sql:631-648`. |
| A10 | Relationships: type, cardinality, confirmed-by | CONFIRMED | `schema.sql:857-885`; `REJECTED` maps to `judge_verdict='declined'`. |
| A11 | Hierarchies: drilldown/alias/role, levels | CONFIRMED | `schema.sql:536-557` plus g3, role_verdict, identity_confidence, needs_confirmation. |
| A12 | Slice definitions: priority + business context | PARTLY | `schema.sql:922-950` — priority is `slice_relevance` plus a second axis `slice_interest` (primary/supporting); more unmentioned fields. |
| A13 | Enrichment selection | CONFIRMED | `schema.sql:563-580`; `is_grain_verified` matches §3.2. |
| A14 | Business cycles (LLM) | CONFIRMED | `schema.sql:141-167`; richer than the spec's four items (`entity_flows`, `tables_involved`, family/direction binding). |
| A15 | Surrogate key confirmation | CONFIRMED | `schema.sql:609-627`. |
| A16 | Groundings as snippet parts + provenance | CONFIRMED | `schema.sql:333-357`; `snippet_models.py:109-116`; `schema_read.sql:523-545` projects exactly concept/relation/expr/filters. `parts` also carries a `period_binding` the spec doesn't mention. |
| A17 | Teach payloads: "all 8 types, today free JSON" | PARTLY (both halves wrong in detail) | 8 registered (`teach.validation.ts:195-216`) + a 9th direct-read type `expected_dependency` (`overlay.py:49-55`). "Free JSON" is wrong: every type is Zod-validated (`teach.validation.ts:328-332`) and bound to one merge fn (`overlay.py:338-385`). Surface split three ways by call site (`:227-264`). |
| A18 | DB recipes + `recipe_hash` | CONFIRMED | `recipe-source.ts:134-160`; hash construction `source-content-hash.ts:74-100`; `schema.sql:319-331`. |

### B. Measurements (§2.2)

All twelve non-numeric rows CONFIRMED (profiles `schema.sql:958-976`; quality
`:982-997`; temporal `:1003-1025`; type candidates `:1029-1047`; eligibility
`:485-505` + `phases/column_eligibility.yaml:23-35`; derived columns `:700-716`;
overlap as evidence blob on relationships `analysis/relationships/detector.py`;
g3 `:544,555`; aggregation lineage `:817-851`; drivers `:724-748`; additivity
`:233-258`; validation execution contract `validation-verdict.ts:14-18` +
ADR-0017). **"17 entropy detectors across 4 layers" is WRONG on count: 4 layers
confirmed, but 18 detectors** (`entropy/detectors/__init__.py:72-96`).

### C. Evidence (§2.3)

- `claim_witnesses` CONFIRMED (`schema.sql:650-666`) — but "closed spaces" is not
  a data property today: `claim_field` is a bare VARCHAR; label sets are Python
  constants (`temporal_behavior.py:53`). `DECLARE ASPECT … VALUES` has no
  counterpart in the running system.
- "JSONB evidence columns" PARTLY: exactly one JSONB (`entropy_objects.evidence`);
  the others are plain JSON.
- Profile tables CONFIRMED.

### D. Serving (§2.4 / §3.5 fieldwork)

- "Six-block answer-agent context" — PARTLY: it is **nine** blocks
  (`query.ts:813-836`); the ninth, `<business_concepts>`
  (`query-context.ts:1290-1327`), appears nowhere in §2.
- "One mechanism, two policies" for GLOSS — aspirational: the engine GraphAgent
  context (`graphs/context_format.py`) is a different, larger, independently
  implemented block set in another language; the cockpit documents hand-mirroring
  it (`query-context.ts:69-73`, `:714`).
- Readiness surfaces CONFIRMED (`schema.sql:791-809`; bands
  `entropy/loss.yaml:24-28`).
- Why-tools CONFIRMED but understated: six of them, and they are LLM syntheses,
  not pure relations — which §3.5's `WHY()`-as-relation quietly drops.
- Look-tools: eight, not four.
- `og_*` projections CONFIRMED (`schema_graph.sql`, property graph at `:561`).
- **"9 of 25 element views with no production reader" — 25 confirmed; the 9
  matches no artifact.** The only census on record (ADR-0024, Status: *Proposed*)
  says 15 of 26. A crude fresh census finds 4 with zero mentions.
- **"Hardcoded dimension budget of 12" — already removed**; the code describes
  `CURATED_SLICE_BUDGET` in the past tense as a replaced defect
  (`slicing/models.py:11`, `query-context.ts:361`).
- Prefer-enriched rule CONFIRMED, hardcoded in both languages.
- Join whitelist CONFIRMED — prompt-enforced, not admission-enforced (§3.2's
  admission check has no counterpart today).
- Byte-identical cached context prefix CONFIRMED (DAT-660, `query.ts:837-892`).
- Part-of closure depth hand-mirrored CONFIRMED (`context_reads.py:729-735` ↔
  `concept-graph-load.ts:69-75`, drift-guard tests only defense).

### E. Mechanisms the spec says exist today

- Resolve write-back CONFIRMED — three write-backs (`entropy/resolve.py`:
  null_tokens :32-68, temporal_behavior :72, stored_sign :146).
- `standard_field` joins PARTLY — string equality on a *declared concept name*
  (~43 name-keyed joins per ADR-0024), not a "naming convention"; §3.0's phrasing
  mislabels what it replaces.
- Actor vocabularies CONFIRMED and understated: the one `BY` clause must absorb
  **≥7** disjoint source vocabularies (source, confirmation_source,
  detection_source, annotation_source, decision_source, detection_method,
  detector_id), not 3.

### F. Reverse check — what §2's map does not cover at all

1. **The run/promotion axis** (largest gap; an axis, not a row):
   `metadata_snapshot_head` (`schema.sql:223-231`) — every `current_*` read view
   joins through it; `lifecycle_artifacts` four-state machine
   (declared/executed/grounded/canonical, with `strictness`, `grounded_against`,
   and a second teach channel in `teaches`); `type_decisions`
   (`schema.sql:1051-1065`). The spec flattens all of it into supersession and
   never lists it as covered, reserved, or dropped.
2. **Bus matrix / conformed dimensions** (`schema.sql:459-479`): served as a graph
   edge, rendered in agent context, own cockpit tools and LLM prompts and UI
   screen. No row in §2, no head in §3.
3. **`concept_reconciliation`** (`schema.sql:22-58`): the execution results of
   `RECONCILES WITH`, with a verdict and a seven-value abstain vocabulary. No
   measurement id, no derived-plane mention.
4. **Cockpit's own authored persistence** (`cockpit/src/db/cockpit/schema.ts`):
   reports (frozen composed CTE + drill params + chart config + evolve lineage),
   conversations, UI state. Directly contradicts §10's "the cockpit is a pure
   consumer" — by §10's own rule, a large unopened bucket-two finding.
5. **`analysis_hints`** (`finance/cycles.yaml:376-395`): authored, pack-shipped
   prose steering detectors — §6 "reserves" what already ships.
6. **Detector tuning constants** (`entropy/thresholds.yaml`): per-detector scoring
   parameters; not covered by `DECLARE RELIABILITY` (pooling weights only) nor
   excluded by §7.
7. **Physical materialization**: `materialization_recipes` (per-layer DDL),
   `tables.layer` (four physical rows per logical name), the surrogate mint
   (writes `_sk__*` columns), enriched-column lineage.
8. **Per-phase config as policy** (`phases/*.yaml`): e.g. column-eligibility rule
   lists with thresholds and reason templates — results mapped in §2.2, rules
   unmapped.
9. **Teach-as-detector-input**: `relationship` and `expected_dependency` overlays
   feed detectors' measurements directly (`overlay.py:49-55`) — §4/§5's clean
   witness-vs-declaration split has no place for a declaration that is a
   detector's input.
10. **The drill family** (five canonical questions, ~75 call sites per ADR-0024)
    and ~25 further cockpit tools with no §2 row.
11. **Vertical pack contents** beyond §2: the pack envelope (name/version/
    description → `vertical_envelopes`), `compositions`, `cycle_types`'s
    aliases/feeds_into/business_value, metric interpretation persistence.

### Overall

Directionally sound, numerically unreliable, materially incomplete. §2 is a good
inventory and a bad census: trust it that something exists; do not trust it for
how many, what it's called, or whether it's still there. Four numeric claims,
four defects. One entire dimension (run scoping/promotion) plus at least two full
artifact families are missing. If §9.1 ran against §2 as written it would declare
completeness while F1–F4 and F8 go untranscribed.

---

## Part III — Transcription test (§9.1, first execution)

# Adversarial transcription test — glossql SPEC.md vs dataraum-context real artifacts

Method: real artifacts pulled verbatim from /Users/philipp/Code/dataraum/dataraum-context;
transcription attempted strictly per SPEC.md §3 (no invented syntax — any invention is
flagged as the finding). Classifications: TRANSCRIBES CLEANLY / GRAMMAR GAP /
SEMANTICS UNDEFINED / INFORMATION LOST. A single artifact can carry several classifications
(per-field). Raw findings, no softening.

---

## Part A — Skeleton coverage: can §3.0's own grammar produce the spec's own examples?

The §3.0 skeleton:

```
declaration := DECLARE class name clauses provenance ';'
             | DECLARE aspect '(' subject { ',' arg ':=' value } ')' provenance ';'
witness     := WITNESS aspect '(' subject { ',' arg ':=' value } ')' provenance ';'
provenance  := BY actor [ CONFIDENCE number ] [ EVIDENCE ref ]
subject     := workspace | table | table '.' column | pair | declared_name
```

Checked against every example in §3.1–§3.6 and §10. Failures:

1. **`DECLARE RELATIONSHIP orders.customer_id REFERENCES customers.id CARDINALITY … BY …`**
   NOT derivable. The named-class production is `DECLARE class name clauses provenance`,
   where "name" is a single token (§3.0: "the token after the class is the name"). Here the
   token after the class is a *pair* (a subject form), not a name — and §3.0 itself says
   RELATIONSHIP is a *keyed* class that "carries no name". The skeleton has no production
   `DECLARE class subject clauses provenance` or `DECLARE class pair …`. Keyed classes are
   described in prose but have no grammar production. Same failure for the composite form
   `DECLARE RELATIONSHIP txn (account, business_id) REFERENCES coa (…) …`.

2. **`DECLARE RELATIONSHIP net_revenue PART OF revenue BY SEED finance;`**
   NOT derivable, worse than (1): the token after the class (`net_revenue`) *looks* like a
   name but is actually the edge's left endpoint; `PART OF revenue` must then be a "clause",
   but clauses are undefined nonterminals. Also `pair` in §3.0 only covers `REFERENCES`
   pairs — the concept-space edge key (`x PART OF y`) is not a `pair` and not a `subject`.
   `DECLARE RELATIONSHIP DISJOINT (a, b, c)` is even further off: the token after the class
   is an *operator*, with no name and no left endpoint at all. Three edge operators, three
   different token shapes after `RELATIONSHIP`, none produced by the skeleton.

3. **`DECLARE POLICY interpretation FOR dso BANDS (…) BY SEED finance;`**
   NOT derivable. Two tokens (`POLICY` + kind) occupy the class position, and the identity
   is the `FOR dso` key, not a name. §3.0's three declaration shapes say policy keys are
   "mechanism-defined" — but no production exists for `DECLARE POLICY kind [name] [FOR key]`.
   `DECLARE POLICY readiness` (keyless singleton), `DECLARE POLICY contract
   exploratory_analysis` (named), and `DECLARE POLICY interpretation FOR dso` (keyed) are
   three different shapes under one undeclared head.

4. **`DECLARE RELIABILITY DETECTOR aggregation_lineage FOR behavior 0.72 BY CALIBRATION '2026-07';`**
   NOT derivable. After the class comes an *actor* (`DETECTOR aggregation_lineage`), then a
   `FOR aspect` key, then a bare number with no clause keyword. "name" cannot produce
   `DETECTOR aggregation_lineage`, and a floating literal `0.72` is not a clause under any
   stated clause convention. Keyed by (actor, aspect) per prose; no production.

5. **`DECLARE GROUNDING revenue IN orders AS sum(amount) WHERE doc_type = 'invoice' …`**
   NOT derivable as stated. GROUNDING is keyed by (concept, relation, parameter); the token
   after the class is the concept (fine as "name"), but `IN orders` / `AS expr` / `WHERE pred`
   are clause forms the skeleton leaves as the unconstrained nonterminal `clauses` — while
   the *key* (concept, relation, parameter) straddles the name position and two clauses. The
   parameter member of the stated supersession key has NO surface syntax anywhere in §3.2's
   examples — nothing in `DECLARE GROUNDING revenue IN orders AS … WHERE …` carries a
   parameter. Key member with no syntax.

6. **`DECLARE CYCLE FAMILY conversion DIRECTIONS (forward, reverse) …`**
   Marginal: "class" must be the two-token sequence `CYCLE FAMILY`. Nothing in §3.0 says a
   class can be multi-token. Minor but real.

7. **`DECLARE calendar(workspace, fiscal_year_starts := april) BY USER analyst;`**
   Derivable only if `calendar` is a declared ASPECT — no `DECLARE ASPECT calendar …`
   appears anywhere in the spec, and its argument payload (`fiscal_year_starts := april`)
   fits neither a VALUES label set nor the stated argument/label discrimination rule
   (admission tells arguments from labels "by the aspect's declaration"; calendar has none).

8. **`OBSERVE profile, temporal, quality ON orders;`** — `observation` is named in the
   writing alternatives (`writing := declaration | witness | observation | lifecycle`) but
   has NO production at all. Same for `lifecycle` (`RETRACT aspect '(' subject [, arg] ')'
   provenance`): used in §3.6, never defined. `OBSERVE validation receivables_roll_forward`
   (measurement id + declared name) and `OBSERVE overlap ON (orders.customer_id,
   customers.id)` (a parenthesized column *tuple* that is not the §3.0 `pair` form — no
   REFERENCES) are both underivable even charitably.

9. **`GLOSS revenue AS PACK;`** — derivable from the `gloss` production. But `AT '2026-07-01'
   SELECT …` requires `sql_query` to be a defined nonterminal (it is inherited, fine), while
   `reading := [AT timestamp] (gloss | sql_query)` makes `GLOSS` and SQL peers — yet §3.5's
   context relations (`CONTEXT('orders.amount')`) are "standard SQL", so the reading side
   holds. No failure here.

10. **`WITNESS behavior(orders.amount, flow := 0.83, stock := 0.11, …)`** — derivable.
    But note the witness production forces `arg ':=' value` pairs where the *labels of the
    claim space* appear in argument position; the skeleton cannot syntactically distinguish
    `token := 'n/a'` (an argument) from `is_null := 0.91` (a distribution label) — the spec
    admits this is resolved semantically at admission. Parses; deliberate.

**Skeleton verdict: 7 of the spec's own statement families (RELATIONSHIP both spaces,
POLICY ×3 shapes, RELIABILITY, GROUNDING key, CYCLE FAMILY, calendar aspect, OBSERVE,
RETRACT) are not derivable from the §3.0 skeleton as written.** The skeleton covers named
classes and aspect applications/witnesses; every *keyed* class it describes in prose has no
production. `clauses` is an undefined nonterminal doing all load-bearing work.

---

## Part B — Artifact transcriptions

### Artifact 1 — Concept definition (finance pack)

Source: `/Users/philipp/Code/dataraum/dataraum-context/packages/dataraum-config/verticals/finance/ontology.yaml`

```yaml
  - name: revenue
    description: Income from sales or services
    indicators:
      - revenue
      - sales
      - income
      - turnover
      - receipts
    exclude_patterns:
      - cost
      - expense
    kind: measure
    unit_from_concept: currency
```

Pack envelope around it (same file): `name: financial_reporting`, `version: "1.0.0"`,
`description: |  Financial analysis and reporting context. …`

Attempted transcription (per §3.1):

```sql
DECLARE CONCEPT revenue
  KIND measure
  DESCRIPTION 'Income from sales or services'
  INDICATORS ('revenue', 'sales', 'income', 'turnover', 'receipts')
  EXCLUDE ('cost', 'expense')
  UNIT FROM currency
  BY SEED finance;
```

Classification: **TRANSCRIBES CLEANLY** for the concept row itself — §3.1's clause set maps
1:1 (name, kind, description, indicators, exclude_patterns→EXCLUDE, unit_from_concept→UNIT
FROM).

Residual findings:
- **INFORMATION LOST — pack envelope.** The vertical's `version: "1.0.0"` and pack-level
  `description` have nowhere to go. §3.1 says "a pack is a file of statements" and provenance
  is `BY SEED finance` — the seed *name* survives but the pack *version* does not. The
  running system persists it (`vertical_envelopes.version` in schema.sql). No statement form
  carries a pack version; §6 reserves "distribution (registries, versioning)", so this is a
  deliberate drop — but it drops a field the running system stores today.
- **INFORMATION LOST — `account_balance` comment block** (ontology.yaml lines 195–201): the
  DAT-405 rationale comment ("a balance column is a point-in-time stock per period…") is
  authored knowledge that steers agents. In glossql it could only survive as DESCRIPTION
  prose; the file-comment channel has no home. Marginal (comments are not data), noted.
- The `compositions:` block at the bottom of ontology.yaml (`whole: current_assets, parts:
  cash, accounts_receivable, inventory`) transcribes as three `DECLARE RELATIONSHIP cash
  PART OF current_assets BY SEED finance;` statements — clean *semantically*, but each such
  statement is skeleton-underivable (Part A item 2).

### Artifact 1b — Convention (same file, served verbatim to SQL agents)

```yaml
  - id: sign_natural_balance
    targets: [extraction, qa]
    statement: >
      Express every monetary measure in its natural-balance direction …
      never normalize only one side of a comparison.
    concept_groups:
      credit_normal:
        - revenue
        - accounts_payable
        - current_liabilities
        - equity
      debit_normal:
        - cost_of_goods_sold
        - operating_expense
        - depreciation
        - tax
        - accounts_receivable
        - inventory
        - current_assets
        - cash
```

Attempted transcription (§3.1 gives only `DECLARE CONVENTION name STATEMENT '…' BY …`):

```sql
DECLARE CONVENTION sign_natural_balance
  STATEMENT 'Express every monetary measure in its natural-balance direction …'
  BY SEED finance;
```

Classification: **GRAMMAR GAP (two fields have no clause).**
1. `targets: [extraction, qa]` — the routing that decides *which* SQL-authoring agents
   receive the convention. In the spec's model this is serving policy (`DECLARE SERVING …
   INCLUDE (conventions, …)`), but §3.4's INCLUDE selects the conventions *family*, not
   individual conventions per consumer; there is no per-convention target/scope surface.
   The running system also routes per-validation via `relevant_conventions` — that half
   maps (VALIDATION side, see Artifact 3), but the convention-side `targets` half does not.
2. `concept_groups` — machine-readable, engine-*linted* structure ("lints that group
   members are declared concepts" — the OVER-style membership contract, applied *inside*
   a convention). §1.2(4) makes convention prose opaque; but `concept_groups` is exactly
   the part the engine does NOT treat as opaque today. No clause exists to carry a named
   group of concept references on a CONVENTION. Folding it into the prose loses the
   declaration-time membership check the running system performs.

### Artifact 2 — Metric with parameters and interpretation ranges (dso)

Source: `/Users/philipp/Code/dataraum/dataraum-context/packages/dataraum-config/verticals/finance/metrics/working_capital/dso.yaml` (full file):

```yaml
graph_id: dso
version: '1.0'
metadata:
  name: Days Sales Outstanding
  description: Average days to collect payment after sale
  category: working_capital
  source: system
  tags: [ar, collection, working-capital]
output:
  type: scalar
  metric_id: dso
  unit: days
  decimal_places: 1
parameters:
  days_in_period:
    type: integer
    default: 30
    options: [30, 90, 365]
    description: Analysis period length
    derivation: period_grain
dependencies:
  accounts_receivable:
    level: 1
    type: extract
    source:
      standard_field: accounts_receivable
      statement: balance_sheet
    aggregation: sum  # period axis (all vs latest) is data-reconciled via target_type — never hardcode end_of_period
  revenue:
    level: 1
    type: extract
    source: {standard_field: revenue, statement: income_statement}
    aggregation: sum
  days_in_period:
    level: 1
    type: constant
    parameter: days_in_period
    default: 30
  dso:
    level: 2
    type: formula
    expression: (accounts_receivable / revenue) * days_in_period
    depends_on: [accounts_receivable, revenue, days_in_period]
    output_step: true
    validation:
    - condition: 0 <= value <= 365
      severity: warning
      message: DSO outside typical range
interpretation:
  ranges:
  - {min: 0, max: 30, label: EXCELLENT, description: Very efficient collection}
  - {min: 31, max: 45, label: GOOD, description: Strong collection performance}
  - {min: 46, max: 60, label: CONCERNING, description: Review collection processes}
  - {min: 61, max: 90, label: POOR, description: Significant working capital tied up}
  - {min: 91, max: 999, label: CRITICAL, description: Urgent intervention required}
```

DB shape confirms the fields are persisted: `metrics` (graph_id, name, category, unit,
output_type, version, description, output JSON, dependencies JSON) and `metric_parameters`
(name, param_type, default_value JSON, options JSON, description, derivation CHECK IN
('period_grain')) in `/Users/philipp/Code/dataraum/dataraum-context/packages/engine/schema.sql`.

Attempted transcription (§3.1 + §3.4):

```sql
DECLARE METRIC dso
  AS (accounts_receivable / revenue) * days_in_period
  UNIT 'days'
  PARAMETER days_in_period GRAIN month DEFAULT 30      -- ← forced, see below
  BY SEED finance;

DECLARE POLICY interpretation FOR dso
  BANDS (excellent < 31, good < 46, concerning < 61, poor < 91, critical)
  BY SEED finance;
```

Findings, per field:

- Expression, unit, dependency DAG: **TRANSCRIBES CLEANLY.** The concept-space resolution
  rule (§3.1) covers `accounts_receivable`/`revenue` as concepts and `days_in_period` as a
  parameter; the level-1/level-2 DAG is derivable from the expression AST. Good.
- `parameters.days_in_period`: **GRAMMAR GAP.** §3.1's only parameter example is
  `PARAMETER period GRAIN month DEFAULT last_complete` — and §3.1's own status flag admits
  the clause is "a sketch". The real parameter is `type: integer`, `options: [30, 90, 365]`,
  `default: 30`, `derivation: period_grain`. The sketch clause has NO surface for: (a) the
  parameter *type*; (b) the closed *options* list; (c) the `derivation: period_grain`
  coupling (the engine derives 30/90/365 from the query's period grain — the spec's `GRAIN
  month` names a grain *value*, not a derivation *rule*); (d) a plain literal default vs
  the enum-like `last_complete`. Transcribing forces inventing `PARAMETER days_in_period
  TYPE integer OPTIONS (30, 90, 365) DERIVED FROM period_grain DEFAULT 30` — invented
  syntax, which is the finding.
- `interpretation.ranges`: **GRAMMAR GAP + INFORMATION LOST.** §3.4's `BANDS (ok < 45,
  warn < 75, critical)` carries ascending thresholds with open top. It CAN encode the five
  ranges' boundaries (modulo the min/max ↔ strict-< off-by-one: ranges are inclusive
  integer pairs 31–45; BANDS thresholds are exclusive comparisons — representable, but
  only after a semantic translation the spec nowhere states). It CANNOT carry each range's
  `description` ('Very efficient collection', …) — prose per band has no clause. Served
  today to agents as interpretation guidance; nowhere to go. Also the top range's explicit
  max (999) is unrepresentable (final band is unbounded) — minor, arguably spurious data.
- `output.decimal_places: 1`, `output.type: scalar`, `metadata.tags`, `metadata.category`,
  `metadata.name` ('Days Sales Outstanding' — the display name distinct from the
  identifier), `version: '1.0'`: **INFORMATION LOST.** No clauses exist for display name,
  category grouping, tags, output typing, or rendering precision on `DECLARE METRIC`.
  Category/tags feed the cockpit's metric browsing; display name feeds every surface.
- Step-level `validation:` (`condition: 0 <= value <= 365, severity: warning, message: …`):
  **SEMANTICS UNDEFINED.** Closest construct is `DECLARE VALIDATION … KIND constraint`,
  but its OVER contract takes *concepts*, not "the value of metric dso" — the spec never
  says whether a metric's declared name is admissible in OVER, and `KIND` vocabulary
  (balance|comparison|constraint|aggregate) has no range-check-on-metric-output reading
  defined. Parseable as a VALIDATION; meaning unspecified.
- The `aggregation: sum` comment ("period axis … data-reconciled via target_type — never
  hardcode end_of_period"): the *mechanism* (additivity resolution) is §2.6-derived, fine —
  but the extract's `statement: balance_sheet` / `income_statement` axis is per §8.2
  "plausibly just part-of" — i.e. **open, not covered**; today it is part of the snippet
  semantic key (see Artifact 6). GRAMMAR GAP until §8.2 resolves.

Overall: **GRAMMAR GAP** (dominant), with two INFORMATION LOST families and one
SEMANTICS UNDEFINED.

### Artifact 3 — Validation definition

Two real shapes exist. (a) The typed row every producer writes — `validations` table,
`/Users/philipp/Code/dataraum/dataraum-context/packages/engine/schema.sql`:

```sql
CREATE TABLE validations (
	row_id VARCHAR NOT NULL,
	vertical VARCHAR NOT NULL,
	validation_id VARCHAR NOT NULL,
	name VARCHAR NOT NULL,
	description TEXT NOT NULL,
	category VARCHAR NOT NULL,
	severity VARCHAR NOT NULL,
	check_type VARCHAR NOT NULL,
	tolerance FLOAT,
	guidance TEXT,
	expected_outcome TEXT,
	relevant_cycles JSON,
	relevant_conventions JSON,
	tags JSON,
	version VARCHAR NOT NULL,
	source VARCHAR,
	...
	CONSTRAINT ck_validations_check_type CHECK (check_type IN ('aggregate', 'balance', 'comparison', 'constraint'))
);
```

matching `ValidationSpec` in
`/Users/philipp/Code/dataraum/dataraum-context/packages/engine/src/dataraum/analysis/validation/models.py`
(fields: validation_id, name, description, category, severity, check_type — union with
`Literal["expected_formula"]`, tolerance, guidance, expected_outcome,
expected_formula {table, column, formula}, tags, relevant_cycles, relevant_conventions,
version, source). (b) A shipped seed YAML (epic-dat-853 worktree,
`.claude/worktrees/epic-dat-853/packages/dataraum-config/verticals/finance/validations/trial_balance.yaml`),
excerpt:

```yaml
validation_id: trial_balance
name: Trial Balance (Accounting Equation)
description: >
  Validates the expanded accounting equation:
  Assets + Expenses = Liabilities + Equity + Revenue. …
category: financial
severity: critical
version: "1.1"
tags: [accounting, trial-balance, balance-sheet, equation]
relevant_cycles: [journal_entry_cycle, accounts_receivable, accounts_payable]
check_type: balance
parameters:
  tolerance: 0.01
  asset_types: ["asset", "assets"]
  liability_types: ["liability", "liabilities"]
  ...
sql_hints: >
  Join the trial balance table with the chart of accounts … Compute the expanded
  accounting equation: left_side = SUM(net_balance) for asset + expense accounts …
expected_outcome: >
  Total debits must equal total credits across all account types. …
```

(Note: DAT-880 retired `parameters`/`sql_hints` in favor of typed `tolerance` + `guidance`;
the current wire shape is (a).)

Attempted transcription (§3.1):

```sql
DECLARE VALIDATION trial_balance
  KIND balance
  ON CYCLE journal_entry_cycle           -- ← singular; real field is a LIST of three
  OVER (???)                             -- ← see below
  TOLERANCE 0.01
  SEVERITY critical
  GUIDANCE 'Join the trial balance table with the chart of accounts …'
  BY SEED finance;
```

Findings:

- check_type/tolerance/severity/guidance: **TRANSCRIBES CLEANLY** — §3.1's envelope was
  visibly designed off this exact row, and the no-formal-check-expression decision matches
  DAT-735's own direction (guidance prose + derived SQL).
- `OVER` **cannot be filled: GRAMMAR GAP.** §3.1: "OVER is the membership contract: every
  name it lists must resolve in the glossary." The real check's operands are *account-type
  families resolved at bind time* (asset/expense vs liability/equity/revenue) — the
  spec'd example `OVER (receivables, revenue, collections)` presumes concept-shaped
  operands, but trial_balance's membership is "all five account families", which exist as
  concepts only partially (equity yes; "expenses" as a family only via operating_expense
  etc.). Transcribing honestly either fabricates an OVER list that is not what the check
  reads, or omits OVER — and §3.1 gives no optional-OVER form. The membership contract is
  cleanly satisfiable for receivables_roll_forward-shaped checks, not for this one.
- `relevant_cycles: [journal_entry_cycle, accounts_receivable, accounts_payable]` —
  **GRAMMAR GAP.** §3.1 offers `ON CYCLE order_to_cash`, singular. The real field is a
  list, and empty-means-universal is defined semantics today ("empty = universal",
  models.py line 155). No list form, no universal form.
- `relevant_conventions` (typed validation→convention dependency, DAT-865): **GRAMMAR
  GAP.** No clause on DECLARE VALIDATION references declared CONVENTIONs. This is the
  load-bearing half of the convention-routing contract (Artifact 1b): the SQL binder is
  fed exactly these. Nowhere to declare it.
- `category: financial` and `tags`: **INFORMATION LOST** (no clause; same finding as
  metric tags).
- `expected_outcome` (what a pass looks like — prose the SQL binder receives *separately*
  from guidance): **INFORMATION LOST** — GUIDANCE is a single prose slot; the two-field
  split (how to bind vs what passing means) collapses.
- `check_type: expected_formula` + `expected_formula: {table, column, formula}` (the
  DAT-447/880 column-identity teach): **GRAMMAR GAP** in the VALIDATION family — but
  arguably transcribes *outside* it as an aspect application (`DECLARE derived(orders.total,
  formula := 'subtotal + tax') BY USER analyst`) pooled against the `derived_value`
  witness. The spec's §8.3 explicitly lists expectation teaches as having "no statement
  form yet", so by the spec's own accounting this is an admitted open gap.
- `version: "1.1"`: supersession replaces versioning — arguably covered by design
  (re-declare supersedes). Not counted as loss.

Overall: **GRAMMAR GAP** (OVER membership vs family-resolved operands; cycle-scope list;
relevant_conventions), plus INFORMATION LOST (category/tags/expected_outcome).

### Artifact 4 — Business-cycle definition (stages, status column, completion semantics)

Source (vocabulary side):
`/Users/philipp/Code/dataraum/dataraum-context/packages/dataraum-config/verticals/finance/cycles.yaml`:

```yaml
cycle_types:
  accounts_receivable:
    description: "AR collection cycle: customer invoices settled by INCOMING flows from
      the counterparty — the counterparty is a customer who owes us; the counterparty
      code axis on the invoice fact carries the direction"
    business_value: high
    aliases: [ar_cycle, receivables_cycle, collection_cycle]
    typical_stages:
      - {name: "Invoice Created",  order: 1, indicators: [created, new, open, issued]}
      - {name: "Invoice Sent",     order: 2, indicators: [sent, delivered, notified]}
      - {name: "Payment Due",      order: 3, indicators: [due, outstanding, pending]}
      - {name: "Payment Received", order: 4, indicators: [paid, received, collected, cleared]}
    completion_indicators: [paid, collected, cleared, closed]
    feeds_into: [journal_entry_cycle]

cycle_families:
  settlement:
    directions:
      incoming: accounts_receivable
      outgoing: accounts_payable
```

Source (detected/asserted side): `detected_business_cycles` in
`/Users/philipp/Code/dataraum/dataraum-context/packages/engine/schema.sql` — fields:
cycle_name, cycle_type, canonical_type, is_known_type, family, direction, description,
business_value, confidence, tables_involved JSON, stages JSON, entity_flows JSON,
status_table, status_column, completion_value, total_records, completed_cycles,
completion_rate, evidence JSON, plus CHECK ((family IS NULL AND direction IS NULL) OR
(family IS NOT NULL AND direction IS NOT NULL)).

Attempted transcription. The spec deliberately offers NO `DECLARE CYCLE` body — §2.1 maps
"business cycles (LLM)" to `DECLARE CYCLE`, but §3 contains no such statement; §8.4
prescribes decomposition instead. Following §8.4's own recipe:

```sql
-- stage concepts in a pack:
DECLARE CONCEPT invoice_created KIND ??? BY SEED finance;   -- no cycle-stage KIND exists
DECLARE RELATIONSHIP invoice_created PART OF accounts_receivable BY SEED finance;
-- a stage aspect binding the status column's values:
DECLARE ASPECT ar_stage VALUES (created, sent, due, paid) BY SEED finance;
DECLARE ar_stage(invoices.status, value := ???) BY AGENT cycles;
-- completion-semantics validations:
DECLARE VALIDATION ar_completion KIND aggregate ON CYCLE ??? … BY AGENT inductor;
DECLARE CYCLE FAMILY settlement DIRECTIONS (incoming, outgoing) BY SEED finance;
```

Findings:

- `DECLARE CYCLE` **does not exist in §3 at all** while §2.1 promises it: the map row
  "business cycles (LLM) → `DECLARE CYCLE`" points at a statement the grammar chapter
  never defines. **GRAMMAR GAP by the spec's own §2 completeness rule.**
- §8.4's decomposition, attempted honestly, fails on four fields:
  1. **Stage ORDER.** `order: 1..4` is load-bearing (stage progression, stuck-cycle
     analysis). `PART OF` edges are unordered; ASPECT VALUES lists are label sets with no
     declared ordering (only *dimension concepts* may declare ORDERING, per §3.1 comment —
     stages are not dimension concepts). No construct carries "Invoice Sent comes after
     Invoice Created". **GRAMMAR GAP.**
  2. **Per-stage indicator lists** (`indicators: [sent, delivered, notified]` — the
     value→stage binding). An aspect application binds a *column* to a *value*; binding
     *multiple status values* to *one stage* per column needs one statement per (value,
     stage) pair with the value as argument — expressible only by inventing an argument
     convention (`DECLARE ar_stage(invoices.status, token := 'delivered', value :=
     invoice_sent)`), i.e. null_token's pattern borrowed without a spec statement that
     stage aspects take arguments. **SEMANTICS UNDEFINED** (the aspect model *could* carry
     it; nothing says so, and the ASPECT declaration grammar shows no argument
     declarations beyond VALUES).
  3. **`completion_value` / completion_indicators.** "This status value means the cycle
     is complete" is neither a validation (it is a *semantics assertion*, not a check) nor
     a stage aspect value property. §8.4 says "completion-semantics validations" — but a
     validation judges data, and completion semantics must first be *declared* for the
     completion-rate measurement to exist. Circular; no construct. **GRAMMAR GAP.**
  4. **`feeds_into: [journal_entry_cycle]`.** §3.1 explicitly excludes it: "Domain edges
     (feeds-into, stage order) stay pack vocabulary: no operator without a named
     mechanism." But pack vocabulary IS declarations — and no declaration form can carry
     an edge between two cycle concepts except the three closed operators. The spec bans
     the edge and offers no home; today it is typed, seeded data (`cycle_types.feeds_into`
     JSON). **INFORMATION LOST, by explicit design decision** — the §9.1 test ("confirms
     the decomposition covers stages, status column, and completion semantics without
     workaround") currently FAILS on 1–3.
- `cycle_families` → `DECLARE CYCLE FAMILY settlement DIRECTIONS (incoming, outgoing)`:
  **GRAMMAR GAP (spec-admitted).** §8.4 itself flags it: "Real cycle-family directions
  bind *concepts* (`incoming accounts_receivable`), not bare labels as §3.1 sketches."
  The real artifact is direction→member-cycle *mappings*; the sketch carries bare labels.
  Confirmed against the artifact: `incoming: accounts_receivable` is a pair.
- Detected-cycle instance fields (confidence, stages-as-bound, status_table/status_column,
  completion_rate, evidence): the *assertion* half (status column binding, direction) maps
  to aspect applications `BY AGENT … CONFIDENCE …` + WITNESS/evidence; completion_rate and
  total_records are derived — correctly excluded per §2.6. `is_known_type` /
  `canonical_type` (alias resolution) have no home: aliases (`aliases: [ar_cycle, …]`) are
  §6 *reserved* (Synonyms). **INFORMATION LOST (reserved, acknowledged).**

Overall: **GRAMMAR GAP** (the heaviest of the eight — the §8.4 decomposition does not yet
cover stage order, value→stage binding, or completion semantics; `DECLARE CYCLE` is
promised in §2 and absent in §3).

### Artifact 5 — Claim-witness row shape + one real detector's output

Source: `claim_witnesses` in
`/Users/philipp/Code/dataraum/dataraum-context/packages/engine/schema.sql`:

```sql
CREATE TABLE claim_witnesses (
	claim_witness_id VARCHAR NOT NULL,
	table_id VARCHAR,
	column_id VARCHAR,
	run_id VARCHAR NOT NULL,
	target VARCHAR NOT NULL,
	claim_field VARCHAR NOT NULL,
	witness_id VARCHAR NOT NULL,
	distribution JSONB,
	reliability FLOAT NOT NULL,
	detector_id VARCHAR NOT NULL,
	computed_at TIMESTAMP WITHOUT TIME ZONE NOT NULL,
	...
	CONSTRAINT uq_claim_witness_target_field_witness_run UNIQUE (target, claim_field, witness_id, run_id)
);
```

In-memory model (`/Users/philipp/Code/dataraum/dataraum-context/packages/engine/src/dataraum/entropy/models.py`):

```python
class WitnessClaim:
    claim_field: str       # claim-slot identity, e.g. "null_token:TBD"
    witness_id: str
    distribution: dict[str, float]
    reliability: float
```

Real detector: `temporal_behavior`
(`/Users/philipp/Code/dataraum/dataraum-context/packages/engine/src/dataraum/entropy/detectors/computational/temporal_behavior.py`,
measurement `entropy/measurements/temporal_behavior.py`): `CLAIM_SPACE = ("stock", "flow")`;
pools TWO witnesses per column — `llm_claim` (r=0.838) and `structural_reconciliation`
(r=0.889), values from
`/Users/philipp/Code/dataraum/dataraum-context/packages/dataraum-config/entropy/reliabilities.yaml`:

```yaml
witnesses:
  null_semantics:
    quarantine_clustering: 0.8681
    type_claim: 0.2658
    null_vocabulary: 0.944
  temporal_behavior:
    llm_claim: 0.838
    structural_reconciliation: 0.889
```

Attempted transcription (§3.3, §3.4):

```sql
WITNESS behavior(orders.amount, stock := 0.11, flow := 0.89)
  BY DETECTOR temporal_behavior
  EVIDENCE 'obs://run-342/temporal_behavior/orders.amount';

DECLARE RELIABILITY DETECTOR temporal_behavior FOR behavior 0.838 BY CALIBRATION '2026-06-10';
```

Findings:

- The witness statement itself: **TRANSCRIBES CLEANLY** — target→subject,
  claim_field→(aspect, argument), distribution→labelled args, detector_id→BY DETECTOR,
  run_id→opaque inside EVIDENCE ref (§3.3 sanctions this). The claim-slot encoding
  `"null_token:TBD"` maps to the argumented form `null_token(col, token := 'TBD', …)`. Good.
- **GRAMMAR GAP — the witness/detector distinction is collapsed.** The real reliability
  key is (measurement, WITNESS) — one detector pools SEVERAL witnesses with DIFFERENT
  reliabilities (`temporal_behavior` has two; `null_semantics` has three:
  quarantine_clustering 0.8681, type_claim 0.2658, null_vocabulary 0.944). glossql's
  `DECLARE RELIABILITY DETECTOR x FOR aspect r` keys reliability by (actor, aspect) — ONE
  number per detector per aspect. Transcribing null_semantics forces either (a) three
  detector actors (`BY DETECTOR null_vocabulary` etc.) — losing the fact that one
  measurement pools them and inflating the actor roster, or (b) one pooled r — losing the
  calibrated per-witness weights entirely. The row's `witness_id` column (distinct from
  `detector_id`, part of the UNIQUE key) has NO glossql counterpart. This is a structural
  mismatch in the load-bearing novelty (§4), not a clause detail.
- **INFORMATION LOST — calibration provenance.** reliabilities.yaml carries per-measurement
  `calibrated: true/false`, corpus_version, estimator, per_class_accuracy
  (sensitivity/specificity), pooled_brier_holdout, sample sizes, date, and stance notes.
  `BY CALIBRATION '2026-07'` carries a name. Everything else (notably the
  calibrated-vs-placeholder flag, which consumers read via
  `ReliabilityConfig.calibrated_for(id)`) has nowhere to go.
- **SEMANTICS UNDEFINED — pre-declaration reliability.** The system runs on "placeholder
  priors" before calibration (cross_table_consistency, stored_sign are `calibrated:
  false` today). §3.3 says an undeclared producer pools at "whatever weight the
  reliability policy grants — by default, none." The reliability *policy* (as opposed to
  per-actor RELIABILITY declarations) is never given a statement form in §3.4 — WEIGHT
  appears once inside POLICY readiness with admittedly sketch semantics. How placeholder
  priors are expressed (a RELIABILITY by a non-CALIBRATION actor? a policy default?) is
  undefined.

### Artifact 6 — Grounding / snippet artifact (concept → relation/expression/filter)

Source: `sql_snippets` in
`/Users/philipp/Code/dataraum/dataraum-context/packages/engine/schema.sql`:

```sql
CREATE TABLE sql_snippets (
	snippet_id VARCHAR NOT NULL,
	workspace_id VARCHAR NOT NULL,
	snippet_type VARCHAR NOT NULL,          -- 'extract' | 'constant' | 'formula' | 'query'
	standard_field VARCHAR,
	statement VARCHAR,
	aggregation VARCHAR,
	predicate VARCHAR DEFAULT '' NOT NULL,
	schema_mapping_id VARCHAR NOT NULL,
	parameter_value VARCHAR,
	normalized_expression VARCHAR,
	input_fields JSON,
	sql TEXT NOT NULL,
	description TEXT NOT NULL,
	source VARCHAR NOT NULL,                -- e.g. "graph:dso"
	provenance JSON,
	parts JSON,
	execution_count INTEGER NOT NULL,
	failure_count INTEGER NOT NULL,
	...
	CONSTRAINT uq_snippet_semantic_key UNIQUE (snippet_type, standard_field, statement,
	  aggregation, predicate, schema_mapping_id, parameter_value)
);
```

`parts` (DAT-671, `query/snippet_models.py`): `{select: [{expr, alias}], from: [relation],
where: [pred, …]}` — "the parts ARE the artifact; sql is their one-time render".
`provenance` (healthy rows, `graphs/models.py::HealthySnippetProvenance`):

```
{column_mappings_basis: {concept: {measure_columns[], filter_columns[], filter,
   filter_members: [{column, value}]}},
 assumptions: [{dimension, assumption, basis, confidence}]}
```

plus `FailedSnippetProvenance {failure_mode ∈ (execution_failed, verifier_rejected,
provenance_invalid, disjoint_collision), failure_reason}` on retained failures.

Attempted transcription (§3.2):

```sql
DECLARE GROUNDING accounts_receivable IN journal_lines_enriched
  AS sum(debit_amount) - sum(credit_amount)
  WHERE account_type IN ('asset') AND account_name LIKE '%receivable%'
  BY AGENT grapher CONFIDENCE 0.9;
```

Findings:

- The core mapping — concept, relation, expression, filter → GROUNDING clauses — is real
  and direct: `standard_field`→concept, parts.from→IN, parts.select→AS, parts.where→WHERE.
  §3.2's claim that columns-used and rendered SQL derive from the AST matches
  parts-at-source exactly. **TRANSCRIBES CLEANLY** for the extract core.
- **GRAMMAR GAP — the supersession key cannot be written.** The real semantic key is
  (snippet_type, standard_field, **statement**, aggregation, **predicate**,
  schema_mapping_id, **parameter_value**). The spec's key is (concept, relation,
  parameter). Confirmed collision: dso needs `accounts_receivable` extracted from
  statement `balance_sheet` while a P&L metric needs `revenue` from `income_statement` —
  two groundings of different *statement axes* over the SAME relation. The spec's key has
  no statement member (§8.2 punts it to part-of), and — worse — its *parameter* member has
  no surface syntax anywhere in the GROUNDING statement (Part A item 5). The real
  `parameter_value` (e.g. days_in_period=90 variants of constant snippets) and `predicate`
  (DAT-838: same field+statement+aggregation, different row restriction = different
  measurement) are both key members with no clause.
- **INFORMATION LOST — assumptions.** `assumptions: [{dimension: 'period.binding',
  assumption, basis, confidence}]` is authored-by-agent judgment that feeds the DAT-631
  confidence gate (a metric assembled from cache surfaces its weakest grounding's
  confidence). CONFIDENCE on the statement is one number; the per-assumption records
  (each with its own dimension, basis, confidence) have nowhere to go, and §3.0 explicitly
  makes CONFIDENCE non-adjudicating metadata — while today's confidence gate is a
  *consumer mechanism* over these records.
- **INFORMATION LOST — retained failures.** `FailedSnippetProvenance` rows (including
  `disjoint_collision` — the cross-concept guard that no per-statement check can reach)
  are *negative knowledge* fed back into `_build_prior_context` so the agent does not
  re-author a rejected grounding. glossql has negative declarations for RELATIONSHIP
  (`REJECTED`) but no failed/rejected GROUNDING form; a failure is not an OBSERVE result
  (it is authored by the guard) and not a witness (no claim space). No home.
- `execution_count`/`failure_count` (usage tracking): derived state, correctly excluded
  (§2.6). `filter_members` for dimension-member edges: derivable from the WHERE AST as
  §3.2 claims. Fine.

Overall: **GRAMMAR GAP** (key mismatch: statement/predicate/parameter_value members
unwritable) + **INFORMATION LOST** (assumption records; retained failures).

### Artifact 7 — Teach payload (config_overlay rows)

Source: `/Users/philipp/Code/dataraum/dataraum-context/packages/cockpit/src/tools/teach.validation.ts`.
The live teach-type roster (TYPE_SCHEMAS): `type_pattern`, `null_value`, `unit`,
`relationship`, `hierarchy`, `validation`, `cycle`, `metric` (8; the former `concept` trio
was retired by DAT-728 to a typed table write). Chosen payload — `type_pattern` (an
AGENT_AUTOAPPLY type, so it is also the constrained-decoding surface §2.5 claims):

```ts
const TypePatternPayload = z.object({
  name: z.string().min(1),          // 'eu_date'
  pattern: z.string().min(1),       // '^\\d{2}\\.\\d{2}\\.\\d{4}$'
  inferred_type: z.string().optional(),      // 'DATE', 'DECIMAL'
  semantic_type: z.string().optional(),      // 'currency', 'percentage'
  detected_unit: z.string().optional(),      // 'EUR', 'kg'
  case_sensitive: z.boolean().optional(),
  standardization_expr: z.string().optional(),  // SQL normalizing a match
}).passthrough();
```

Also quoted for contrast, `relationship` (maps cleanly):
`{action: confirm|reject|add, from_column_id, to_column_id}` and `null_value`:
`{category ∈ (standard_nulls, spreadsheet_nulls, placeholder_nulls, missing_indicators),
value, description?}`.

Attempted transcriptions:

- `relationship` teach → **TRANSCRIBES CLEANLY** (modulo skeleton, Part A item 1):
  confirm → `DECLARE RELATIONSHIP a.x REFERENCES b.y … BY USER analyst;`
  reject → the same pair with `REJECTED`; add → a plain declaration. The
  column-id-vs-column-name identity difference is absorbed by §1.2(5) (structural paths).
- `hierarchy` teach (`{action: add|reject|alias, table_id, members[]}`) → add maps to
  `DECLARE HIERARCHY geo IN customers LEVELS (country > region > city) KIND drilldown BY
  USER analyst;`. **GRAMMAR GAP (small):** there is no REJECTED form on HIERARCHY — §3.2
  defines the negative declaration only for RELATIONSHIP; rejecting a discovered
  drill-down (a real, shipped action) has no statement. `alias` maps to KIND alias per
  §2.1's kinds; the artifact's `canonicalLabel`/needs_confirmation halves are derived —
  fine.
- `null_value` teach → §3.2's `DECLARE null_token(orders.amount, token := 'n/a', value :=
  is_null) BY USER analyst;` — but the REAL teach is **workspace-scoped, not
  column-scoped** ("the overlay vocabulary is workspace-scoped", teach.ts DAT-506), and
  carries a `category` axis. §3.1's comment says workspace-scoped null tokens land with
  §8.3 — i.e. **GRAMMAR GAP, spec-admitted** ("typing/null/expectation teaches (§8.3)
  have no statement form yet"). The `category` field has no home in the sketched
  column-scoped form either.
- `type_pattern` teach → **GRAMMAR GAP, spec-admitted (§8.3).** No statement form exists.
  Honest attempt fails immediately: it is workspace-scoped vocabulary (subject =
  workspace), its payload is 7 typed fields including an executable
  `standardization_expr` (SQL — a transported body, needing the recipe-string posture),
  and nothing in §3 hosts it. §8.3's direction ("type patterns are expressions and likely
  ride an existing head") remains unrealized.
- `unit` teach (`{table, column, unit}`) → `DECLARE unit(orders.amount, value := 'EUR')
  BY USER analyst;` — **TRANSCRIBES CLEANLY.**
- `validation`/`cycle`/`metric` teaches → same statements as artifacts 2–4 with `BY USER`
  (§3.2: "a human teach is any of these statements with BY USER") — they inherit every
  gap found there; nothing additional.

Overall: **GRAMMAR GAP** — 2 of the 8 live teach types (type_pattern, null_value-as-shipped)
have no statement form, both acknowledged under §8.3; 1 small unacknowledged gap
(hierarchy reject). 3 transcribe cleanly; 3 inherit other artifacts' verdicts.

### Artifact 8 — The curated answer-agent context (query-context.ts) vs GLOSS + DECLARE SERVING

Source: `/Users/philipp/Code/dataraum/dataraum-context/packages/cockpit/src/tools/query-context.ts`.
Blocks actually served (each a build*Block function): `<schema>` (prefer-enriched tables,
columns with types + `[meaning:]` tags + `(additive)/(point_in_time)` verdicts),
`<dimensions>` (judged slice axes with value_count, measured relevance, interest tier,
column ids, alias/drill-down hierarchies, curation disclosure notes), `<relationships>`
(confirmed join predicates + cardinality + fan-out caution + omitted-count disclosure),
`<entities>` (entity type, table_role, grain, event-time axes with anchor, identities with
notes), `<drivers>` (ranked dimensions with gain, drill paths, notable slices with
effect/support, other-grain drivers), `<grain>` (near-unique columns, NEAR_UNIQUE_RATIO
0.9), `<business_concepts>` (concept graph + groundings). Conventions ride a separate
block outside this file.

Could `GLOSS (query) USING SERVING answer_agent` reproduce it? Per-block:

| Served content | Authored statement that could produce it | Verdict |
|---|---|---|
| table inventory, prefer-enriched cut | DECLARE TABLE/VIEW + `PREFER enriched` (§3.4) | covered |
| column meanings | `DECLARE meaning(col, …)` | covered |
| stock/flow markers | declaration ⊕ pooled posterior (derived) | covered |
| join whitelist + "never invent a join" rule | DECLARE RELATIONSHIP + `RESTRICT JOINS TO DECLARED RELATIONSHIPS` | covered |
| dimensions + priority/interest | `DECLARE dimension(col, priority := …)` + `DIMENSION BUDGET` | covered |
| hierarchies | DECLARE HIERARCHY | covered |
| entity/grain/time/identity | entity/role/grain/time_axis aspects (§3.2) | covered |
| conventions | DECLARE CONVENTION + INCLUDE (conventions) | covered (minus targets, Art. 1b) |
| drivers, readiness | derived (OBSERVE drivers; §2.6) | covered as derived |

Served fields with **NO authored statement that could produce them** (the question asked):

1. **The instructional prose of every block.** Each block is ~40% imperative teaching
   text authored by engineers: "Ground EVERY join on a pair listed here; if the join you
   need isn't listed, do not invent one — abstain or state the limitation", "(additive) is
   a flow: SUM it across ALL periods…", "Never guess a value or match by substring", the
   grain block's per-row-dump warning. This is not derived state and not a declaration —
   it is serving-policy *rendering prose*. §3.4's SERVING clauses (PREFER, DIMENSION
   BUDGET, RESTRICT, INCLUDE) select *content*; nothing authors the words. §7 excludes
   "prompt configuration" from the language — so under the spec's own rules the largest
   authored surface of the real served context is engine config, unreproducible from the
   log. Defensible, but then `GLOSS` output is NOT f(log, lake): it is f(log, lake,
   renderer-prose).
2. **Curation-disclosure arithmetic as contract.** "Showing 9 of 41 catalogued dimensions
   — the other 32 … were never assessed, not axes assessed and rejected", the
   omitted-relationships count + reason, the unjudged-fallback branch and its
   UNJUDGED_FALLBACK_MAX=25. Derivable-in-principle from budget policy + state, but the
   spec's SERVING clause list has no disclosure mechanism, and DAT-622/671's finding is
   that *silence converts to false abstention* — i.e. disclosure is load-bearing serving
   semantics, currently unspecifiable. SEMANTICS UNDEFINED for `DECLARE SERVING`.
3. **Alias confirmation gate** (`needs_confirmation === false` → "group by the canonical
   only"; unconfirmed → "do NOT merge"): the confirmed/unconfirmed axis of a HIERARCHY is
   adjudication state (declaration vs judged), fine — but the *serving rule* keyed to it
   (collapse vs never-collapse) is policy with no clause.
4. **look_values grounding loop** (`[id: …]` handles + "drill the COMPLETE value-set via
   look_values"): serves engine-internal column ids so a *tool* can be called. Ids are
   §1.2(5)-banned surrogate identity; the served artifact depends on them. The spec would
   serve paths — plausible, but the value-set-grounding contract ("never guess a value")
   spans serving + tools and has no statement.
5. **Fan-out caution** (`evidence.introduces_duplicates`): observation-derived, covered
   as derived state — but only if the `overlap`/relationship measurement's result schema
   includes it; §3.5 leaves all relation schemas unspecified (flagged in the spec's own
   status list).

Also confirmed from the fieldwork side: the file demonstrates the spec's own §3.5 claim
(byte-identical curated block per session, one serving policy, no per-call knobs) — the
GLOSS/SERVING *architecture* fits the running system's shape. The gaps are in what SERVING
can express, not in the one-mechanism bet.

Overall: **SEMANTICS UNDEFINED** (serving clause list is a sketch by the spec's own flag;
disclosure, alias-gate, and rendering prose have no clauses) + **INFORMATION LOST**
(instructional prose unproducible from any authored statement — excluded by §7 as prompt
config, which contradicts §10's "a cockpit feature that cannot be written this way is a
grammar gap" for the answer surface).

---

## Part C — Final tally

Artifacts attempted: **9** (8 requested + convention 1b, counted separately since it
carries its own verdict).

| # | Artifact | Verdict |
|---|---|---|
| 1 | concept `revenue` (finance pack) | TRANSCRIBES CLEANLY (pack version + composition-edge skeleton issue noted) |
| 1b | convention `sign_natural_balance` | GRAMMAR GAP (targets, concept_groups) |
| 2 | metric `dso` | GRAMMAR GAP (parameter clause, interpretation descriptions) + INFORMATION LOST (name/category/tags/decimal_places) + SEMANTICS UNDEFINED (step validation) |
| 3 | validation `trial_balance` / ValidationSpec | GRAMMAR GAP (OVER vs family operands; cycle list; relevant_conventions) + INFORMATION LOST (category/tags/expected_outcome) |
| 4 | cycle `accounts_receivable` + family `settlement` | GRAMMAR GAP (stage order, completion semantics, value→stage binding, DECLARE CYCLE promised-not-defined; family directions spec-admitted) + INFORMATION LOST (feeds_into, aliases) |
| 5 | claim_witnesses + temporal_behavior/null_semantics reliabilities | CLEAN witness statement; GRAMMAR GAP (per-witness reliability — witness_id has no counterpart) + INFORMATION LOST (calibration provenance) + SEMANTICS UNDEFINED (placeholder priors) |
| 6 | sql_snippets grounding | CLEAN core; GRAMMAR GAP (semantic key: statement/predicate/parameter members unwritable) + INFORMATION LOST (assumptions, retained failures) |
| 7 | teach payloads (8 types) | 3 CLEAN (relationship, unit, hierarchy-add); GRAMMAR GAP (type_pattern + null_value — §8.3-admitted; hierarchy reject — unadmitted) |
| 8 | query-context.ts served context | SEMANTICS UNDEFINED (serving clauses) + INFORMATION LOST (instructional prose; §7 exclusion vs §10 cockpit rule) |

**Tally: 9 attempted · 1 fully clean (Artifact 1) · 7 with grammar gaps · 3 with
undefined semantics · 6 with information loss.** (Categories overlap; per-field detail
above.)

Skeleton coverage (Part A): **7 of the spec's own statement families are not derivable
from §3.0** — keyed classes (RELATIONSHIP, GROUNDING key, RELIABILITY), POLICY's three
key shapes, CYCLE FAMILY's two-token class, the undeclared `calendar` aspect, and the
entirely missing `observation` and `lifecycle` productions.

Sharpest single findings, ranked by structural weight:
1. The reliability model collapses witness_id into detector_id — the real system's
   calibrated per-witness weights (three per null_semantics) cannot be declared (Art. 5).
2. The GROUNDING supersession key names a parameter member that has no syntax, and omits
   the statement/predicate members the running system's uniqueness constraint depends on
   (Art. 6, Part A item 5).
3. §8.4's cycle decomposition fails its own §9.1 acceptance test on stage order,
   value→stage binding, and completion semantics (Art. 4).
4. §3.0's skeleton derives none of its keyed-class examples; `clauses`, `observation`,
   `lifecycle` are undefined nonterminals (Part A).
5. The served context's instructional prose is excluded by §7 (prompt config) while §10
   declares any cockpit surface not writable as statements to be a grammar gap — the two
   rules contradict on the system's single most-consumed artifact (Art. 8).
