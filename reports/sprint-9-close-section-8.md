# Sprint 9 · Closing §8 — the last four decisions of the review phase

**DECIDED 2026-07-30: all four = A** (project lead). Applied: §3.0 skeleton
folded (all reconstructed productions now in the spec; grammar.ebnf REPAIR
markers dropped), §8 emptied, status list refreshed (§3.1/§3.3 now Ready),
RECONCILES rhs tightened to a bare concept (grammar + parser + §3.1),
core-pack ownership in §3.3, workspace-scoped teach examples in §3.2, groups-
as-concepts in §3.1 + fixtures 02/04.

Four decisions; if the recommendations hold, §8 empties and §3.0's skeleton is
folded to match grammar.ebnf. What stays open afterward is implementation-facing
and stays honestly flagged: serving clause semantics (fixture 09), grounding
follow-ups (fixture 07), the log envelope (§1.1, deliberately last), the
remaining §2 transcription burn-down, and the replay/authoring harness parts.

## Q1 — §8.1 Universal-core aspects: who owns the list?

Direction already fixed: the built-in-function model, shipped as a seed pack,
product-standard, never grammar. Remaining: "the core list and its label sets."

**A — The pack owns the list; the spec owns the mechanism — recommended.**
The spec commits to: the aspect mechanism (§3.3, settled), the *existence* of a
core seed pack, and the §9.2 slice's two named members (`null_token`,
`behavior`) needed for the PoC. The full roster (type patterns, stored sign,
derived formulas, …) is core-pack content, versioned with the product like a
standard library — `md5()` is documented with the engine, not in the SQL
standard's grammar. §8.1 closes.

**B — The spec enumerates the core list now.** Freezes today's detector roster
into normative prose; every new core aspect becomes a spec change. The list is
exactly the kind of vocabulary §1.2(6) keeps out of the grammar.

## Q2 — §8.2 `RECONCILES WITH` right-hand side

Evidence: `concept_edges` rows are **concept pairs** (predicate + tolerance);
`concept_reconciliation` executes **pairs** (from_concept, to_concept, delta,
verdict, 7-value abstain). No expression-shaped edge exists anywhere. The
spec's example rhs `(revenue - collections)` is invented.

**A — Bare concept rhs; no subsumption — recommended.**
`x RECONCILES WITH y [TOLERANCE t]`. The expression case decomposes the
sprint-2 way: the derived quantity earns a concept (declared, grounded or
metric-defined), then reconciles pairwise. And the two mechanisms stay
distinct, as they are in the running system: `RECONCILES WITH` feeds the
pairwise `reconciliation` measurement (§2.2); `KIND balance` validations stay
the authored n-ary checks. §8.2 closes.

**B — Expression rhs as sketched.** Puts a concept-space expression on an edge
— an unnamed derived quantity with no grounding home and no supersession slot
of its own; the reconciliation executor would hold expressions the metric
plane already knows how to hold.

## Q3 — §8.3 Typing / null / expectation teaches

Evidence (fixture 08): `type_pattern` and `null_value` are workspace-scoped
vocabulary; expectation teaches assert per-column formulas/dependencies. §8.3's
own direction: resist new DECLARE families.

**A — Core-pack aspects at workspace/column scope; zero new grammar — recommended.**
Sprint 3's `ARGUMENTS` already provides everything needed:

```sql
DECLARE type_pattern(workspace, name := 'eu_date',
  pattern := '^\d{2}\.\d{2}\.\d{4}$', type := 'DATE',
  standardize := 'strptime(value, ''%d.%m.%Y'')') BY USER analyst;
DECLARE null_token(workspace, token := 'TBD', value := is_null) BY USER analyst;
DECLARE derived(orders.total, formula := 'subtotal + tax') BY USER analyst;
```

The aspects are declared in the core pack (Q1-A); claim slots are
(workspace, type_pattern, name) and (workspace, null_token, token) — the same
aspect can bind at both workspace scope (vocabulary) and column scope (a
specific binding), which is what the running system's overlay/annotation split
already does. §8.3 closes with no grammar change.

**B — New DECLARE families** (`DECLARE TYPE PATTERN …`). Violates the
resist-direction; three new statement heads for what the claim-slot mechanism
already expresses.

## Q4 — Group-shaped operands (fixture 04's OVER + fixture 02's concept_groups)

Evidence: trial_balance's real operands are account-type *families*;
`sign_natural_balance`'s `concept_groups` are engine-linted member lists. Both
are the same shape: a named group of concepts.

**A — Groups are concepts with `PART OF` members — recommended.**
The sprint-2 move again: name the structure. Families/groups are declared
concepts (`KIND group`), members are `PART OF` edges; `OVER` stays required
and lists the family concepts; a convention's machine-checked half lives in
concept space, referenced from its prose by name — the convention's
`concept_groups` field dissolves, and its `targets` routing stays deferred to
serving policy (fixture 02's other half).

```sql
DECLARE CONCEPT credit_normal KIND group BY SEED finance;
DECLARE RELATIONSHIP revenue PART OF credit_normal BY SEED finance;
DECLARE VALIDATION trial_balance KIND balance
  OVER (credit_normal, debit_normal) … BY SEED finance;
```

**B — A GROUP clause on conventions/validations.** A second membership surface
beside `PART OF` for the same relation; two homes for one fact.

**C — Make OVER optional.** Gives up the membership contract — the one
declaration-time check that stops fabricated references.
