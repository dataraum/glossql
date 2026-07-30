# Sprint 1 · Per-witness reliability — decision forks

**DECIDED 2026-07-30: Fork B** (project lead). Applied: `grammar.ebnf` actor
production, `harness/glossql_parser.py`, `corpus/06` flipped to ` ```glossql `,
SPEC.md §3.0 skeleton + keyed-class key + §3.4 example (+13/−3 lines). Bare
`DETECTOR x` is defined as shorthand for `DETECTOR x WITNESS x`. Still open
(smaller, separate forks): calibration provenance payload; placeholder priors.

Gap (fixture `corpus/06-witness-reliability.md`): the real reliability key is
(measurement, witness) — `null_semantics` pools three witnesses at 0.8681 /
0.2658 / 0.944 (`dataraum-config/entropy/reliabilities.yaml`); `claim_witnesses`
keys rows by `witness_id`, distinct from `detector_id`. glossql's
`DECLARE RELIABILITY DETECTOR x FOR aspect r` allows one number per (actor,
aspect). The structural mismatch sits inside §4's "load-bearing novelty."

Test artifact for all forks — transcribe this losslessly:

```yaml
witnesses:
  null_semantics:
    quarantine_clustering: 0.8681
    type_claim: 0.2658
    null_vocabulary: 0.944
```

## Fork A — Flatten: every witness is its own DETECTOR actor

```sql
WITNESS null_token(orders.amount, token := 'TBD', is_null := 0.91, is_value := 0.09)
  BY DETECTOR null_vocabulary
  EVIDENCE 'obs://run-342/null_semantics/orders.amount';
DECLARE RELIABILITY DETECTOR null_vocabulary FOR null_token 0.944
  BY CALIBRATION '2026-06-09';
```

- Zero grammar change; the numbers transcribe.
- **Loses the pooling group**: that one measurement pools these three witnesses
  is real structure (the eval rigs calibrate per-witness *within* a
  measurement's corpus); it survives only inside the opaque EVIDENCE ref.
- The actor roster triples, and `OBSERVE null_semantics` no longer names any
  witness actor — the request→result join weakens.

## Fork B — Two-level provenance: `DETECTOR <measurement> WITNESS <witness>` — recommended

```sql
WITNESS null_token(orders.amount, token := 'TBD', is_null := 0.91, is_value := 0.09)
  BY DETECTOR null_semantics WITNESS null_vocabulary
  EVIDENCE 'obs://run-342/null_semantics/orders.amount';
DECLARE RELIABILITY DETECTOR null_semantics WITNESS null_vocabulary
  FOR null_token 0.944
  BY CALIBRATION '2026-06-09';
```

- Grammar delta: one optional `WITNESS name` member on the DETECTOR actor form;
  the reliability key becomes (actor, witness, aspect); the witness statement's
  claim-slot row gains the member `claim_witnesses.uq(target, claim_field,
  **witness_id**, run_id)` already has.
- Transcribes reliabilities.yaml losslessly; keeps measurement identity for
  OBSERVE and the calibration rigs; keeps the attribution/subject boundary
  (§3.0's three name kinds) intact.
- Cost: compound actor identity — `BY DETECTOR x` (no WITNESS) must be defined
  as either shorthand for a single-witness detector or an error.

## Fork C — Radical: kill the RELIABILITY class; reliability is an aspect on the actor

```sql
DECLARE reliability(null_vocabulary, aspect := null_token, value := 0.944)
  BY CALIBRATION '2026-06-09';
```

- Removes a keyed class (one fewer [REPAIR] production); reliability becomes an
  ordinary claim slot — supersedable, retractable, WHY-traceable like any
  declaration. Maximum uniformity.
- **Spends a principled boundary**: §3.0's attribution names are "never defined,
  never resolved" — this makes an actor a *subject* of declarations, and the
  subject grammar must admit bare attribution names, colliding with the already
  open GLOSS namespace-resolution problem (grammar.ebnf U4).

## Recommendation

**B.** It is the only fork that transcribes the artifact losslessly without
spending the attribution/subject boundary. A discards real structure to save a
clause; C saves one production at the price of widening U4. If B is chosen:
flip `corpus/06` block 4's tag to ` ```glossql `, add the WITNESS member to
`grammar.ebnf` (actor production) and `harness/glossql_parser.py`, and fold a
~6-line diff into SPEC.md §3.3/§3.4 (replacing the one-number-per-detector
sentence). Calibration provenance (corpus id, calibrated-vs-placeholder flag)
stays open — it is a separate, smaller fork about what `BY CALIBRATION` may
carry.
