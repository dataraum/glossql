# Sprint 3 · Cycles — decision forks

**DECIDED 2026-07-30: Fork B** (project lead). Applied: SPEC §3.3 (ARGUMENTS,
ordered VALUES, TERMINAL + decomposition paragraph), §3.1 CYCLE FAMILY example
(concept-binding directions), §2.1 map row, §8.4 struck (SQL-inventor test
folded into §1.2(6)), grammar + parser + fixture 05. `feeds_into` accepted as
deliberate loss.

Gap (fixture `corpus/05`): §8.4's decomposition fails its own §9.1 acceptance
test on stage ORDER, value→stage binding, and completion semantics; `DECLARE
CYCLE` is promised in §2.1 and absent from §3; family directions bind concepts,
not labels; `feeds_into` is typed seed data with no home.

## Fork A — Admit `DECLARE CYCLE` as a named class

```sql
DECLARE CYCLE accounts_receivable
  STAGES (invoice_created > invoice_sent > payment_due > payment_received)
  STATUS invoices.status
  COMPLETION (paid, collected, cleared, closed)
  BY SEED finance;
```

Transcribes cycles.yaml 1:1. Fails the SQL-inventor test §8.4 established
(cycles are domain, not data processing); reverses a held direction; every
future domain family gets to cite the precedent.

## Fork B — Two generic mechanisms complete the decomposition — recommended

1. **Ordered label sets.** `VALUES` may declare an ordering — generic to data
   processing (stages, severity ladders, interpretation bands), mechanism-backed
   (progression/monotonicity checks, ordered rendering):

```sql
DECLARE ASPECT ar_stage VALUES (created < sent < due < paid) TERMINAL (paid)
  BY SEED finance;
```

2. **TERMINAL** marks absorbing labels — completion semantics as a property of
   the label set (order statuses, ticket states — generic). Completion-rate
   measurement and completion validations *derive* from it; the §8.4
   circularity (a validation needs semantics declared first) resolves.

   Value→stage binding rides the existing argumented-application pattern
   (null_token's, §3.3 — the spec must say aspects may declare arguments):

```sql
DECLARE ar_stage(invoices.status, token := 'delivered', value := sent)
  BY AGENT cataloguer CONFIDENCE 0.8;
```

3. Stages stay concepts in the pack (`PART OF` the cycle concept); family
   directions bind concepts per §8.4's own note
   (`DIRECTIONS (incoming accounts_receivable, outgoing accounts_payable)` —
   pair form). `DECLARE CYCLE` is struck from §2.1's map (the row points at the
   decomposition). **`feeds_into` is accepted as INFORMATION LOST** (bucket
   three: prose in the cycle concept's DESCRIPTION if wanted); aliases stay §6
   reserved.

## Fork C — No grammar change; stages as prose

Stages/order/completion ride DESCRIPTION prose; only the status-column binding
is an aspect. The detected `completion_rate` loses its declared semantics —
judgment leaks into detector code, violating principle 3. Too lossy.

## Recommendation

**B.** Two small, genuinely generic mechanisms (ordered VALUES, TERMINAL) close
three gaps at once, pass the SQL-inventor test, and §8.4's acceptance criteria
are met without admitting a domain statement. If chosen: §3.3 ASPECT grammar
gains ordering + TERMINAL + argument declarations; §8.4 closes; `DECLARE CYCLE
FAMILY` directions become concept-binding pairs; fixture 05 rewrites.
