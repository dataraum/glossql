# 13 · Typing patterns and null values — config as glosses

Source artifacts (verified 2026-08-03):
`packages/dataraum-config/phases/typing.yaml` — the pattern list; a real
pattern carries name, regex, inferred_type, examples, ambiguity flag,
standardization SQL (`STRPTIME("{col}", '%d.%m.%Y')`), case sensitivity,
locale, and PII marks, plus file-level `min_confidence` and sample size.
`packages/dataraum-config/null_values.yaml` — categorized null strings with
per-value flags. `packages/engine/src/dataraum/core/overlay.py`
(`_apply_type_pattern`, `_apply_null_value`) — merges human teaches into
those files: a hand-built base-plus-amendment mechanism.

Fork tested: a dedicated `DECLARE PATTERN [regex] FOR [TYPE | NULL_VALUE]`
head fails transcription — the real pattern shape needs eight more fields
than a regex and a target, so the head grows back into a JSON body with a
keyword in front, and the grammar by one head. The surviving fork: the
configs are FACT glosses on the dataset. The typing function reads the
latest body; the base set is written at vertical replay; a teach is a human
re-gloss superseding whole-body (approved: the bodies are small JSON
documents, edited and read as wholes). Base-vs-taught falls out of the
(subject, aspect, actor kind) key; the overlay module has nothing left to do.

```glossql
USE fin;

DECLARE ASPECT null_values WITH {
  "type": "object",
  "properties": {
    "values": {"type": "array", "items": {"type": "object",
      "properties": {"value": {"type": "string"},
                     "case_sensitive": {"type": "boolean"},
                     "category": {"type": "string"}},
      "required": ["value"]}}},
  "required": ["values"],
  "additionalProperties": false
} AS FACT;

GLOSS null_values ON fin AS {"values": [
  {"value": "", "category": "standard"},
  {"value": "NULL", "case_sensitive": false, "category": "standard"},
  {"value": "#N/A", "category": "spreadsheet"},
  {"value": "TBD", "category": "missing_indicator"}
]};

DECLARE ASPECT type_patterns WITH {
  "type": "object",
  "properties": {
    "min_confidence": {"type": "number"},
    "patterns": {"type": "array", "items": {"type": "object",
      "properties": {"name": {"type": "string"},
                     "pattern": {"type": "string"},
                     "inferred_type": {"type": "string"},
                     "ambiguous": {"type": "boolean"},
                     "standardization_expr": {"type": "string"},
                     "examples": {"type": "array"}},
      "required": ["name", "pattern", "inferred_type"]}}},
  "required": ["patterns"],
  "additionalProperties": false
} AS FACT;

GLOSS type_patterns ON fin AS {"min_confidence": 0.85, "patterns": [
  {"name": "iso_date", "pattern": "^\\d{4}-\\d{2}-\\d{2}$",
   "inferred_type": "DATE", "examples": ["2024-01-15"]},
  {"name": "eu_date", "pattern": "^\\d{1,2}\\.\\d{1,2}\\.\\d{2,4}$",
   "inferred_type": "DATE",
   "standardization_expr": "STRPTIME(\"{col}\", '%d.%m.%Y')"},
  {"name": "us_date", "pattern": "^\\d{1,2}/\\d{1,2}/\\d{2,4}$",
   "inferred_type": "DATE", "ambiguous": true,
   "standardization_expr": "STRPTIME(\"{col}\", '%m/%d/%Y')"}
]};
```

A teach is the same statement on a human connection — the whole amended body,
read first via `GLOSSARY(fin.null_values)`:

```glossql
GLOSS null_values ON fin AS {"values": [
  {"value": "", "category": "standard"},
  {"value": "NULL", "case_sensitive": false, "category": "standard"},
  {"value": "#N/A", "category": "spreadsheet"},
  {"value": "TBD", "category": "missing_indicator"},
  {"value": "~~~~~", "category": "taught"}
]};
```

## Findings

- Zero grammar change: both artifacts transcribe with existing constructs —
  `DECLARE ASPECT … AS FACT` + `GLOSS`, subject = the dataset.
- Whole-body supersession replaces the overlay's per-entry merge: the teach
  skill does read–amend–re-gloss. The human slot supersedes the base slot by
  the ordinary key; no merge machinery survives.
- The typing function's contract is the consumer: it ACCEPTS the aspect's
  schema, so the same JSON Schema governs the gloss and the script input.
