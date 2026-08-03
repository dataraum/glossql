//! The two fixed schemas the language nails down: the standard grounding
//! schema (SPEC.md §5.2, validates every QUERY gloss) and the attest shape
//! check for detector eligibility (SPEC.md §7.1).

use serde_json::Value;

/// The standard grounding schema, verbatim from SPEC.md §5.2.
pub const GROUNDING_SCHEMA: &str = r#"{
  "type": "object",
  "required": ["sql"],
  "additionalProperties": false,
  "properties": {
    "sql": {"type": "string"},
    "assumptions": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["assumption"],
        "properties": {
          "dimension": {"type": "string"},
          "assumption": {"type": "string"},
          "basis": {"type": "string"},
          "confidence": {"type": "number", "minimum": 0, "maximum": 1}
        }
      }
    }
  }
}"#;

pub fn grounding_schema() -> Value {
    serde_json::from_str(GROUNDING_SCHEMA).expect("SPEC §5.2 schema is valid JSON")
}

/// A function is eligible as detector only if its RETURNS conforms to the
/// standard attest schema. Full subschema entailment is undecidable in
/// general; the check here is the honest shallow version — RETURNS must
/// declare both `band` and `score` properties.
pub fn returns_carries_attest_shape(returns: &Value) -> bool {
    let props = returns.pointer("/properties");
    matches!(
        props,
        Some(Value::Object(map)) if map.contains_key("band") && map.contains_key("score")
    )
}

/// Validate `instance` against `schema`, first violation as the message.
pub fn validate_instance(schema: &Value, instance: &Value) -> Result<(), String> {
    let validator = jsonschema::validator_for(schema).map_err(|e| e.to_string())?;
    validator.validate(instance).map_err(|e| e.to_string())
}
