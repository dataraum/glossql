# Begin-session analysis — what ports, what doesn't

Date: 2026-08-05. The project lead's ask: analyze v0.3's begin-session
stage; say what is well done and should be ported, what not. Everything
that ports is functions + skills — wipeable library content, never
server surface. The criterion is the product goal, not v0.3 coverage:
a lane is worth porting exactly insofar as it carries an agent from
landed tables to **correct metric definitions in minutes** — the
operating-model phase (metric, cycles, validation) runs on top of what
this stage produces.

Grounded by a four-way sweep of `../dataraum-context` (execution map,
measurement internals, every LLM judge with its prompt and schema, and
a consumption trace of every output). Module paths below are under
`packages/engine/src/dataraum/` unless noted.

## The stage map, dispositioned

| v0.3 stage | verdict | glossql shape |
|---|---|---|
| begin_session_select | absorbed | `USE` + the `imports` relation |
| relationships | **ported** (2026-08-05) | `detect_relationships` + glossql-relationships |
| semantic_per_table | **port** | `entity` FACT aspect + table-verdict rules in glossql-add-source |
| materialize overlays | absorbed | both actor kinds write the same statements; supersession |
| surrogate_mint | drop machinery | carry the hash-cure hazards as composite-cure prose |
| enriched_views | **port** | grain-check + join/column judgment as skill content; `CREATE VIEW` is native |
| slicing | **port** | dimension inventory + relevance measurement; interest judgment in a dimensions skill |
| catalogue_semantics | **port** | judging discipline into glossql-add-source (no new machinery) |
| dimension_hierarchies | **port, simplified** | SQL FD/containment measurement at high recall; the judge replaces the precision apparatus |
| bus_matrix | port later, simplified | structural conform floor as SQL; conform judgment as prose |
| aggregation_lineage | **port** | `behavior_evidence` — the ruled shape (2026-08-05) |
| correlations / derived | port, bounded | formula detection on measure columns; the correlation half is dead in v0.3 itself |
| session_detect | deferred by ruling (2026-08-05) | not evaluated |
| driver_rankings | port later | operating-model territory; the numeric-kernel question arrives here |
| keepers / promote | absorbed | supersession + cache; no heads, no promote |

## What is well done — the parts worth carrying

**The stock/flow discriminator** (`analysis/lineage/reconcile.py`) is
the strongest single statistic in the stage. Two competing hypotheses
scored as residuals against independently aggregated period movements
from the event table — flow: the column *is* the period movement;
stock: its delta is — with abstention gates that are derived, not
tuned: a wrong-anchor gate (median residual ≈ 1.0 on a wrong anchor
vs ≈ 0.0–0.1 on a right one) and a near-tie separation gate coupled
to it algebraically, plus entity voting (≥2 voters, ≥0.8 agreement).
The recorded falsification behind it matters more than the formula:
a column's own trajectory cannot decide stock vs flow — a trending
flow and a mean-reverting stock look alike — so the evidence must be
cross-table period movement. That defines `behavior_evidence`'s
required shape, and the computation is per-period sums plus dict
arithmetic: SQL through the script door, no numeric library.

**The slicing relevance score** (`analysis/slicing/relevance.py`):
coverage × evenness (Pielou index over the profiler's buckets, unseen
tail as one bucket). Zero free parameters — the module says so and
means it — and its history records why the alternatives failed (a
normalized-perplexity variant scored a 99/1 boolean at 0.53). The
inventory gates beside it are equally clean: distinct ≥ 2 (NULL is a
bucket), null ratio ≤ 0.5, near-key as a *fraction* ceiling (0.9) —
the absolute-count version was a recorded bug.

**The grain check** (`pipeline/phases/enriched_views_phase.py`): one
`COUNT(*)` probe per candidate join, kept iff it equals the fact's
row count; a one-hop star makes the probes independent, so no
bisection. It is the cheapest possible verification of the most
consequential property a view has. Ports as skill prose — it is one
query the judging agent runs before `CREATE VIEW`, not a function.

**The judge discipline.** Every v0.3 prompt file carries a changelog,
and nearly every recorded change is a removal: fields elicited and
discarded, echoes of measured numbers the model was asked to re-derive,
an ordinal ranking replaced by absolute labels, numeric confidence
gates deleted so the judge sees everything. What remains converged on
one shape across all five lanes: measurements are served as facts
("treat these as authoritative; do not second-guess a measurement"),
the model answers only what no statistic can settle, abstention is a
complete answer ("a hedged claim is gradeable, a withheld one is
not"), and a failed judgment means the stats stand — a judgment field
is never filled deterministically ("a column name is not a concept
label"). This is skill prose for us, and it is the most portable
thing in the stage.

**The hierarchy stack** (`analysis/hierarchies/`) is statistically
serious — functional-dependency screens with permutation nulls,
false-discovery control, a pre-registered vacuousness floor, role
verdicts validated on adversarial data. But its precision apparatus
exists to compensate for judge-less operation: v0.3 had to control
false positives deterministically because nothing downstream would.
Our economics are inverted — the measurement's job is recall, the
judge removes false positives against the data. So the port is the
cheap SQL core (pairwise `GROUP BY` FD/containment checks, near-key
and distinct-count guards), not the permutation machinery, and the
two identity lessons become judge prose: a perfect 1:1 can be a
code↔label alias or a coincidence, and only meaning separates them;
same-family role columns (an origin and a destination) must stay
apart however cleanly they align — the false merge silently corrupts
every cross-fact aggregation.

**The formula detector** (`analysis/correlation/within_table/`):
target ≈ a op b within an absolute tolerance, match rate ≥ 0.8,
zero-target rows excluded (a recorded false-positive source). For the
product goal this is not a data-quality nicety — a detected
`gross_profit = revenue − cogs` *is* a metric candidate. v0.3 runs it
O(n³) with a full scan per candidate; the port bounds it to
measure-role columns and lets the agent direct it.

## What not to port

**The pipeline itself.** Fifteen stages in a fixed order, run
versioning, sticky-shape inheritance, overlay materialization, keeper
lifts, head promotion — this is the largest complexity mass in the
stage, and the language already absorbed all of it: statements in any
order, cache + cache deletion for recomputation, supersession for
versioning, both actor kinds writing the same statements (fixture 12
recorded this before the sweep confirmed it). The ordering
constraints the workflow enforces become one paragraph of skill
guidance about what to read before judging what.

**The serving aggregators.** v0.3 has two: the cockpit's context
assembly and a property graph that folds eight outputs into one
surface. Both are the serving layer the 2026-08-03 pivot dropped;
skills over `GLOSSARY()` and the store relations are the replacement
bet, and nothing in the sweep argues the bet is losing.

**The surrogate mint.** Recipe rewrapping, column reconciliation,
freeze semantics — machinery serving v0.3's typed-table substrate.
The decided cure (keyed view, then declare) already covers the
deliverable. What carries is one paragraph of hazards for the cure
itself, since a `||`-concat view key has exactly the hazards the mint
guarded: NULL must propagate (a placeholder false-joins NULL↔NULL),
mixed types render differently ('007' vs '7'), floats render
unstably, and an in-value delimiter can collide.

**Dead weight v0.3 itself abandoned.** Pearson/Spearman correlations
(computed nowhere in the pipeline; "no downstream consumer acts on
them"), Cramér's V and multicollinearity (no caller outside their own
test), a write-only intents table, an orphaned query-context view.
The consumption trace's lesson generalizes: outputs earn their
existence by consumers — we should not declare an aspect nothing will
read.

**The dedicated semantic agents, as agents.** The lead asked whether
the semantic-agent additions (hierarchies, enriched views, catalogue
readings) are unnecessary. The deliverables are necessary — they sit
directly on the metric path. The dedicated per-phase agents are not:
all five LLM lanes are the same actor doing the same thing (read
measurements, judge, write), which in glossql is one agent connection
with skills. The lanes collapse into skill sections; nothing else was
ever in them.

## The port list, in order

1. **`entity`** — a FACT aspect (fixture 12's spelling: value, role,
   grain, time_axis) plus table-verdict rules in glossql-add-source:
   fact vs dimension read from grain and key structure, event vs
   attribute time columns with exactly one anchor, identity columns
   as structural observations. Smallest slice, and everything later
   reads it — a measure without a declared table grain cannot be
   aggregated correctly.
2. **`behavior_evidence`** — the ruled shape: a measurement tying
   each measure column to period movements from related event tables
   (the two-residual discriminator, entity voting, honest
   abstention), read by the agent before it glosses behavior. Correct
   SUM-vs-end-of-period is load-bearing for every metric the
   operating-model phase will define.
3. **The dimensions deliverable** — one slice, three parts sharing a
   skill: the slice inventory + relevance measurement (dataset
   grain — the aspect-grain question flagged in the relationships
   report becomes acute here); the simplified hierarchy/FD
   measurement; enriched-view construction with the grain check and
   the conform/alias judge prose. This is the biggest remaining
   slice and the natural home for RelBench data (rel-f1 first).
4. **`derived_formulas`** — bounded formula detection over
   measure-role columns; candidates are metric hypotheses for the
   operating-model phase.
5. **Drivers** — deferred to the operating-model phase where its
   question ("why did the metric move") lives. This is where numeric
   kernels beyond SQL genuinely become pressing (permutation-gated
   variance reduction); nothing earlier needs them.

After 1–4 the operating-model phase has what it needs underneath:
entities, edges, dimensions, behavior, units, formula candidates —
and `../dataraum-testdata/output/clean/ground_truth.yaml` (monthly
revenue, DSO/DPO, balances, invariants) is a ready oracle for scoring
what the agent defines on top.

## Well built vs brittle — the risk map (added same day)

The lead's follow-up ask: what in v0.3 is built well and brings
value, what is brittle and brings risk — against a stated hesitation
about moving into the statistical field at all. The sweeps answer it,
and the answer is bimodal: **quality concentrates in the
deterministic statistical cores; brittleness concentrates in the
plumbing between them.** The two populations barely overlap.

### Well built — and how we know

The trust evidence is not that the code looks clean; it is that each
core carries its own falsification history and validation record:

- **Falsification-driven design.** Every strong statistic documents
  what was tried and failed, in place: the relevance score records
  the normalized-perplexity variant it replaced and why; the
  reconcile module records that a time-series persistence statistic
  and an LLM trajectory read were both falsified before the
  cross-table design; temporal completeness records the
  unit-mismatch bug that forced calendar counting and *deleted* the
  clamp that had hidden it. Code that remembers its failures is code
  whose thresholds mean something.
- **Pre-registration and validation.** The hierarchy stack's
  vacuousness floor was pre-registered before its benchmark run
  (48 false positives killed, zero truth lost); the gate stack
  scores 32/32 on an adversarial matrix; the role check validated
  9/9 on wild role-playing FK data; the alias-identity merge floor
  sits in a measured bimodal gap (true aliases 0.95–0.98,
  coincidences 0.03–0.10, the middle empty).
- **Determinism discipline, everywhere.** Fixed seeds with derived
  per-pair generators, bottom-k-by-hash sampling (justified twice,
  independently), sorted enumeration behind every streaming argmax.
  Reruns reproduce; flakiness cannot hide.
- **Loud failure.** The grain check fails the run rather than ship a
  fan-out view; drivers persist abstentions as rows ("born loud,
  never silently absent"); no TODO/FIXME anywhere — caveats are
  prose comments at the point of risk.

These are the parts the port list takes.

### Brittle — five patterns, not five bugs

1. **Identity by name across boundaries.** The enrichment lane keys
   table identity on names and its own comment admits a collision
   "aliases silently"; the surrogate column prefix is hand-mirrored
   into the other package as an unchecked cross-package contract.
2. **Shadow authority in aggregators.** The property graph COALESCEs
   the lineage pattern *ahead of* the pooled behavior verdict, so a
   secondary output silently becomes the materialization truth every
   SQL author sees — with a documented abstention-masking caveat.
   Two writers, one surface, no arbiter.
3. **Metadata drift.** The pipeline config describes the hierarchies
   phase as LLM-free while the phase runs two judges; the same
   omission puts those calls under the wrong retry policy despite a
   guard comment written to prevent exactly that; config comments
   cite features that moved phases. The pipeline's description of
   itself is a second document, and it decays.
4. **Invisible abandonment.** A detector registered but executed by
   no phase; an orphaned context view whose consumer was retired; a
   write-only intents table; correlation algorithms with no caller;
   result fields hardwired to zero. The phase shape hides
   unconsumed outputs — nothing forces an output to have a reader.
5. **Undefended assumptions, shipped.** The stored-sign witness
   documents that its central assumption is "load-bearing and
   unverified" and ships `calibrated: false`; the null sentinel's
   collision risk is "accepted as vanishingly rare rather than
   guarded".

### What the map means for the hesitation

The brittleness lives almost entirely in layers the port list
already refuses: the pipeline shell and its self-description (3, 4),
the serving aggregators (2), name-keyed seams between phases (1),
and the adjudication layer that is deferred by ruling (5 sits
there). The well-built cores are what ports — and they port into a
containment the lead already named: functions and skills, wipeable
without disturbing the stack.

One residual risk is real and worth a standing rule: a statistic
ported without its falsification history invites cargo-culting — the
formula survives, the reasons die. So: **no statistic ports without
its oracle.** Every measurement lands with an acceptance test
against ground truth we hold (the finance generator's
`ground_truth.yaml`, RelBench's declared FKs, booksql's known
schema), and carries the recorded falsification that shaped it as
comment or skill prose. That is the corpus-first process applied to
numerics, and it is what turns the hesitation into a gate instead
of a wall.

## Closing the ledger (added 2026-08-06)

The lead's question — are the soft lanes (derived_formulas,
business_cycles) functions at all, or the analysing agent's
judgment? — closed the remaining rows. The dividing principle the
ported planes settled: a **function owns a combinatorial recall
sweep** no agent should hand-run (1,100 column pairs; 121
conventions × 850 entities), and its outputs are refutable by data;
the **agent owns meaning-formed hypotheses** — few, cheap, one probe
query each. Relationships and behavior were sweeps, not a deviation;
the judge kept every verdict.

- **derived_formulas** — deferred until the operating-model target
  pulls it. Once meaning and role glosses exist the formula space is
  hypothesis-shaped (runs 3 and 4 probed `net = debit − credit`
  unprompted, before glossing), and behavior_evidence's conventions
  already discover the cross-table SUM family as reconciliations.
  Its residual value is surprise recall only. When pulled, its scope
  is a fact plus its judged grain-preserving joins — not per table:
  v0.3 ran it cross-table over enriched views, and the views ruling
  makes the judged join the equivalent scan scope.
- **business_cycles** — never a statistic in v0.3 either: the phase
  header says "the engine induces nothing" (declared vocabulary, LLM
  grounding, completion from status columns). Agent territory: a
  `cycle` FACT aspect framed by an operating-model skill, the
  `entity` pattern again. No function.
- **bus_matrix** conform floor — agent-side: reachability over the
  `relationships` relation is two queries, no sweep. Skill prose
  when a multi-fact dataset arrives.
- **drivers** — stands as row 5: the one remaining future function,
  pulled by the operating-model phase.

The stage map above is otherwise landed or absorbed. The
enriched_views row's "`CREATE VIEW` is native" is superseded by the
views ruling (grain-checked joins are the construct — the 2026-08-06
f1 report records it).
