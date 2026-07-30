# 05 · Cycle `accounts_receivable` + family `settlement` — GRAMMAR GAP (heaviest)

Source: `dataraum-context/packages/dataraum-config/verticals/finance/cycles.yaml`

```yaml
cycle_types:
  accounts_receivable:
    description: "AR collection cycle: customer invoices settled by INCOMING flows …"
    business_value: high
    aliases: [ar_cycle, receivables_cycle, collection_cycle]
    typical_stages:
      - {name: "Invoice Created",  order: 1, indicators: [created, new, open, issued]}
      - {name: "Invoice Sent",     order: 2, indicators: [sent, delivered, notified]}
      - {name: "Payment Due",      order: 3, indicators: [due, outstanding, pending]}
      - {name: "Payment Received", order: 4, indicators: [paid, received, collected, cleared]}
    completion_indicators: [paid, collected, cleared, closed]
    feeds_into: [journal_entry_cycle]
cycle_families:
  settlement:
    directions: {incoming: accounts_receivable, outgoing: accounts_payable}
```

Asserted side: `detected_business_cycles` (schema.sql) — cycle_name, cycle_type,
canonical_type, is_known_type, family, direction, stages, entity_flows,
status_table, status_column, completion_value, completion_rate, evidence,
confidence, business_value.

## Transcription — §8.4's decomposition recipe, attempted honestly

```glossql
DECLARE CONCEPT invoice_created KIND event BY SEED finance;
DECLARE RELATIONSHIP invoice_created PART OF accounts_receivable BY SEED finance;
DECLARE ASPECT ar_stage VALUES (created, sent, due, paid) BY SEED core;
DECLARE ar_stage(invoices.status, value := created) BY AGENT cataloguer CONFIDENCE 0.8;
DECLARE CYCLE FAMILY settlement DIRECTIONS (incoming, outgoing) BY SEED finance;
```

## Gaps — four fields the decomposition cannot carry

```glossql-gap
DECLARE ASPECT ar_stage ORDERED VALUES (created, sent, due, paid) BY SEED core;
DECLARE CONCEPT accounts_receivable COMPLETION (paid, collected, cleared, closed)
  BY SEED finance;
DECLARE RELATIONSHIP accounts_receivable FEEDS INTO journal_entry_cycle
  BY SEED finance;
DECLARE CYCLE FAMILY settlement
  DIRECTIONS (incoming accounts_receivable, outgoing accounts_payable)
  BY SEED finance;
```

## Findings

- **`DECLARE CYCLE` is promised in §2.1 and absent from §3** — a gap by the
  spec's own completeness rule.
- **GRAMMAR GAP — stage ORDER.** `order: 1..4` is load-bearing (progression,
  stuck-cycle analysis); PART OF edges are unordered, ASPECT VALUES are
  unordered label sets (only dimension concepts may declare ORDERING).
- **SEMANTICS UNDEFINED — value→stage binding.** Binding several status values
  to one stage needs per-(value, stage) argumented applications; nothing says
  stage aspects take arguments.
- **GRAMMAR GAP — completion semantics.** "This status value means complete" is
  a semantics assertion, not a check; §8.4 says "completion-semantics
  validations," but the completion-rate measurement needs the declaration first.
  Circular; no construct.
- **INFORMATION LOST (by explicit design) — `feeds_into`.** §3.1 bans domain
  edges without a mechanism; today it is typed, seeded data.
- **GRAMMAR GAP (spec-admitted, §8.4)** — family directions bind *concepts*
  (`incoming: accounts_receivable`), not the bare labels §3.1 sketches.
- Aliases → §6 reserved (Synonyms). Detected-instance derived fields correctly
  excluded per §2.6.

**§9.1 acceptance status for §8.4's decomposition: FAILS** on stage order,
value→stage binding, and completion semantics.
