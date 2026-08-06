# The v0.3 remainder — rulings on the ledger's classification

Date: 2026-08-06. The adversarial review's ledger (§5 of
`2026-08-06-adversarial-review.md`) classified what v0.3 still holds.
This record follows a source-grounded evaluation of that classification
(four independent readings of `../dataraum-context`) and the project
lead's rulings on it. Four items are ruled in, in order; everything
else is named here so it stays named, not rebuilt by accident.

## Ruled in, in order

**1. The ADBC executor, with key harvest as evidence.** `DECLARE
SOURCE … type relational_db` stores today and every recipe against it
errors (`crates/import/src/lib.rs:44`). The Rust stack is compatible as
is: `adbc_core`/`adbc_driver_manager` 0.24 accept arrow `>=58,<60`, the
workspace holds arrow 58.4.0, and the driver manager returns Arrow
batches that drop into `Landed` unconverted. The executor runs recipe
and probe SQL at the source; the operational cost is the driver shared
libraries, which ship outside cargo.

Key harvest rides the same door: a probe against a relational source
can read the backend's own catalog (information_schema and its
per-backend spellings), so declared PK/FK arrive without new statement
surface — the skill teaches the spellings, the door stays as it is.

**The lead's caveat, binding:** recipes reshape what lands. A declared
FK describes the source's tables, not necessarily the landed tables —
an interesting signal that can also be wrong. Harvested keys therefore
enter as evidence for the relationship judge only, never as declared
relationships, and the skill says so where the judge reads it.

**2. Import cast accounting — cells and sentinel candidates, no
vocabulary.** Typing is authored in recipes, so a silently-nulling
cast is our own risk, and the import counts rows, never cells. The
v0.3 transcription is closed to us on our own rulings: its chain needs
a raw layer beside the typed one and a calibrated pooling apparatus
(both dropped by design), and its curated null vocabulary destroys
known sentinels at CSV load — only novel tokens ever reach its
detector, and two of its six vocabulary families are dead config.

The glossql form works at the one moment raw and typed values coexist:
the landing. For each `try_*` cast in the recipe's SELECT list, one
companion aggregate at the source counts `input IS NOT NULL AND cast
IS NULL` and takes the top failing tokens by frequency. The counts
arrive in the `DECLARE RECIPE` outcome — the decision moment — and
persist beside `source_rows`/`landed_rows`. The agent judges the
tokens; closure is an authored recipe amendment that supersedes and
re-lands. **No sentinel list exists anywhere in this design (ruled:
none may).**

**3. The grounding-collision guard.** Two concepts grounding to the
same rows make every ratio between them compute 1.0, silently. v0.3
compares canonical SQL parse trees and flags only pairs already
declared disjoint; we have no disjoint predicate and need none —
bucket groundings by canonicalized SQL and report every shared bucket,
the agent judge removes deliberate synonyms. Recall from the
measurement, judgment from the judge.

**4. Sign partition and ΔBIC in the reconcile kernel.** The ledger
said we ported the statistic but not the search; the source says
otherwise — `crates/scripts/src/lib.rs:936-950` enumerates v0.3's full
convention space as one matrix product and `behavior_evidence.rhai`
selects support-first by Wilson lower bound with the common-denominator
rule. What v0.3 still has over the port: the sign partition
(re-classify winning voters against the negated anchor; feeds the
natural-vs-ledger-signed reading of a balance) and the ΔBIC>10 arity
tiebreak. Both are arithmetic over data already in the kernel, no new
SQL. Pairing stays on declared relationships — judged inputs, kept
deliberately over v0.3's slice-identity pairing.

## Named, not built

Held as wishes until a run pulls them, per the corpus-first filter:

- **Join-path ambiguity** — two declared paths reaching the same table;
  a cheap sweep over the `relationships` relation when the dimensions
  flow first trips on one.
- **Unit-token extraction** — value-carried units ("100 kg") as recall
  machinery for the unit gloss; no run has hit a unit-carrying column.
- **Benford** — a KL surprise on leading digits, trivial as a
  measurement if an audit-shaped target ever appears; none does.

## Confirmed not wanted, with the mechanics that confirm it

- **Enriched views** — exist to give v0.3's agent name-addressable
  joined surfaces with grain probes; our joins are inline and
  grain-checked in the relationships and dimensions flows.
- **Column eligibility** — its only drop rule is the 100%-null column;
  everything else warns and keeps. Nothing to want.
- **Run-versioned read views, snapshot heads, the property graph** —
  versioning infrastructure; supersession-as-a-read answers the same
  question.
- **Validation induction as engine machinery** — engine-served context
  plus a membership guard around an LLM proposal step; skills and the
  agent do this without machinery.
- **Readiness/loss rollup and use-case contracts** — weighted loss and
  banded gates; ATTEST bands and read policy carry those roles.
- **Graph topology, dimensional entropy** — prompt-context
  derivations; the agent reads relationships as tables, and the latter
  is orphaned in v0.3 itself.

The sweep also found v0.3 carrying dead machinery — unreachable
algorithms, unphased analyses, dead vocabulary families, a stale
threshold key. Transcription-porting would have imported that rot;
porting only what a run has pulled is the filter doing its job.
