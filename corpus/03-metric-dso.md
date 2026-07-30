# 03 · Metric `dso` — GRAMMAR GAP + INFORMATION LOST + SEMANTICS UNDEFINED

Source: `dataraum-context/packages/dataraum-config/verticals/finance/metrics/working_capital/dso.yaml`
(persisted per `metrics` / `metric_parameters` / `metric_derives_from`, engine schema.sql)

```yaml
graph_id: dso
version: '1.0'
metadata:
  name: Days Sales Outstanding
  description: Average days to collect payment after sale
  category: working_capital
  tags: [ar, collection, working-capital]
output: {type: scalar, metric_id: dso, unit: days, decimal_places: 1}
parameters:
  days_in_period:
    type: integer
    default: 30
    options: [30, 90, 365]
    derivation: period_grain
dependencies:
  accounts_receivable:
    type: extract
    source: {standard_field: accounts_receivable, statement: balance_sheet}
    aggregation: sum
  revenue:
    type: extract
    source: {standard_field: revenue, statement: income_statement}
    aggregation: sum
  days_in_period: {type: constant, parameter: days_in_period, default: 30}
  dso:
    type: formula
    expression: (accounts_receivable / revenue) * days_in_period
    output_step: true
    validation:
    - {condition: 0 <= value <= 365, severity: warning, message: DSO outside typical range}
interpretation:
  ranges:
  - {min: 0,  max: 30,  label: EXCELLENT,  description: Very efficient collection}
  - {min: 31, max: 45,  label: GOOD,       description: Strong collection performance}
  - {min: 46, max: 60,  label: CONCERNING, description: Review collection processes}
  - {min: 61, max: 90,  label: POOR,       description: Significant working capital tied up}
  - {min: 91, max: 999, label: CRITICAL,   description: Urgent intervention required}
```

## Transcription — forced into §3.1/§3.4 as written

```glossql
DECLARE METRIC dso
  AS (accounts_receivable / revenue) * days_in_period
  UNIT 'days'
  PARAMETER days_in_period GRAIN month DEFAULT 30
  BY SEED finance;

DECLARE POLICY interpretation FOR dso
  BANDS (excellent < 31, good < 46, concerning < 61, poor < 91, critical)
  BY SEED finance;
```

## Gap — what the real parameter and metadata need

```glossql-gap
DECLARE METRIC dso
  AS (accounts_receivable / revenue) * days_in_period
  UNIT 'days'
  PARAMETER days_in_period TYPE integer OPTIONS (30, 90, 365)
    DERIVED FROM period_grain DEFAULT 30
  DISPLAY NAME 'Days Sales Outstanding'
  CATEGORY working_capital
  BY SEED finance;
```

## Findings

- Expression, unit, dependency DAG: clean — concept-space resolution covers the
  concepts and the parameter; the level-1/2 DAG derives from the AST.
- **GRAMMAR GAP — `PARAMETER` clause.** The sketch has no surface for parameter
  *type*, the closed *options* list, or `derivation: period_grain` (a derivation
  *rule*, not a grain *value* — `GRAIN month` names the wrong thing).
- **GRAMMAR GAP + INFORMATION LOST — interpretation.** `BANDS` can encode the
  boundaries (after an unstated inclusive-range → strict-< translation) but not
  the per-range `description` prose served to agents today.
- **INFORMATION LOST:** display name, category, tags, `decimal_places`,
  `output.type`.
- **SEMANTICS UNDEFINED — step-level validation.** `0 <= value <= 365` on the
  metric's output: `DECLARE VALIDATION … KIND constraint OVER (…)` takes
  concepts; whether a metric name is admissible in OVER, and what
  range-check-on-output means under the KIND vocabulary, is unspecified.
- The extract axis `statement: balance_sheet | income_statement` is part of the
  snippet semantic key today and open under §8.2 — see fixture 07.
