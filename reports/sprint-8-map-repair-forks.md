# Sprint 8 · §2 map repair — decision forks + mechanical corrections

**DECIDED 2026-07-30: Fork 1 = A (dropped by redesign), Fork 2 = A (reserve)**
(project lead). Applied: §2.1 rows (type decisions, bus matrix), §2.2 rows
(`reconciliation`, 18 detectors), §2.4 nine-block row, §2.5 actor
vocabularies, §2.6 current-state surface, §3.0 standard_field phrasing, §3.4
constants, §3.5 element-view precision dropped, §6 (bus matrix reserved;
analysis_hints acknowledgment), §7 (lifecycle states, detector calibration,
application state), §10 scoped to context surfaces. All nine mechanical
corrections applied.

The fact-check (`reports/2026-07-30-adversarial-review.md`, Part II-F) found §2
numerically unreliable and missing an entire axis plus artifact families. Two
findings need a design decision; the rest are truth repairs applied with this
sprint.

## Fork set 1 — The run/promotion axis

Today: `metadata_snapshot_head` (per-(target, stage) promoted run; every
`current_*` view joins through it), `lifecycle_artifacts` (four states),
`type_decisions` (measured candidates vs resolved decision), `resolve.py`
write-backs. §2 never lists any of it.

### A — Dropped by redesign; map each mechanism — recommended

The axis exists to answer "which run's rows are current." glossql's core
invariant answers it differently: current = the active (unsuperseded)
statements, state = f(log, lake). Mapping, added as §2 rows:

- promoted heads / `current_*` views → replay + supersession define "current";
  runs are engine-internal (§3.3 already says so).
- `type_decisions` → a typing aspect application (`DECLARE type(orders.amount,
  value := 'DECIMAL') BY AGENT typing` / `BY USER …`); candidates stay
  measurements.
- resolve write-backs → already redesigned in §5 (acceptance is a new DECLARE).
- `lifecycle_artifacts` states/strictness → orchestration (§7); its `teaches`
  payload → ordinary statements.

### B — Reserve a promotion statement family

Keeps the gate authorable ("promote run R for stage S"). But it reintroduces a
second notion of currency beside supersession — the exact two-sources-of-truth
shape the log/lake split exists to kill.

## Fork set 2 — Bus matrix / conformed dimensions

Today: `bus_matrix` (attachment folded/referenced, conformed_group, roles,
attributes, confirmation_source) — served, graphed, tooled; absent from §2/§3.

### A — Reserve explicitly — recommended

A §6 row + §2.1 row marked reserved. Conformance is authored/judged (it has a
confirmation half), cross-fact and n-ary — it deserves a designed statement,
not a shoehorn; nothing in v0's slice needs it.

### B — Aspect family now

`DECLARE conformed(orders.customer_id, group := customer, …)` — n-ary
cross-fact structure forced through a per-subject claim slot.

### C — Derive from relationships

Loses the judged/confirmed half (confirmation_source is authored state).

## Mechanical corrections (applied with this sprint, no fork)

1. §2.2: "17 entropy detectors" → 18.
2. §2.4: six-block context → nine blocks (add `business_concepts` to the row).
3. §2.1 teach row: "all 8 types, today free JSON" → "9 types, Zod-typed per
   payload" (8 registered + direct-read `expected_dependency`).
4. §3.4 fieldwork: "today's hardcoded curation constants" → name only the ones
   still live (prefer-enriched, part-of depth); the dimension budget of 12 is
   already removed upstream.
5. §3.0: "joined by naming convention, as today's `standard_field` strings
   are" → string-equality on a declared concept name (ADR-0024's ~43
   name-keyed joins), not a naming convention.
6. §2.5: the `BY` clause absorbs ≥7 disjoint source vocabularies, not 3.
7. §2.2: add measurement id `reconciliation` (concept_reconciliation results:
   delta, verdict, 7-value abstain vocabulary → derived plane).
8. §7: add exclusions — detector scoring calibration (thresholds.yaml;
   RELIABILITY covers pooling weights only) and application state (cockpit
   reports/conversations/UI) — and scope §10's "the cockpit is a pure
   consumer" claim to context surfaces accordingly.
9. §6 synonyms/agent-instructions rows: note that pack `analysis_hints` prose
   ships today (reserved ≠ nonexistent).
