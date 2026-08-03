# 03 · Metric `dso` — TRANSCRIBES (metric = function script)

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

## Transcription

A metric is a script in the function library. The parameter surface is
`ACCEPTS`; the output contract is `RETURNS`; the concept inputs are the
script's business (it reads the groundings via the glossary).

```glossql
DECLARE FUNCTION dso FOR fin FROM 'functions/dso.py'
  ACCEPTS {
    "type": "object",
    "properties": {
      "days_in_period": {"type": "integer", "default": 30, "enum": [30, 90, 365]}
    }
  }
  RETURNS {
    "type": "object",
    "required": ["value"],
    "properties": {
      "value": {"type": "number"},
      "unit": {"const": "days"},
      "interpretation": {"enum": ["EXCELLENT", "GOOD", "CONCERNING", "POOR", "CRITICAL"]}
    }
  };

SELECT dso(days_in_period => 90) FROM fin;
```

## Findings

- **TRANSCRIBES as a script.** The declarative expression, dependency DAG,
  parameter clause, step-level validation, and interpretation ranges all move
  into `functions/dso.py` — swappable code, not grammar. This is the accepted
  trade: analytical logic leaves the declarative layer.
- The old track's unresolved display-metadata gap (name, category, tags,
  decimal_places) closes by relocation, not by a clause: display metadata is
  script-side or RETURNS-schema annotation. Nothing left for the grammar to
  carry.
- `derivation: period_grain` (parameter derived from another producer) is the
  `ACCEPTS [schema]#[json_pointer]` form — pointer syntax still a placeholder:

```glossql
DECLARE FUNCTION dso_auto FOR fin FROM 'functions/dso.py'
  ACCEPTS period_grain#/properties/days
  RETURNS {"type": "object", "properties": {"value": {"type": "number"}}};
```
