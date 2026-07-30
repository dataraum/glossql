# Sprint 7 · The VALIDATION envelope — decision forks

**DECIDED 2026-07-30: Fork A** (project lead). Applied: SPEC §3.1 example
(ON CYCLES / CONVENTIONS / OUTCOME, prose split from GUIDANCE) + envelope
prose, grammar + parser, fixture 04 (gap block retired). Remaining there:
OVER's family-resolved operands (open sprint candidate); category/tags accepted
as loss.

Gap (fixture `corpus/04`): `ON CYCLE` is singular where `relevant_cycles` is a
list with empty-means-universal; `relevant_conventions` (the typed
validation→convention dependency the SQL binder is fed) has no clause;
`expected_outcome` (what passing *means*, served separately from the binding
guidance) collapses into GUIDANCE; category/tags have no home.

## Evidence (verified 2026-07-30)

- `validation_phase.py:252` errors when a declared `relevant_conventions` id is
  not among served conventions — a membership contract, load-bearing.
- The dependency exists on BOTH sides today: `validations.relevant_conventions`
  and convention `targets: validation:<id>` routing. The load-bearing pull
  direction is validation→conventions.
- `config.py:49`: a spec omitting `relevant_conventions` validates to `[]` —
  absent has defined semantics.

## Fork A — Validation-side ownership, typed clauses — recommended

```sql
DECLARE VALIDATION trial_balance
  KIND balance
  ON CYCLES (journal_entry_cycle, accounts_receivable, accounts_payable)
  CONVENTIONS (sign_natural_balance)
  TOLERANCE 0.01
  SEVERITY critical
  GUIDANCE 'Join the trial balance table with the chart of accounts …'
  OUTCOME 'Total debits must equal total credits across all account types.'
  BY SEED finance;
```

- `ON CYCLES (…)` — a list; absent = universal (today's defined semantics).
- `CONVENTIONS (…)` — every id must resolve to a declared CONVENTION (the OVER
  membership pattern applied to conventions); the edge lives on the validation,
  where the pull happens. Convention-side `targets` shrinks to coarse consumer
  routing — serving-policy territory, deferred with fixture 02's other half.
- `OUTCOME` — the second prose slot; binding guidance and pass semantics stay
  split as the binder consumes them.
- category/tags: accepted INFORMATION LOST (browsing metadata; revisit as
  generic aspects on declared names if it ever earns a mechanism).

## Fork B — Convention-side ownership

The edge rides convention `TARGETS (validation: …)`. Spreads a validation's
dependencies across other statements' declarations; adding a validation means
re-declaring conventions (supersession churn on the wrong statement).

## Fork C — Prose only

Conventions and outcome folded into GUIDANCE. The binder is *fed* the split,
typed fields today; prose-folding un-types a working contract.

## Recommendation

**A.** The clause set mirrors the real envelope, both new clauses reuse the
existing membership-contract pattern, and ownership sits where the runtime
pull is.
