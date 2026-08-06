//! Recipes over file sources: the recipe's schema is the landed schema
//! (typing is authored), an uncast csv column stays byte-exact raw text,
//! paths cannot escape the source root.

use std::sync::Arc;

use datafusion::arrow::array::{Int64Array, RecordBatch, StringArray, TimestampNanosecondArray};
use datafusion::arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use datafusion::dataframe::DataFrameWriteOptions;
use datafusion::prelude::SessionContext;
use glossql_import::{SourceKind, SourceSpec, run_recipe};
use serde_json::json;

fn spec(kind: &str, root: &std::path::Path) -> SourceSpec {
    SourceSpec::from_settings(
        "src",
        &json!({"type": kind, "location": root.display().to_string()}),
    )
    .unwrap()
}

async fn write_parquet_fixture(dir: &std::path::Path) {
    let schema = Arc::new(Schema::new(vec![
        Field::new("order_id", DataType::Int64, true),
        Field::new(
            "ordered_at",
            DataType::Timestamp(TimeUnit::Nanosecond, None),
            true,
        ),
        Field::new("amount", DataType::Utf8, true),
    ]));
    let batch = RecordBatch::try_new(Arc::clone(&schema), vec![
        Arc::new(Int64Array::from(vec![1, 2])),
        Arc::new(TimestampNanosecondArray::from(vec![1_700_000_000_000_000_000i64, 1_700_000_100_000_000_000i64])),
        Arc::new(StringArray::from(vec!["12.50", "8.00"])),
    ])
    .unwrap();
    let ctx = SessionContext::new();
    ctx.register_batch("t", batch).unwrap();
    ctx.table("t")
        .await
        .unwrap()
        .write_parquet(
            &dir.join("orders").display().to_string(),
            DataFrameWriteOptions::new(),
            None,
        )
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn parquet_recipe_keeps_types_and_folds_ns_to_us() {
    let dir = tempfile::tempdir().unwrap();
    write_parquet_fixture(dir.path()).await;

    let landed = run_recipe(
        &spec("parquet", dir.path()),
        "SELECT * FROM read_parquet('orders/*.parquet')",
    )
    .await
    .unwrap();
    let (schema, batches) = (landed.schema, landed.batches);
    assert_eq!(landed.source_rows, 2, "the recipe scanned two source rows");

    assert_eq!(schema.field(0).data_type(), &DataType::Int64);
    assert_eq!(
        schema.field(1).data_type(),
        &DataType::Timestamp(TimeUnit::Microsecond, None),
        "ns folds to µs — TimestampNs is a format-v3 type"
    );
    assert_eq!(schema.field(2).data_type(), &DataType::Utf8);
    assert_eq!(batches.iter().map(|b| b.num_rows()).sum::<usize>(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn csv_typing_is_authored_uncast_stays_byte_exact() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("accounts.csv"),
        "account_no,balance\n00123,42.0\n00456,7.5\n",
    )
    .unwrap();

    // No casts authored: the read side is all-Utf8, so raw text lands
    // byte-exact — the author's default, not an import refold.
    let landed = run_recipe(
        &spec("csv", dir.path()),
        "SELECT * FROM read_csv('accounts.csv')",
    )
    .await
    .unwrap();
    assert!(
        landed.schema.fields().iter().all(|f| f.data_type() == &DataType::Utf8),
        "an uncast csv column is a string"
    );
    let col = landed.batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<StringArray>()
        .unwrap();
    assert_eq!(col.value(0), "00123", "no inferred typing — leading zeros survive");

    // Authored casts land typed: the landed table is the typed table
    // (ruled 2026-08-04) — the schema the probe rehearsed, not a refold
    // (the force_utf8 that discarded these casts died 2026-08-05).
    let landed = run_recipe(
        &spec("csv", dir.path()),
        "SELECT account_no, try_cast(balance AS DOUBLE) AS balance \
         FROM read_csv('accounts.csv')",
    )
    .await
    .unwrap();
    assert_eq!(landed.schema.field(0).data_type(), &DataType::Utf8);
    assert_eq!(
        landed.schema.field(1).data_type(),
        &DataType::Float64,
        "the authored cast is the landed type"
    );
    let balances = landed.batches[0]
        .column(1)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Float64Array>()
        .unwrap();
    assert_eq!(balances.value(0), 42.0, "numbers aggregate as numbers");
}

#[tokio::test(flavor = "multi_thread")]
async fn recipe_paths_cannot_escape_the_source_root() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.csv"), "x\n1\n").unwrap();

    let err = run_recipe(
        &spec("csv", dir.path()),
        "SELECT * FROM read_csv('../a.csv')",
    )
    .await
    .unwrap_err();
    assert!(err.to_string().contains("must stay under the source's location"));

    // A symlink under the root is the same escape by another spelling
    // (2026-08-06): the fence resolves the path before reading it.
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("secret.csv"), "x\n9\n").unwrap();
    std::os::unix::fs::symlink(outside.path(), dir.path().join("link")).unwrap();
    let err = run_recipe(
        &spec("csv", dir.path()),
        "SELECT * FROM read_csv('link/secret.csv')",
    )
    .await
    .unwrap_err();
    assert!(err.to_string().contains("outside the source's location"), "{err}");
}

#[tokio::test(flavor = "multi_thread")]
async fn relational_sources_error_until_the_adbc_executor_exists() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = spec("csv", dir.path());
    s = SourceSpec {
        kind: SourceKind::RelationalDb,
        ..s
    };
    let err = run_recipe(&s, "SELECT 1").await.unwrap_err();
    assert!(err.to_string().contains("ADBC"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_recipe_body_cannot_write_outside_its_read() {
    // Recipe and probe SQL runs in a scratch context that the statement
    // allowlist never sees, and DataFusion's default options permit DDL,
    // DML and COPY — a probe could write a parquet file anywhere the
    // process can (found 2026-08-06).
    let dir = tempfile::tempdir().unwrap();
    write_parquet_fixture(dir.path()).await;
    let spec = spec("parquet", dir.path());
    let escape = dir.path().join("escaped.parquet");

    let e = run_recipe(
        &spec,
        &format!(
            "COPY (SELECT 1 AS a) TO '{}' STORED AS PARQUET",
            escape.display()
        ),
    )
    .await
    .unwrap_err();
    assert!(e.to_string().to_lowercase().contains("copy"), "{e}");
    assert!(!escape.exists(), "nothing was written");

    let e = glossql_import::run_probe(
        &spec,
        &format!(
            "COPY (SELECT 1 AS a) TO '{}' STORED AS PARQUET",
            escape.display()
        ),
        200,
    )
    .await
    .unwrap_err();
    assert!(e.to_string().to_lowercase().contains("copy"), "{e}");
    assert!(!escape.exists(), "nothing was written");

    // Reading is untouched.
    let landed = run_recipe(&spec, "SELECT * FROM read_parquet('orders/*.parquet')")
        .await
        .unwrap();
    assert_eq!(landed.batches.iter().map(|b| b.num_rows()).sum::<usize>(), 2);
}
