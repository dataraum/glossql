//! The fixture-11 flow against a real warehouse (corpus/11-flow-add-source),
//! under the authored-typing ruling (2026-08-04): the agent probes the
//! source through the statement door, the recipe carries the casts, the
//! landed table IS the typed table. Glosses carry snapshot ids; recipe
//! identity is content; `DROP TABLE` and the substrate allowlist hold the
//! lifecycle rules.

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
        Arc::new(StringArray::from(vec!["12.50", "8.00", "n/a"])),
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
    session
        .execute(&format!(
            "DECLARE DATASET fin SET (purpose: 'working-capital analysis');\n\
             USE fin;\n\
             DECLARE SOURCE erp_export SET (type: parquet, location: '{}');",
            erp_root.display()
        ))
        .await
        .unwrap();

    // The probe: recipe-shaped SQL at the source, landing nothing — the
    // path's first segment names the source.
    let filled = session
        .execute(
            "SELECT count(\"amount\") FROM read_parquet('erp_export/orders/*.parquet');",
        )
        .await
        .unwrap();
    assert_eq!(single_value(&filled), "3");
    let parsed = session
        .execute(
            "SELECT count(try_cast(\"amount\" AS DOUBLE)) \
             FROM read_parquet('erp_export/orders/*.parquet');",
        )
        .await
        .unwrap();
    assert_eq!(single_value(&parsed), "2", "the probe finds one bad value");

    // Typing is authored: the recipe carries the casts, and this author
    // drops the unparseable row deliberately.
    let outcomes = session
        .execute(
            "DECLARE RECIPE orders ON fin FROM erp_export AS $$\
               SELECT order_id, try_cast(amount AS DOUBLE) AS amount \
               FROM read_parquet('orders/*.parquet') \
               WHERE try_cast(amount AS DOUBLE) IS NOT NULL$$;",
        )
        .await
        .unwrap();
    assert_eq!(done(&outcomes[0]), "DECLARE RECIPE orders ON fin (2 rows)");

    // The landed table is the typed table — no view, no raw twin.
    let total = session
        .execute("SELECT sum(amount) FROM orders;")
        .await
        .unwrap();
    assert_eq!(single_value(&total), "20.5");
    let qualified = session
        .execute("SELECT count(*) FROM fin.orders;")
        .await
        .unwrap();
    assert_eq!(single_value(&qualified), "2");

    // The engine keeps one number about what the recipe filtered away.
    let dropped = session
        .execute(
            "SELECT dropped_rows_count FROM imports WHERE table_name = 'orders';",
        )
        .await
        .unwrap();
    assert_eq!(single_value(&dropped), "1", "which row is the author's question");

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
    assert_ne!(single_value(&stamped), "", "column gloss carries the snapshot id");

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

    // §3: unchanged recipe is a no-op; a changed one is refused outright —
    // replacement is postponed.
    let redeclare = "DECLARE RECIPE orders ON fin FROM erp_export AS $$\
               SELECT order_id, try_cast(amount AS DOUBLE) AS amount \
               FROM read_parquet('orders/*.parquet') \
               WHERE try_cast(amount AS DOUBLE) IS NOT NULL$$;";
    let outcomes = session.execute(redeclare).await.unwrap();
    assert_eq!(done(&outcomes[0]), "DECLARE RECIPE orders ON fin (unchanged)");
    let changed = "DECLARE RECIPE orders ON fin FROM erp_export AS $$SELECT order_id FROM read_parquet('orders/*.parquet')$$;";
    let err = session.execute(changed).await.unwrap_err();
    assert!(
        matches!(
            &err,
            SessionError::Store(glossql_glossary::Error::RecipeChanged { table }) if table == "orders"
        ),
        "{err}"
    );

    // The substrate allowlist: schema-altering SQL is refused at the door.
    let err = session
        .execute("CREATE VIEW shadow AS SELECT * FROM orders;")
        .await
        .unwrap_err();
    assert!(
        matches!(err, SessionError::SubstrateClosed(_)),
        "{err}"
    );
    let err = session
        .execute("INSERT INTO orders VALUES (9, 1.0);")
        .await
        .unwrap_err();
    assert!(matches!(err, SessionError::SubstrateClosed(_)), "{err}");

    // DROP TABLE refuses while the table holds data.
    let err = session.execute("DROP TABLE orders;").await.unwrap_err();
    assert!(
        matches!(&err, SessionError::DropRefused { reason, .. } if reason.contains("data")),
        "{err}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn drop_table_removes_an_empty_misdeclaration_whole() {
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
             DECLARE RECIPE mistake ON fin FROM erp AS $$\
               SELECT order_id FROM read_parquet('orders/*.parquet') WHERE false$$;",
            erp_root.display()
        ))
        .await
        .unwrap();

    let outcomes = session.execute("DROP TABLE mistake;").await.unwrap();
    assert_eq!(done(&outcomes[0]), "DROP TABLE mistake");

    // Gone whole: the table, the recipe row, the import record — so the
    // name is free for a different SQL.
    let gone = session
        .execute("SELECT count(*) FROM imports WHERE table_name = 'mistake';")
        .await
        .unwrap();
    assert_eq!(single_value(&gone), "0");
    let outcomes = session
        .execute(
            "DECLARE RECIPE mistake ON fin FROM erp AS $$SELECT order_id FROM read_parquet('orders/*.parquet')$$;",
        )
        .await
        .unwrap();
    assert_eq!(done(&outcomes[0]), "DECLARE RECIPE mistake ON fin (3 rows)");
}
