//! Recipe execution: source files in, Arrow batches out (SPEC.md §3).
//!
//! A recipe at a file source runs on the server: the recipe SQL executes in
//! a scratch DataFusion context where `read_parquet` / `read_csv` /
//! `read_json` resolve under the source's `location` root. A recipe at a
//! relational source runs its SQL at the source — that executor (ADBC) is
//! planned, not built; declaring such a source stores it, running its
//! recipe errors.

mod normalize;

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use datafusion::arrow::array::RecordBatch;
use datafusion::arrow::datatypes::{DataType, Field, Schema, SchemaRef};
use datafusion::catalog::{Session, TableFunctionImpl, TableProvider};
use datafusion::datasource::file_format::FileFormat;
use datafusion::datasource::file_format::csv::CsvFormat;
use datafusion::datasource::file_format::json::JsonFormat;
use datafusion::datasource::file_format::parquet::ParquetFormat;
use datafusion::datasource::listing::{
    ListingOptions, ListingTable, ListingTableConfig, ListingTableUrl,
};
use datafusion::error::DataFusionError;
use datafusion::logical_expr::Expr;
use datafusion::prelude::SessionContext;
use datafusion::scalar::ScalarValue;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("source `{name}`: {detail}")]
    BadSource { name: String, detail: String },
    #[error(
        "source `{0}` is relational — its recipes run at the source, and that executor (ADBC) is not built yet"
    )]
    RelationalSource(String),
    #[error("recipe failed: {0}")]
    Recipe(#[from] DataFusionError),
    #[error("recipe result: {0}")]
    Batches(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Parquet,
    Csv,
    Json,
    RelationalDb,
}

/// A declared source, decoded from its stored `SET (…)` settings.
#[derive(Debug, Clone)]
pub struct SourceSpec {
    pub name: String,
    pub kind: SourceKind,
    /// File sources: the root directory recipe paths resolve under.
    pub location: PathBuf,
}

impl SourceSpec {
    pub fn from_settings(name: &str, settings: &serde_json::Value) -> Result<Self> {
        let get = |key: &str| {
            settings
                .get(key)
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::BadSource {
                    name: name.into(),
                    detail: format!("missing `{key}` in settings"),
                })
        };
        let kind = match get("type")? {
            "parquet" => SourceKind::Parquet,
            "csv" => SourceKind::Csv,
            "json" => SourceKind::Json,
            "relational_db" => SourceKind::RelationalDb,
            other => {
                return Err(Error::BadSource {
                    name: name.into(),
                    detail: format!("unknown type `{other}`"),
                });
            }
        };
        Ok(SourceSpec {
            name: name.into(),
            kind,
            location: PathBuf::from(get("location")?),
        })
    }
}

/// Run a recipe against its source and return the batches that will land as
/// the table — csv/json folded to the raw all-VARCHAR shape, parquet folded
/// to Iceberg-v2-compatible types.
pub async fn run_recipe(spec: &SourceSpec, sql: &str) -> Result<(SchemaRef, Vec<RecordBatch>)> {
    if spec.kind == SourceKind::RelationalDb {
        return Err(Error::RelationalSource(spec.name.clone()));
    }
    let root = spec
        .location
        .canonicalize()
        .map_err(|e| Error::BadSource {
            name: spec.name.clone(),
            detail: format!("location {}: {e}", spec.location.display()),
        })?;

    let ctx = SessionContext::new();
    for (fn_name, kind) in [
        ("read_parquet", SourceKind::Parquet),
        ("read_csv", SourceKind::Csv),
        ("read_json", SourceKind::Json),
    ] {
        ctx.register_udtf(
            fn_name,
            Arc::new(ReadFiles {
                root: root.clone(),
                kind,
            }),
        );
    }

    let df = ctx.sql(sql).await?;
    let schema: SchemaRef = Arc::new(df.schema().as_arrow().clone());
    let batches = df.collect().await?;
    match spec.kind {
        SourceKind::Csv | SourceKind::Json => normalize::force_utf8(schema, batches),
        _ => normalize::compat(schema, batches),
    }
}

/// `read_parquet('…') | read_csv('…') | read_json('…')` — one file format,
/// rooted at the source's location. CSV reads with an all-Utf8 schema so
/// raw text survives byte-exact (no inferred typing to undo); parquet and
/// json read as the files are typed.
#[derive(Debug)]
struct ReadFiles {
    root: PathBuf,
    kind: SourceKind,
}

impl TableFunctionImpl for ReadFiles {
    fn call(&self, args: &[Expr]) -> datafusion::error::Result<Arc<dyn TableProvider>> {
        let plan_err = |m: String| DataFusionError::Plan(m);
        let rel = match args {
            [Expr::Literal(ScalarValue::Utf8(Some(s)), _)] => s.clone(),
            _ => {
                return Err(plan_err(
                    "read_* takes exactly one string: a path or glob under the source's location"
                        .into(),
                ));
            }
        };
        let rel_path = Path::new(&rel);
        if rel_path.is_absolute()
            || rel_path
                .components()
                .any(|c| matches!(c, Component::ParentDir))
        {
            return Err(plan_err(format!(
                "`{rel}` must stay under the source's location — relative, no `..`"
            )));
        }
        let target = self.root.join(rel_path);

        let format: Arc<dyn FileFormat> = match self.kind {
            SourceKind::Parquet => Arc::new(ParquetFormat::default()),
            SourceKind::Csv => Arc::new(CsvFormat::default().with_has_header(true)),
            SourceKind::Json => Arc::new(JsonFormat::default()),
            SourceKind::RelationalDb => unreachable!("never registered"),
        };
        let mut options = ListingOptions::new(format);
        if rel.contains(['*', '?', '[']) {
            // the glob names the files; the extension filter would fight it
            options = options.with_file_extension("");
        }
        let url = ListingTableUrl::parse(target.display().to_string())?;

        let state = SessionContext::new().state();
        let inferred = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(options.infer_schema(&state as &dyn Session, &url))
        })?;
        let schema = if self.kind == SourceKind::Csv {
            Arc::new(Schema::new(
                inferred
                    .fields()
                    .iter()
                    .map(|f| Field::new(f.name(), DataType::Utf8, true))
                    .collect::<Vec<_>>(),
            ))
        } else {
            inferred
        };

        let config = ListingTableConfig::new(url)
            .with_listing_options(options)
            .with_schema(schema);
        Ok(Arc::new(ListingTable::try_new(config)?))
    }
}
