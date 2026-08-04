//! The M4 acceptance test: fixture 11 end-to-end with real scripts. A
//! recipe lands `orders_raw`; `infer_types` measures candidates from the
//! taught patterns; `decide_types`' pick fills the `type` slot; the typed
//! view serves under the bare name with `orders_quarantined` beside it; an
//! agent gloss supersedes the pick; writes invalidate dependent evidence;
//! the detector adjudicates at read; the collapsed read discloses state;
//! the quality plane (outliers, eligibility) chains on the profile through
//! ACCEPTS; an ineligible column leaves the view and returns by gloss.

use std::sync::Arc;

use datafusion::arrow::array::{Int64Array, RecordBatch, StringArray};
use datafusion::arrow::datatypes::{DataType, Field, Schema};
use datafusion::dataframe::DataFrameWriteOptions;
use datafusion::prelude::SessionContext;
use glossql_catalog::Lake;
use glossql_glossary::{Actor, ActorKind, Store};
use glossql_scripts::RhaiRuntime;
use glossql_session::{Outcome, Session};

async fn parquet_fixture(root: &std::path::Path) {
    let schema = Arc::new(Schema::new(vec![
        Field::new("order_id", DataType::Int64, true),
        Field::new("amount", DataType::Utf8, true),
        Field::new("order_date", DataType::Utf8, true),
        Field::new("legacy_code", DataType::Utf8, true),
    ]));
    let batch = RecordBatch::try_new(Arc::clone(&schema), vec![
        Arc::new(Int64Array::from(vec![1, 2, 3, 4, 5])),
        Arc::new(StringArray::from(vec![
            "12.50", "8.00", "99.90", "7.25", "n/a",
        ])),
        Arc::new(StringArray::from(vec![
            "15.01.2024",
            "16.01.2024",
            "17.01.2024",
            "18.01.2024",
            "19.01.2024",
        ])),
        Arc::new(StringArray::from(vec![None::<&str>; 5])),
    ])
    .unwrap();
    let ctx = SessionContext::new();
    ctx.register_batch("t", batch).unwrap();
    ctx.table("t")
        .await
        .unwrap()
        .write_parquet(
            &root.join("orders").display().to_string(),
            DataFrameWriteOptions::new(),
            None,
        )
        .await
        .unwrap();
}

fn runtime() -> Arc<RhaiRuntime> {
    Arc::new(RhaiRuntime::new(env!("CARGO_MANIFEST_DIR")))
}

fn session_for(kind: ActorKind, id: &str, store: &Store, lake: &Lake) -> Session {
    Session::new(store.clone(), Actor {
        kind,
        id: id.into(),
    })
    .unwrap()
    .with_lake(lake.clone())
    .with_runtime(runtime())
}

fn one(outcomes: &[Outcome]) -> String {
    match outcomes.last().unwrap() {
        Outcome::Rows(batches) => {
            let rows: usize = batches.iter().map(|b| b.num_rows()).sum();
            assert_eq!(rows, 1, "expected one row");
            let batch = batches.iter().find(|b| b.num_rows() > 0).unwrap();
            datafusion::arrow::util::display::array_value_to_string(batch.column(0), 0).unwrap()
        }
        other => panic!("expected Rows, got {other:?}"),
    }
}

const DECLARATIONS: &str = r#"
DECLARE ASPECT type WITH $${
  "type": "object", "required": ["value"],
  "properties": {"value": {"type": "string"}, "expr": {"type": "string"}}
}$$ AS FACT;
DECLARE ASPECT type_candidates WITH $${
  "type": "object", "required": ["candidates"],
  "properties": {"candidates": {"type": "array"}}
}$$ AS MEASUREMENT;
DECLARE ASPECT type_patterns WITH $${
  "type": "object", "required": ["patterns"],
  "properties": {
    "min_confidence": {"type": "number"},
    "patterns": {"type": "array"}}
}$$ AS FACT;
DECLARE ASPECT behavior WITH $${
  "type": "object", "required": ["value"],
  "properties": {"value": {"enum": ["stock", "flow"]}}
}$$ AS FACT;
DECLARE ASPECT column_profile WITH $${"type": "object"}$$ AS MEASUREMENT;
DECLARE ASPECT outlier_profile WITH $${"type": "object"}$$ AS MEASUREMENT;
DECLARE ASPECT eligible WITH $${
  "type": "object", "required": ["value"],
  "properties": {"value": {"type": "boolean"}, "reason": {"type": "string"}}
}$$ AS FACT;

DECLARE FUNCTION infer_types FOR GLOBAL FROM 'functions/infer_types.rhai'
  ACCEPTS (type_patterns)
  RETURNS $${"type": "object", "required": ["candidates"],
             "properties": {"candidates": {"type": "array"}}}$$;
DECLARE FUNCTION decide_types FOR GLOBAL FROM 'functions/decide_types.rhai'
  ACCEPTS (type_candidates, type_patterns)
  RETURNS $${"type": "object", "required": ["value"],
             "properties": {"value": {"type": "string"}, "expr": {"type": "string"}}}$$;
DECLARE FUNCTION profile FOR GLOBAL FROM 'functions/profile.rhai'
  RETURNS $${"type": "object"}$$;
DECLARE FUNCTION outliers FOR GLOBAL FROM 'functions/outliers.rhai'
  ACCEPTS (column_profile)
  RETURNS $${"type": "object", "required": ["applicable"]}$$;
DECLARE FUNCTION decide_eligibility FOR GLOBAL FROM 'functions/decide_eligibility.rhai'
  ACCEPTS (column_profile)
  RETURNS $${"type": "object", "required": ["value"],
             "properties": {"value": {"type": "boolean"}, "reason": {"type": "string"}}}$$;
DECLARE FUNCTION slot_entropy FOR GLOBAL FROM 'functions/slot_entropy.rhai'
  RETURNS $${
    "type": "object",
    "required": ["subject", "aspect", "witness", "band", "score", "computed_at"],
    "properties": {
      "subject": {"type": "string"}, "aspect": {"type": "string"},
      "witness": {"type": "string"},
      "band": {"enum": ["green", "yellow", "orange", "red"]},
      "score": {"type": "number", "minimum": 0, "maximum": 1},
      "computed_at": {"type": "string"}}
  }$$;

DECLARE WITNESS type_candidates_w ON type_candidates BY (FUNCTION infer_types);
DECLARE WITNESS column_profile_w ON column_profile BY (FUNCTION profile);
DECLARE WITNESS outlier_profile_w ON outlier_profile BY (FUNCTION outliers);
DECLARE WITNESS eligible_w ON eligible BY (FUNCTION decide_eligibility, AGENT, HUMAN)
  DETECTOR slot_entropy;
DECLARE WITNESS type_w ON type BY (FUNCTION decide_types, AGENT, HUMAN)
  DETECTOR slot_entropy;
DECLARE WITNESS behavior_w ON behavior BY (AGENT, HUMAN)
  DETECTOR slot_entropy THRESHOLD 0.7;

GLOSS type_patterns ON fin AS $${"min_confidence": 0.75, "patterns": [
  {"name": "eu_date", "pattern": "^\\d{1,2}\\.\\d{1,2}\\.\\d{4}$",
   "inferred_type": "DATE",
   "standardization_expr": "try_to_date(\"{col}\", '%d.%m.%Y')"}
]}$$;
"#;

#[tokio::test(flavor = "multi_thread")]
async fn fixture_11_with_real_scripts() {
    let dir = tempfile::tempdir().unwrap();
    let erp_root = dir.path().join("lake/erp");
    std::fs::create_dir_all(&erp_root).unwrap();
    parquet_fixture(&erp_root).await;

    let lake = Lake::open(&dir.path().join("catalog.db"), &dir.path().join("warehouse"))
        .await
        .unwrap();
    let store = Store::open_memory().await.unwrap();
    let agent = session_for(ActorKind::Agent, "agent-1", &store, &lake);

    agent
        .execute(&format!(
            "DECLARE DATASET fin SET (purpose: 'working-capital analysis');\n\
             USE fin;\n\
             DECLARE SOURCE erp_export SET (type: parquet, location: '{}');\n\
             DECLARE RECIPE orders ON fin FROM erp_export AS $$SELECT * FROM read_parquet('orders/*.parquet')$$;",
            erp_root.display()
        ))
        .await
        .unwrap();
    agent.execute(DECLARATIONS).await.unwrap();

    // Inference measures, the decision function picks — no agent in the
    // loop (ruled 2026-08-04): amount becomes DOUBLE by trial cast, the
    // date column becomes DATE through the taught pattern's expr.
    agent
        .execute(
            "SELECT infer_types() FROM orders.amount;\n\
             SELECT infer_types() FROM orders.order_date;\n\
             SELECT decide_types() FROM orders.amount;\n\
             SELECT decide_types() FROM orders.order_date;",
        )
        .await
        .unwrap();

    let decided = agent
        .execute("SELECT value FROM GLOSSARY(orders.amount::type);")
        .await
        .unwrap();
    assert!(one(&decided).contains("DOUBLE"), "{:?}", one(&decided));

    // The typed shape serves under the bare name; the dirty row is one
    // name away, in raw shape.
    let total = agent
        .execute("SELECT sum(amount) FROM orders;")
        .await
        .unwrap();
    assert_eq!(one(&total), "127.65");
    let quarantined = agent
        .execute("SELECT amount FROM orders_quarantined;")
        .await
        .unwrap();
    assert_eq!(one(&quarantined), "n/a");
    let earliest = agent
        .execute("SELECT min(order_date) FROM orders;")
        .await
        .unwrap();
    assert_eq!(one(&earliest), "2024-01-15", "the pattern expr parsed EU dates");

    // Analysis runs after typing; its evidence caches.
    agent
        .execute("SELECT profile() FROM orders.amount;")
        .await
        .unwrap();
    let cached = agent
        .execute("SELECT count(*) FROM cache WHERE function = 'profile';")
        .await
        .unwrap();
    assert_eq!(one(&cached), "1");

    // The outlier chain rides the profile: the fences come from the cached
    // quartiles and MAD, the door only counts what lies beyond them.
    agent
        .execute("SELECT outliers() FROM orders.amount;")
        .await
        .unwrap();
    let outlier = agent
        .execute("SELECT value FROM GLOSSARY(orders.amount::outlier_profile);")
        .await
        .unwrap();
    assert!(one(&outlier).contains("\"applicable\":true"), "{}", one(&outlier));
    assert!(one(&outlier).contains("\"count\":1"), "99.90 is beyond both fences: {}", one(&outlier));

    // An agent gloss supersedes the function's pick — and the typing
    // decision changing kills the table's evidence, sparing the typing
    // machinery (writes invalidate, reads recompute).
    agent
        .execute(r#"GLOSS type ON orders.amount AS $${"value": "DECIMAL(12,2)"}$$;"#)
        .await
        .unwrap();
    let served = agent
        .execute("SELECT value, state FROM GLOSSARY(orders.amount::type);")
        .await
        .unwrap();
    assert!(one(&served).contains("DECIMAL(12,2)"));
    let profile_rows = agent
        .execute("SELECT count(*) FROM cache WHERE function IN ('profile', 'outliers');")
        .await
        .unwrap();
    assert_eq!(one(&profile_rows), "0", "the table's evidence died with the decision");
    let machinery_rows = agent
        .execute("SELECT count(*) FROM cache WHERE function = 'infer_types';")
        .await
        .unwrap();
    assert_eq!(one(&machinery_rows), "2", "the typing machinery is exempt");
    let total = agent
        .execute("SELECT sum(amount) FROM orders;")
        .await
        .unwrap();
    assert_eq!(one(&total), "127.65", "the view followed the override");

    // No THRESHOLD on the type witness: an override never withholds the
    // value, but the disagreement with the function's pick shows as a band.
    let band = agent
        .execute("SELECT band FROM ATTEST(orders.amount::type);")
        .await
        .unwrap();
    assert_eq!(one(&band), "yellow", "override visible, never blocking");

    // A malformed expr is refused at admission — it would otherwise become
    // view SQL and brick every read.
    let err = agent
        .execute(r#"GLOSS type ON orders.amount AS $${"value": "DATE", "expr": "try_to_date(("}$$;"#)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("type expr"), "{err}");

    // The detector adjudicates at read: one slot is green, a human
    // disagreement turns it red and the collapsed value withholds.
    agent
        .execute(r#"GLOSS behavior ON orders.amount AS $${"value": "flow"}$$;"#)
        .await
        .unwrap();
    let band = agent
        .execute("SELECT band FROM ATTEST(orders.amount::behavior);")
        .await
        .unwrap();
    assert_eq!(one(&band), "green");

    let human = session_for(ActorKind::Human, "philipp", &store, &lake);
    human.execute("USE fin;").await.unwrap();
    human
        .execute(r#"GLOSS behavior ON orders.amount AS $${"value": "stock"}$$;"#)
        .await
        .unwrap();
    let band = human
        .execute("SELECT band, score FROM ATTEST(orders.amount::behavior);")
        .await
        .unwrap();
    assert_eq!(one(&band), "red");
    let contested = human
        .execute("SELECT value, state FROM GLOSSARY(orders.amount::behavior);")
        .await
        .unwrap();
    let row = match contested.last().unwrap() {
        Outcome::Rows(batches) => {
            let b = batches.iter().find(|b| b.num_rows() > 0).unwrap();
            (
                b.column(0).is_null(0),
                datafusion::arrow::util::display::array_value_to_string(b.column(1), 0).unwrap(),
            )
        }
        other => panic!("expected Rows, got {other:?}"),
    };
    assert!(row.0, "a contested value is withheld");
    assert_eq!(row.1, "contested");

    // Disclosure: witnessed aspects nobody spoke to appear as rows.
    let unassessed = human
        .execute("SELECT count(*) FROM GLOSSARY(orders) WHERE state = 'unassessed';")
        .await
        .unwrap();
    assert_ne!(one(&unassessed), "0", "absence is a visible row, not an omission");

    // The teach flow, end to end with no orchestration: re-glossing the
    // patterns invalidates everything that ACCEPTS them; re-extraction
    // re-decides; the view follows.
    human
        .execute(
            r#"GLOSS type_patterns ON fin AS $${"min_confidence": 0.9, "patterns": []}$$;"#,
        )
        .await
        .unwrap();
    let inference_rows = human
        .execute("SELECT count(*) FROM cache WHERE function IN ('infer_types', 'decide_types');")
        .await
        .unwrap();
    assert_eq!(one(&inference_rows), "0", "the teach invalidated its dependents");
    let earliest = human
        .execute("SELECT min(order_date) FROM orders;")
        .await
        .unwrap();
    assert_eq!(
        one(&earliest),
        "15.01.2024",
        "no decision, no cast — the column rides raw again"
    );
    human
        .execute(
            "SELECT infer_types() FROM orders.order_date;\n\
             SELECT decide_types() FROM orders.order_date;",
        )
        .await
        .unwrap();
    let redecided = human
        .execute("SELECT value FROM GLOSSARY(orders.order_date::type);")
        .await
        .unwrap();
    assert!(
        one(&redecided).contains("VARCHAR"),
        "min_confidence 0.9 with no patterns falls back: {}",
        one(&redecided)
    );

    // Eligibility is its own gloss (ruled 2026-08-04): the all-null
    // column's pick drops it from the typed view — raw and the glossary
    // keep it, and a superseding gloss brings it back. v0.3's irreversible
    // ALTER-drop, respelled as a reversible projection.
    human
        .execute(
            "SELECT profile() FROM orders.legacy_code;\n\
             SELECT decide_eligibility() FROM orders.legacy_code;",
        )
        .await
        .unwrap();
    let pick = human
        .execute("SELECT value FROM GLOSSARY(orders.legacy_code::eligible);")
        .await
        .unwrap();
    assert!(one(&pick).contains("false"), "{}", one(&pick));
    let gone = human
        .execute("SELECT legacy_code FROM orders;")
        .await
        .unwrap_err();
    assert!(gone.to_string().contains("legacy_code"), "{gone}");
    let kept = human
        .execute("SELECT count(legacy_code) FROM orders_raw;")
        .await
        .unwrap();
    assert_eq!(one(&kept), "0", "raw keeps the column; every value is NULL");

    human
        .execute(
            r#"GLOSS eligible ON orders.legacy_code AS $${"value": true, "reason": "kept for audit"}$$;"#,
        )
        .await
        .unwrap();
    let back = human
        .execute("SELECT count(legacy_code) FROM orders;")
        .await
        .unwrap();
    assert_eq!(one(&back), "0", "the superseding gloss restored the column");
    let band = human
        .execute("SELECT band FROM ATTEST(orders.legacy_code::eligible);")
        .await
        .unwrap();
    assert_eq!(one(&band), "yellow", "the disagreement with the pick stays visible");
}
