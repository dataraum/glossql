//! The runtime alone: invocation contract, path fence, kernels on the
//! zero-copy handles — through a fake door, no lake.

use std::sync::Arc;

use datafusion::arrow::array::{RecordBatch, StringArray};
use datafusion::arrow::datatypes::{DataType, Field, Schema};
use glossql_glossary::FunctionRow;
use glossql_scripts::RhaiRuntime;
use glossql_session::{FunctionRuntime, SqlDoor};
use serde_json::{Value, json};

struct FakeDoor;

impl SqlDoor for FakeDoor {
    fn sql(&self, _query: &str) -> Result<Vec<RecordBatch>, String> {
        let schema = Arc::new(Schema::new(vec![Field::new("v", DataType::Utf8, true)]));
        let batch = RecordBatch::try_new(schema, vec![
            Arc::new(StringArray::from(vec![
                Some("12.50"),
                Some("8.00"),
                Some("n/a"),
                None,
            ])),
        ])
        .unwrap();
        Ok(vec![batch])
    }
}

fn function(script: &str) -> FunctionRow {
    FunctionRow {
        name: "t".into(),
        scope_dataset: None,
        script: script.into(),
        accepts: vec![],
        returns: json!({"type": "object"}),
    }
}

fn runtime_with(dir: &std::path::Path, name: &str, body: &str) -> RhaiRuntime {
    std::fs::write(dir.join(name), body).unwrap();
    RhaiRuntime::new(dir)
}

#[test]
fn scope_carries_subject_context_and_the_door() {
    let dir = tempfile::tempdir().unwrap();
    let rt = runtime_with(
        dir.path(),
        "t.rhai",
        r#"
        let t = db.query("ignored");
        let c = t.col("v");
        #{
            subject: subject,
            hint: context.hint,
            rows: c.count(),
            nulls: c.null_count(),
            distinct: c.distinct(),
            min: c.min(),
            parse: c.parse_rate("DOUBLE"),
            matched: c.match_rate("^\\d+\\.\\d+$"),
        }
        "#,
    );
    let out = rt
        .invoke(
            &function("t.rhai"),
            "orders.amount",
            &json!({"hint": 7}),
            Arc::new(FakeDoor),
        )
        .unwrap();
    assert_eq!(out["subject"], json!("orders.amount"));
    assert_eq!(out["hint"], json!(7));
    assert_eq!(out["rows"], json!(4));
    assert_eq!(out["nulls"], json!(1));
    assert_eq!(out["distinct"], json!(3));
    assert_eq!(out["min"], json!("12.50"));
    // 2 of 3 non-null values parse as DOUBLE; the same 2 match the pattern.
    assert!((out["parse"].as_f64().unwrap() - 2.0 / 3.0).abs() < 1e-9);
    assert!((out["matched"].as_f64().unwrap() - 2.0 / 3.0).abs() < 1e-9);
}

#[test]
fn script_paths_stay_under_the_root() {
    let dir = tempfile::tempdir().unwrap();
    let rt = RhaiRuntime::new(dir.path());
    let err = rt
        .invoke(
            &function("../outside.rhai"),
            "s",
            &Value::Null,
            Arc::new(FakeDoor),
        )
        .unwrap_err();
    assert!(err.contains("must stay under the functions root"), "{err}");
}

#[test]
fn a_script_error_names_the_function() {
    let dir = tempfile::tempdir().unwrap();
    let rt = runtime_with(dir.path(), "t.rhai", r#"throw "no data";"#);
    let err = rt
        .invoke(&function("t.rhai"), "s", &Value::Null, Arc::new(FakeDoor))
        .unwrap_err();
    assert!(err.contains("`t`") && err.contains("no data"), "{err}");
}
