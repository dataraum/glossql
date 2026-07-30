# 05 · Cycle `accounts_receivable` + family `settlement` — RESOLVED (sprint 3, fork B)

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

Asserted side: `detected_business_cycles` (schema.sql) — stages, status_table,
status_column, completion_value, completion_rate, family, direction, evidence.

## Transcription — the decomposition, complete (decided 2026-07-30)

```glossql
DECLARE CONCEPT invoice_created KIND event
  DESCRIPTION 'AR stage: invoice created' BY SEED finance;
DECLARE RELATIONSHIP invoice_created PART OF accounts_receivable BY SEED finance;

DECLARE ASPECT ar_stage VALUES (created < sent < due < paid) TERMINAL (paid)
  BY SEED finance;

DECLARE ar_stage(invoices.status, token := 'delivered', value := sent)
  BY AGENT cataloguer CONFIDENCE 0.8;
DECLARE ar_stage(invoices.status, token := 'paid', value := paid)
  BY AGENT cataloguer CONFIDENCE 0.9;

DECLARE CYCLE FAMILY settlement
  DIRECTIONS (incoming accounts_receivable, outgoing accounts_payable)
  BY SEED finance;
```

## Findings

- **Stage order — RESOLVED.** `VALUES (a < b < c)` declares ordered label sets
  (generic: stages, severity ladders, bands); progression checks derive.
- **Completion — RESOLVED.** `TERMINAL (paid)` marks absorbing labels;
  completion validations and the completion-rate measurement derive from it —
  the §8.4 circularity is gone.
- **Value→stage binding — RESOLVED.** Per-instance argumented applications
  (`token := 'delivered'`), with argument names declared via `ARGUMENTS` on
  the aspect (spec §3.3).
- **Family directions — RESOLVED.** `DIRECTIONS` binds concepts, never bare
  labels; `DECLARE CYCLE` is struck from the §2.1 map (the row now points at
  this decomposition).
- **INFORMATION LOST, by decision:** `feeds_into` and per-stage indicator
  prose stay pack description; aliases remain §6 reserved (Synonyms).
