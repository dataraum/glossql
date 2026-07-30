# 06 · Claim witnesses + reliabilities — witness CLEAN; per-witness reliability RESOLVED (sprint 1, fork B)

Source: `claim_witnesses` (engine schema.sql):

```sql
CREATE TABLE claim_witnesses (
    target VARCHAR NOT NULL,
    claim_field VARCHAR NOT NULL,        -- claim-slot id, e.g. "null_token:TBD"
    witness_id VARCHAR NOT NULL,         -- ≠ detector_id, part of the UNIQUE key
    distribution JSONB,
    reliability FLOAT NOT NULL,
    detector_id VARCHAR NOT NULL,
    run_id VARCHAR NOT NULL,
    CONSTRAINT uq UNIQUE (target, claim_field, witness_id, run_id)
);
```

Source: `dataraum-config/entropy/reliabilities.yaml` — reliability is calibrated
**per witness within a measurement**, not per detector:

```yaml
witnesses:
  null_semantics:
    quarantine_clustering: 0.8681
    type_claim: 0.2658
    null_vocabulary: 0.944
  temporal_behavior:
    llm_claim: 0.838                    # measured 2026-06-10, stratified corpus
    structural_reconciliation: 0.889    # measured 2026-06-11, wave-2 rig
```

Plus per-measurement calibration provenance: `calibrated: true/false`,
corpus_version, estimator, per_class_accuracy, brier, sample sizes, dates.

## Transcription — the witness statement (clean)

```glossql
WITNESS behavior(orders.amount, stock := 0.11, flow := 0.89)
  BY DETECTOR temporal_behavior
  EVIDENCE 'obs://run-342/temporal_behavior/orders.amount';

DECLARE RELIABILITY DETECTOR temporal_behavior FOR behavior 0.838
  BY CALIBRATION '2026-06-10';
```

## Per-witness reliability — decided 2026-07-30 (sprint 1, fork B)

```glossql
WITNESS null_token(orders.amount, token := 'TBD', is_null := 0.91, is_value := 0.09)
  BY DETECTOR null_semantics WITNESS null_vocabulary
  EVIDENCE 'obs://run-342/null_semantics/orders.amount';

DECLARE RELIABILITY DETECTOR null_semantics WITNESS null_vocabulary
  FOR null_token 0.944 BY CALIBRATION '2026-06-09';
```

## Findings

- Witness statement: **TRANSCRIBES CLEANLY** — target→subject,
  claim_field→(aspect, argument), distribution→labelled args, run_id opaque in
  the EVIDENCE ref, all as §3.3 intends.
- **RESOLVED (was GRAMMAR GAP) — witness_id collapsed into detector_id.** The
  real reliability key is (measurement, witness); one detector pools several
  witnesses at different calibrated weights (null_semantics: three). Fork B
  (2026-07-30): the DETECTOR actor takes an optional `WITNESS name` member;
  reliability is keyed (detector, witness, aspect); bare `DETECTOR x` is
  shorthand for `DETECTOR x WITNESS x` (single-witness detector), which keeps
  every existing example valid. `claim_witnesses.witness_id` now has a
  counterpart. → `reports/sprint-1-reliability-forks.md`.
- **INFORMATION LOST — calibration provenance.** `BY CALIBRATION '2026-07'`
  carries a name; the calibrated-vs-placeholder flag (consumed today via
  `ReliabilityConfig.calibrated_for`), corpus id, estimator, per-class accuracy
  have nowhere to go.
- **SEMANTICS UNDEFINED — placeholder priors.** Uncalibrated detectors run at
  placeholder weights today; §3.3 defers undeclared producers to "the
  reliability policy", which has no statement form (§3.4's WEIGHT is a sketch
  inside POLICY readiness).
