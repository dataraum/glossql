//! Recipe execution: source files in, Arrow batches out (SPEC.md §3).
//!
//! A recipe at a file source runs on the server: the recipe SQL executes in
//! a scratch DataFusion context where `read_parquet` / `read_csv` /
//! `read_json` resolve under the source's `location` root, and
//! `try_to_date`/`try_to_timestamp` are registered — the recipe carries the
//! casts (project lead, 2026-08-04). A probe is the same SQL surface
//! without a landing: paths' first segment names the source. A recipe at a
//! relational source runs its SQL at the source — that executor (ADBC) is
//! planned, not built; declaring such a source stores it, running its
//! recipe errors.

pub mod casts;
mod normalize;

use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

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
use futures::StreamExt as _;
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

/// What a recipe run landed, plus what it read: `source_rows` is the row
/// count of every source relation the recipe scanned, so the caller can
/// record `source_rows - landed` as the dropped-row count (project lead,
/// 2026-08-04 — which rows were dropped is the author's question, answered
/// on the files).
#[derive(Debug)]
pub struct Landed {
    pub schema: SchemaRef,
    pub batches: Vec<RecordBatch>,
    pub source_rows: u64,
}

/// Run a recipe against its source and return the batches that will land
/// as the table — exactly the schema the recipe's SQL produced (the
/// probe's rehearsed identity), folded only where Iceberg v2 cannot hold
/// a type. Typing is authored (ruled 2026-08-04): an uncast csv/json
/// column is Utf8 because the read side is, never because the import
/// refolds it.
pub async fn run_recipe(spec: &SourceSpec, sql: &str) -> Result<Landed> {
    if spec.kind == SourceKind::RelationalDb {
        return Err(Error::RelationalSource(spec.name.clone()));
    }
    let root = canonical_root(spec)?;

    let ctx = SessionContext::new();
    casts::register_try_functions(&ctx);
    let seen: Scanned = Arc::default();
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
                seen: Some(Arc::clone(&seen)),
            }),
        );
    }

    let df = ctx.sql_with_options(sql, read_only()).await?;
    let schema: SchemaRef = Arc::new(df.schema().as_arrow().clone());
    let batches = df.collect().await?;

    let mut source_rows = 0u64;
    let scanned = std::mem::take(&mut *seen.lock().expect("seen"));
    for provider in scanned {
        source_rows += ctx.read_table(provider)?.count().await? as u64;
    }

    let (schema, batches) = normalize::compat(schema, batches)?;
    Ok(Landed {
        schema,
        batches,
        source_rows,
    })
}

/// Run a probe: a recipe rehearsal (`PROBE source AS $$sql$$`) — the same
/// SQL surface, the same path resolution, landing nothing. The result
/// carries the schema the recipe would land, so `LIMIT 0` rehearses the
/// identity a `DECLARE RECIPE` would stamp.
pub async fn run_probe(spec: &SourceSpec, sql: &str, row_cap: usize) -> Result<Vec<RecordBatch>> {
    if spec.kind == SourceKind::RelationalDb {
        return Err(Error::RelationalSource(spec.name.clone()));
    }
    let root = canonical_root(spec)?;
    let ctx = SessionContext::new();
    casts::register_try_functions(&ctx);
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
                seen: None,
            }),
        );
    }
    let df = ctx.sql_with_options(sql, read_only()).await?;
    let schema: SchemaRef = Arc::new(df.schema().as_arrow().clone());
    // A rehearsal is read at the door like any other answer, so it stops at
    // the door's cap — a probe without a LIMIT used to pull the whole
    // source into memory to show 200 rows of it.
    let mut stream = df.execute_stream().await?;
    let mut batches = Vec::new();
    let mut rows = 0usize;
    while let Some(batch) = stream.next().await {
        let batch = batch?;
        rows += batch.num_rows();
        batches.push(batch);
        if rows > row_cap {
            break;
        }
    }
    if batches.is_empty() {
        // An empty result still carries the shape — the whole point of a
        // `LIMIT 0` rehearsal.
        batches.push(RecordBatch::new_empty(schema));
    }
    Ok(batches)
}

/// Recipe and probe SQL is a read at its source: it selects from the
/// `read_*` table functions and nothing else. Without this, DataFusion's
/// default options let a body `COPY` to any path the process can write
/// (found 2026-08-06) — the statement allowlist never sees this SQL.
fn read_only() -> datafusion::prelude::SQLOptions {
    datafusion::prelude::SQLOptions::new()
        .with_allow_ddl(false)
        .with_allow_dml(false)
        .with_allow_statements(false)
}

fn canonical_root(spec: &SourceSpec) -> Result<PathBuf> {
    spec.location.canonicalize().map_err(|e| Error::BadSource {
        name: spec.name.clone(),
        detail: format!("location {}: {e}", spec.location.display()),
    })
}

/// Providers a recipe run scanned, recorded so source rows can be counted.
type Scanned = Arc<Mutex<Vec<Arc<dyn TableProvider>>>>;

/// `read_parquet('…') | read_csv('…') | read_json('…')` — one file format,
/// rooted at the source's location. CSV reads with an all-Utf8 schema so
/// raw text survives byte-exact (no inferred typing to undo); parquet and
/// json read as the files are typed. When `seen` is set, every provider
/// built is recorded so the caller can count source rows.
#[derive(Debug)]
struct ReadFiles {
    root: PathBuf,
    kind: SourceKind,
    seen: Option<Scanned>,
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
        // `..` is not the only way out: a symlink under the root resolves
        // wherever it points. Check the deepest real directory the path
        // names — everything before the first glob segment.
        let mut real = self.root.clone();
        for component in rel_path.components() {
            if component.as_os_str().to_string_lossy().contains(['*', '?', '[']) {
                break;
            }
            real.push(component);
        }
        if let Ok(resolved) = real.canonicalize()
            && !resolved.starts_with(&self.root)
        {
            return Err(plan_err(format!(
                "`{rel}` resolves outside the source's location"
            )));
        }

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
        let provider: Arc<dyn TableProvider> = Arc::new(ListingTable::try_new(config)?);
        if let Some(seen) = &self.seen {
            seen.lock().expect("seen").push(Arc::clone(&provider));
        }
        Ok(provider)
    }
}
