# Sprint 6 · The PARAMETER clause — decision forks

**DECIDED 2026-07-30: Fork B** (project lead). Applied: SPEC §3.1 example +
PARAMETER prose (+ §10 metric aligned), status flag cleared, grammar + parser
(GRAIN dropped, DEFAULT/OPTIONS/DERIVED BY), fixture 03. Remaining there:
display/browsing metadata (gap block), step-level validation semantics.

Gap (fixture `corpus/03`): the sketch `PARAMETER period GRAIN month DEFAULT
last_complete` has no surface for type, options, or the derivation rule — and
`last_complete` appears nowhere in the running system (invented example).

## Evidence (verified 2026-07-30)

- The entire parameter population of the finance vertical is ONE shape:
  `days_in_period — type: integer, default: 30, options: [30, 90, 365],
  derivation: period_grain` — in exactly the 4 working-capital metrics.
- `MetricParameterDerivation` (`metric_graph_db_models.py:54`) has exactly one
  member, `PERIOD_GRAIN`, documented as "a DECLARED, PROJECTED marker — the
  typed side of the declared-default ↔ observed-override split … NOT itself a
  runtime dispatch key". The rule computes an override from data when the
  caller provides none; the declared default stands otherwise.
- `metric_parameters` is the runtime authority for the declared default
  (`GraphAgent._resolve_parameters` reads it from the table).
- Derivation names an engine **mechanism** from a closed vocabulary — exactly
  the pattern §3.0 already has for policies ("mechanisms are finite and
  spec-defined; vocabulary is not").

## Fork A — Full mirror

```sql
PARAMETER days_in_period TYPE integer DEFAULT 30 OPTIONS (30, 90, 365)
  DERIVED BY period_grain
```

Five sub-clauses for a family with one real instance; `TYPE` duplicates what
the `DEFAULT` literal already shows.

## Fork B — Lean: literal-typed, mechanism-named — recommended

```sql
DECLARE METRIC dso
  AS 90 * avg(accounts_receivable) / sum(revenue)
  UNIT 'days'
  PARAMETER days_in_period DEFAULT 30 OPTIONS (30, 90, 365) DERIVED BY period_grain
  BY SEED finance;
```

- No `TYPE`: the default and options literals type the parameter under
  substrate SQL rules (the same posture as everywhere else — ride SQL).
- `OPTIONS` is the closed value set (ASPECT `VALUES`' pattern, applied to a
  parameter).
- `DERIVED BY` names a mechanism from a spec-defined closed list (today:
  `period_grain`) — §1.2(6) satisfied: the grammar enumerates mechanisms,
  never domains. Absent ⇒ plain constant.
- `GRAIN month` disappears — grain was never a parameter property; the
  derivation rule owns the grain ladder.

## Fork C — Name + default only

Options and derivation dropped. Loses the declared-default ↔ observed-override
split, which is persisted, projected structure today. Too lossy.

## Recommendation

**B.** Matches the one real instance losslessly, adds no fictional surface,
and reuses two existing spec patterns (closed value sets, named mechanisms).
