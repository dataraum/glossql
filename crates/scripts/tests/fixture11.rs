//! The acceptance test: fixture 11 end-to-end with real scripts, under the
//! authored-typing ruling (2026-08-04). The agent probes the source through
//! the statement door; the recipe carries the casts and the column choices;
//! the landed table IS the typed table. The measurement plane runs on it
//! (profile, outliers chained through ACCEPTS); semantic glosses and the
//! detector adjudicate as before; the collapsed read discloses state.

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
DECLARE ASPECT behavior WITH $${
  "type": "object", "required": ["value"],
  "properties": {"value": {"enum": ["stock", "flow"]}}
}$$ AS FACT;
DECLARE ASPECT column_profile WITH $${"type": "object"}$$ AS MEASUREMENT;
DECLARE ASPECT outlier_profile WITH $${"type": "object"}$$ AS MEASUREMENT;

DECLARE FUNCTION profile FOR GLOBAL FROM 'functions/profile.rhai'
  RETURNS $${"type": "object"}$$;
DECLARE FUNCTION outliers FOR GLOBAL FROM 'functions/outliers.rhai'
  ACCEPTS (column_profile)
  RETURNS $${"type": "object", "required": ["applicable"]}$$;
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

DECLARE WITNESS column_profile_w ON column_profile BY (FUNCTION profile);
DECLARE WITNESS outlier_profile_w ON outlier_profile BY (FUNCTION outliers);
DECLARE WITNESS behavior_w ON behavior BY (AGENT, HUMAN)
  DETECTOR slot_entropy THRESHOLD 0.7;
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
             DECLARE SOURCE erp_export SET (type: parquet, location: '{}');",
            erp_root.display()
        ))
        .await
        .unwrap();
    agent.execute(DECLARATIONS).await.unwrap();

    // The probe: the agent looks at the source before writing the recipe —
    // trial casts through the same statement door, landing nothing.
    let parsed = agent
        .execute(
            "SELECT count(try_cast(\"amount\" AS DOUBLE)) \
             FROM read_parquet('erp_export/orders/*.parquet');",
        )
        .await
        .unwrap();
    assert_eq!(one(&parsed), "4", "one amount does not read as a number");
    let dates = agent
        .execute(
            "SELECT count(try_to_date(\"order_date\", '%d.%m.%Y')) \
             FROM read_parquet('erp_export/orders/*.parquet');",
        )
        .await
        .unwrap();
    assert_eq!(one(&dates), "5", "the EU date pattern parses every row");
    let legacy = agent
        .execute(
            "SELECT count(\"legacy_code\") \
             FROM read_parquet('erp_export/orders/*.parquet');",
        )
        .await
        .unwrap();
    assert_eq!(one(&legacy), "0", "all null — the author will leave it out");

    // Typing is authored: the recipe carries the casts, keeps every row
    // (a bad amount is a NULL cell), and leaves the all-null column out.
    agent
        .execute(
            "DECLARE RECIPE orders ON fin FROM erp_export AS $$\
               SELECT order_id, \
                      try_cast(amount AS DOUBLE) AS amount, \
                      try_to_date(order_date, '%d.%m.%Y') AS order_date \
               FROM read_parquet('orders/*.parquet')$$;",
        )
        .await
        .unwrap();

    // The landed table is the typed table.
    let total = agent
        .execute("SELECT sum(amount) FROM orders;")
        .await
        .unwrap();
    assert_eq!(one(&total), "127.65");
    let earliest = agent
        .execute("SELECT min(order_date) FROM orders;")
        .await
        .unwrap();
    assert_eq!(one(&earliest), "2024-01-15", "the recipe's expr parsed EU dates");
    let gone = agent
        .execute("SELECT legacy_code FROM orders;")
        .await
        .unwrap_err();
    assert!(gone.to_string().contains("legacy_code"), "{gone}");
    let dropped = agent
        .execute("SELECT dropped_rows_count FROM imports WHERE table_name = 'orders';")
        .await
        .unwrap();
    assert_eq!(one(&dropped), "0", "this author kept every row");

    // The measurement plane runs on the served table; outliers chains on
    // the profile through ACCEPTS.
    agent
        .execute(
            "SELECT profile() FROM orders.amount;\n\
             SELECT outliers() FROM orders.amount;",
        )
        .await
        .unwrap();
    let outlier = agent
        .execute("SELECT value FROM GLOSSARY(orders.amount::outlier_profile);")
        .await
        .unwrap();
    assert!(one(&outlier).contains("\"applicable\":true"), "{}", one(&outlier));
    assert!(one(&outlier).contains("\"count\":1"), "99.90 is beyond both fences: {}", one(&outlier));

    // The ACCEPTS edge is the one declared invalidation: a fresh profile
    // kills the outlier cache.
    agent
        .execute(
            "DELETE FROM cache WHERE function = 'profile';\n\
             SELECT profile() FROM orders.amount;",
        )
        .await
        .unwrap();
    let stale = agent
        .execute("SELECT count(*) FROM cache WHERE function = 'outliers';")
        .await
        .unwrap();
    assert_eq!(one(&stale), "0", "a re-profile invalidated its dependent");

    // Semantic glosses and the detector at read: one slot green, a human
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

    // A typing correction is a recipe correction — and in the PoC the
    // loaded table refuses both replacement and removal.
    let err = human
        .execute(
            "DECLARE RECIPE orders ON fin FROM erp_export AS $$SELECT order_id FROM read_parquet('orders/*.parquet')$$;",
        )
        .await
        .unwrap_err();
    assert!(err.to_string().contains("different table"), "{err}");
    let err = human.execute("DROP TABLE orders;").await.unwrap_err();
    assert!(err.to_string().contains("holds data"), "{err}");
}
