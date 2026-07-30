# Sprint 4 · Reserved words (U5) — decision forks

**DECIDED 2026-07-30: Fork B** (project lead); sprint 5 below: **Fork A**.
Applied: SPEC §3 token rule + OBSERVE provenance (examples + prose + §10),
`grammar.ebnf` (U5 closed, observation gains provenance), parser (double-quoted
identifiers; OBSERVE requires BY). VIEW bodies stay bare SQL with the
scan-from-end rule as specified behavior (U1).

Gap (grammar.ebnf U5, found building the parser): the spec has no reserved-word
rule. Clause heads (`IN`, `AS`, `FROM`, `WHERE`, `UNIT`, …) are legal inside
bare expression payloads (metric `AS`, grounding `AS`/`WHERE`, reconciles RHS),
so expression boundaries are ambiguous. Transported bodies (quoted recipe
strings) are unaffected. Cheap to decide now; expensive after more clauses.

## Fork A — Mandatory parentheses on expression payloads

`AS (sum(amount))`, `WHERE (doc_type = 'invoice')`. Trivially unambiguous,
uniformly noisy; every author pays forever for the parser's convenience.

## Fork B — SQL's own answer: reserve the clause heads — recommended

glossql statement bodies reserve the clause-head words; a colliding identifier
is double-quoted (`"unit"`), exactly as SQL treats its keywords. The substrate
already has the quoting mechanism; the reserved list is `grammar.ebnf`'s
CLAUSE_HEADS — small, closed, published. Expression spans end at any reserved
word; no new syntax.

## Fork C — Expression-last ordering discipline

Every statement puts its one expression clause last, before `BY`. Fails on
GROUNDING (two spans: AS + WHERE) and on VIEW (SQL body contains `BY`); already
needed the scan-from-end hack (U1). Fragile, rejected.

## Recommendation

**B.** One sentence in §3.0 ("clause-head keywords are reserved in glossql
statement bodies; quote to use them as identifiers — transported SQL strings
are unaffected"), plus the published list. Also resolves U1 for VIEW bodies in
principle (BY reserved ⇒ SQL bodies must quote or the body stays a transported
string — see the question the fork asks below).

Open sub-question if B: does `DECLARE VIEW … AS <bare SQL>` survive, or does
the VIEW body become a transported string like recipes (`AS 'SELECT …'`),
removing U1 entirely? Recommendation: keep bare SQL (views are glossql-adjacent
SQL the engine parses for join admission — a string would hide the AST §3.2's
admission check needs) and accept the scan-from-end rule as specified behavior.

---

# Sprint 5 · OBSERVE provenance — decision forks

Gap (found by the parser): §3.0 says "every authored statement carries `BY`";
§3.3/§10's OBSERVE examples carry none. OBSERVE enters the log (replay re-binds
results), so unattributed OBSERVE breaks §1's "every statement names an
identified actor."

## Fork A — Require BY; orchestration code is an AGENT actor — recommended

```sql
OBSERVE profile, typing, temporal ON orders BY AGENT onboarding;
```

No new actor class: §2.5 already has code producing DECLARE statements under
existing classes; the actor name identifies the workflow. Spec examples gain
one clause; the invariant holds.

## Fork B — New actor class (SYSTEM / ENGINE)

Honest about code-not-agent, but §3.0's actor classes are attribution
vocabulary, not an ontology of souls — a sixth class buys nothing pooling or
policy can use, and every class is one more `BY` production forever.

## Fork C — Exempt OBSERVE from provenance

Treats OBSERVE as reading-shaped. But it writes to the log and replay depends
on it; the exemption would be the only unattributed log statement. Rejected.

## Recommendation

**A.** Smallest diff, no new vocabulary, restores the invariant.
