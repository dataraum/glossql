//! The fixture-11 flow against a real warehouse (corpus/11-flow-add-source):
//! source declared, recipe lands a parquet-backed Iceberg table, bare and
//! qualified names resolve, the typed view works beside them, glosses carry
//! the table's snapshot id, and recipe re-declaration follows §3.

use std::sync::Arc;

use datafusion::arrow::array::{Int64Array, RecordBatch, StringArray};
use datafusion::arrow::datatypes::{DataType, Field, Schema};
use datafusion::dataframe::DataFrameWriteOptions;
use datafusion::prelude::SessionContext;
use glossql_catalog::Lake;
use glossql_glossary::{Actor, ActorKind, Store};
use glossql_session::{Outcome, Session, SessionError};

async fn parquet_fixture(root: &std::path::Path) {
    let schema = Arc::new(Schema::new(vec![
        Field::new("order_id", DataType::Int64, true),
        Field::new("amount", DataType::Utf8, true),
    ]));
    let batch = RecordBatch::try_new(Arc::clone(&schema), vec![
        Arc::new(Int64Array::from(vec![1, 2, 3])),
        Arc::new(StringArray::from(vec!["12.50", "8.00", "99.90"])),
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

async fn workspace(dir: &std::path::Path) -> Session {
    let lake = Lake::open(&dir.join("catalog.db"), &dir.join("warehouse"))
        .await
        .unwrap();
    let store = Store::open_memory().await.unwrap();
    Session::new(store, Actor {
        kind: ActorKind::Agent,
        id: "agent-1".into(),
    })
    .unwrap()
    .with_lake(lake)
}

fn done(outcome: &Outcome) -> &str {
    match outcome {
        Outcome::Done(s) => s,
        other => panic!("expected Done, got {other:?}"),
    }
}

fn single_value(outcomes: &[Outcome]) -> String {
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

#[tokio::test(flavor = "multi_thread")]
async fn fixture_11_add_source_flow() {
    let dir = tempfile::tempdir().unwrap();
    let erp_root = dir.path().join("lake/erp");
    std::fs::create_dir_all(&erp_root).unwrap();
    parquet_fixture(&erp_root).await;

    let session = workspace(dir.path()).await;
    let setup = format!(
        "DECLARE DATASET fin SET (purpose: 'working-capital analysis');\n\
         USE fin;\n\
         DECLARE SOURCE erp_export SET (type: parquet, location: '{}');\n\
         DECLARE RECIPE orders ON fin FROM erp_export AS $$SELECT * FROM read_parquet('orders/*.parquet')$$;",
        erp_root.display()
    );
    let outcomes = session.execute(&setup).await.unwrap();
    assert_eq!(done(&outcomes[3]), "DECLARE RECIPE orders ON fin (3 rows)");

    // Bare and dataset-qualified names both resolve to the Iceberg table.
    let n = session.execute("SELECT count(*) FROM orders;").await.unwrap();
    assert_eq!(single_value(&n), "3");
    let n = session
        .execute("SELECT count(*) FROM fin.orders;")
        .await
        .unwrap();
    assert_eq!(single_value(&n), "3");

    // The typed view (fixture 11) lands beside the data tables.
    session
        .execute(
            "CREATE VIEW orders_typed AS \
               SELECT CAST(order_id AS BIGINT) AS order_id, \
                      CAST(amount AS DECIMAL(12,2)) AS amount \
               FROM orders;",
        )
        .await
        .unwrap();
    let total = session
        .execute("SELECT sum(amount) FROM orders_typed;")
        .await
        .unwrap();
    assert_eq!(single_value(&total), "120.40");

    // A gloss on a column subject carries the table's snapshot id.
    session
        .execute(
            r#"DECLARE ASPECT unit WITH $${
                 "type": "object", "required": ["value"],
                 "properties": {"value": {"type": "string"}}
               }$$ AS FACT;
               GLOSS unit ON orders.amount AS $${"value": "EUR"}$$;"#,
        )
        .await
        .unwrap();
    let stamped = session
        .execute("SELECT snapshot_id FROM glossary WHERE aspect = 'unit';")
        .await
        .unwrap();
    let stamped = single_value(&stamped);
    assert_ne!(stamped, "", "column gloss carries the snapshot id");

    // A dataset-level gloss has no table to pin — snapshot id stays NULL.
    session
        .execute(r#"GLOSS unit ON fin AS $${"value": "EUR"}$$;"#)
        .await
        .unwrap();
    let unstamped = session
        .execute(
            "SELECT count(*) FROM glossary WHERE subject = 'fin' AND snapshot_id IS NULL;",
        )
        .await
        .unwrap();
    assert_eq!(single_value(&unstamped), "1");

    // §3: unchanged recipe is a no-op; changed is refused while glossed.
    let redeclare = "DECLARE RECIPE orders ON fin FROM erp_export AS $$SELECT * FROM read_parquet('orders/*.parquet')$$;";
    let outcomes = session.execute(redeclare).await.unwrap();
    assert_eq!(done(&outcomes[0]), "DECLARE RECIPE orders ON fin (unchanged)");

    let changed = "DECLARE RECIPE orders ON fin FROM erp_export AS $$SELECT order_id FROM read_parquet('orders/*.parquet')$$;";
    let err = session.execute(changed).await.unwrap_err();
    assert!(
        matches!(
            &err,
            SessionError::Store(glossql_glossary::Error::RecipeInUse { table, .. }) if table == "orders"
        ),
        "{err}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn changed_recipe_rematerializes_when_nothing_is_glossed() {
    let dir = tempfile::tempdir().unwrap();
    let erp_root = dir.path().join("lake/erp");
    std::fs::create_dir_all(&erp_root).unwrap();
    parquet_fixture(&erp_root).await;

    let session = workspace(dir.path()).await;
    session
        .execute(&format!(
            "DECLARE DATASET fin SET (purpose: 'test');\n\
             USE fin;\n\
             DECLARE SOURCE erp SET (type: parquet, location: '{}');\n\
             DECLARE RECIPE orders ON fin FROM erp AS $$SELECT * FROM read_parquet('orders/*.parquet')$$;",
            erp_root.display()
        ))
        .await
        .unwrap();

    let narrowed = "DECLARE RECIPE orders ON fin FROM erp AS $$SELECT order_id FROM read_parquet('orders/*.parquet')$$;";
    let outcomes = session.execute(narrowed).await.unwrap();
    assert_eq!(done(&outcomes[0]), "DECLARE RECIPE orders ON fin (3 rows)");

    // The rebuilt table has the narrowed shape, one column.
    let out = session.execute("SELECT * FROM orders;").await.unwrap();
    match out.last().unwrap() {
        Outcome::Rows(batches) => {
            assert_eq!(batches[0].num_columns(), 1);
        }
        other => panic!("expected Rows, got {other:?}"),
    }
}
