//! `Session`: the per-connection statement router.

use std::sync::{Arc, RwLock};

use datafusion::arrow::record_batch::RecordBatch;
use datafusion::catalog::{CatalogProvider, SchemaProvider, TableProvider};
use datafusion::common::DataFusionError;
use datafusion::datasource::MemTable;
use datafusion::execution::session_state::SessionStateBuilder;
use datafusion::prelude::{SessionConfig, SessionContext};
use datafusion::sql::parser::Statement as DFStatement;
use datafusion::sql::sqlparser::ast::{FromTable, Statement as SQLStatement, TableFactor};
use datafusion::sql::sqlparser::parser::ParserError;
use serde_json::Value;

use glossql_catalog::Lake;
use glossql_glossary::{Actor, FunctionRow, RecipeAdmission, Store, schemas};
use glossql_import::SourceSpec;
use glossql_parser::{Declaration, Extract, Gloss, GlossqlParser, RelOp, Statement, Subject};

use crate::reads::{GlossqlReads, Shared};
use crate::subject::{Resolved, pair_subject, resolve_endpoint, resolve_path};

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("parse: {0}")]
    Parse(#[from] ParserError),
    #[error(transparent)]
    Store(#[from] glossql_glossary::Error),
    #[error(transparent)]
    DataFusion(#[from] DataFusionError),
    #[error("no dataset in use — USE one first")]
    NoDataset,
    #[error("not a subject: {0}")]
    BadSubject(String),
    #[error("unknown function `{0}` — DECLARE it (or check its FOR scope)")]
    UnknownFunction(String),
    #[error("output of `{function}` violates its RETURNS contract: {detail}")]
    OutputRejected { function: String, detail: String },
    #[error("function runtime: {0}")]
    Runtime(String),
    #[error(transparent)]
    Lake(#[from] glossql_catalog::Error),
    #[error(transparent)]
    Import(#[from] glossql_import::Error),
}

/// What one statement produced. `Rows` for anything that reads, `Affected`
/// for forwarded deletes, `Done` for declarations and writes.
#[derive(Debug)]
pub enum Outcome {
    Done(String),
    Rows(Vec<RecordBatch>),
    Affected(u64),
}

/// The query capability every measurement invocation receives (SPEC.md §6 —
/// scripts run any SQL against the dataset). Sync because scripts are; the
/// session implements it over its context with the block-in-place bridge
/// the reads already use. Detectors get a door that refuses (§7.1).
pub trait SqlDoor: Send + Sync {
    fn sql(&self, query: &str) -> Result<Vec<RecordBatch>, String>;
}

/// The seam scripts plug into (rhai + arrow kernels, `glossql-scripts`).
/// `context` is the document the server assembled from the function's
/// `ACCEPTS` aspects (SPEC.md §6) — or, for a detector, its slots and
/// threshold (§7.1). Tests inject fakes.
pub trait FunctionRuntime: Send + Sync + std::fmt::Debug {
    fn invoke(
        &self,
        function: &FunctionRow,
        subject: &str,
        context: &Value,
        door: Arc<dyn SqlDoor>,
    ) -> Result<Value, String>;
}

#[derive(Debug)]
pub struct NoRuntime;

impl FunctionRuntime for NoRuntime {
    fn invoke(
        &self,
        function: &FunctionRow,
        _: &str,
        _: &Value,
        _: Arc<dyn SqlDoor>,
    ) -> Result<Value, String> {
        Err(format!(
            "no function runtime configured — `{}` cannot run without scripts",
            function.name
        ))
    }
}

/// The session's own door: statements run against its context, so scripts
/// see the mounted lake tables, the derived views, and the read relations.
struct CtxDoor {
    ctx: SessionContext,
    handle: tokio::runtime::Handle,
}

impl SqlDoor for CtxDoor {
    fn sql(&self, query: &str) -> Result<Vec<RecordBatch>, String> {
        tokio::task::block_in_place(|| {
            self.handle.block_on(async {
                self.ctx
                    .sql(query)
                    .await
                    .map_err(|e| e.to_string())?
                    .collect()
                    .await
                    .map_err(|e| e.to_string())
            })
        })
    }
}

pub struct Session {
    ctx: SessionContext,
    shared: Arc<Shared>,
    actor: Actor,
    /// Bare-name mounts of the `USE`'d dataset's raw tables in the default
    /// schema (`orders_raw`), so recipe tables and the derived views resolve
    /// side by side.
    aliased: RwLock<Vec<String>>,
    /// The derived pair's last-emitted SQL per logical table — regeneration
    /// happens at read, only when the emitted text changes.
    derived: RwLock<std::collections::HashMap<String, String>>,
}

impl Session {
    /// Must be called inside a multi-thread tokio runtime — read planning
    /// blocks in place on store queries.
    pub fn new(store: Store, actor: Actor) -> Result<Self, SessionError> {
        let shared = Arc::new(Shared {
            store,
            dataset: RwLock::new(None),
            handle: tokio::runtime::Handle::current(),
            lake: RwLock::new(None),
            runtime: RwLock::new(Arc::new(NoRuntime)),
        });
        let config = SessionConfig::new()
            .set_str("datafusion.sql_parser.dialect", "postgres")
            // Iceberg's arrow fields carry `PARQUET:field_id` metadata; a
            // cast in a derived view drops it logically but not physically,
            // and the aggregate schema check trips on the difference. The
            // knob exists for exactly this (datafusion-common config.rs:532).
            .set_bool(
                "datafusion.execution.skip_physical_aggregate_schema_check",
                true,
            );
        let state = SessionStateBuilder::new()
            .with_default_features()
            .with_config(config)
            .with_relation_planners(vec![Arc::new(GlossqlReads {
                shared: Arc::clone(&shared),
            })])
            .build();
        let mut ctx = SessionContext::new_with_state(state);
        datafusion_functions_json::register_all(&mut ctx)?;
        crate::typing::register_try_functions(&ctx);
        Ok(Session {
            ctx,
            shared,
            actor,
            aliased: RwLock::new(Vec::new()),
            derived: RwLock::new(Default::default()),
        })
    }

    pub fn with_runtime(self, runtime: Arc<dyn FunctionRuntime>) -> Self {
        *self.shared.runtime.write().expect("runtime lock") = runtime;
        self
    }

    /// Attach the workspace data plane: recipes materialize, `USE` mounts
    /// the dataset's tables, gloss and cache writes carry snapshot ids.
    pub fn with_lake(self, lake: Lake) -> Self {
        *self.shared.lake.write().expect("lake lock") = Some(lake);
        self
    }

    fn lake(&self) -> Option<Lake> {
        self.shared.lake()
    }

    fn door(&self) -> CtxDoor {
        CtxDoor {
            ctx: self.ctx.clone(),
            handle: self.shared.handle.clone(),
        }
    }

    /// Data-plane tables come from recipes at M3; until then (and in tests)
    /// they are registered directly.
    pub fn register_table(
        &self,
        name: &str,
        provider: Arc<dyn TableProvider>,
    ) -> Result<(), SessionError> {
        self.ctx.register_table(name, provider)?;
        Ok(())
    }

    pub async fn execute(&self, sql: &str) -> Result<Vec<Outcome>, SessionError> {
        let statements = GlossqlParser::parse_sql(sql)?;
        let mut outcomes = Vec::with_capacity(statements.len());
        for statement in statements {
            outcomes.push(match statement {
                Statement::Declare(d) => self.declare(*d).await?,
                Statement::Use(u) => self.use_dataset(&u.dataset.value).await?,
                Statement::Gloss(g) => self.gloss(g).await?,
                Statement::Extract(e) => self.extract(e).await?,
                Statement::Substrate(s) => self.substrate(*s).await?,
            });
        }
        Ok(outcomes)
    }

    async fn declare(&self, declaration: Declaration) -> Result<Outcome, SessionError> {
        let store = &self.shared.store;
        let done = match &declaration {
            Declaration::Source(d) => {
                store.declare_source(d).await?;
                format!("DECLARE SOURCE {}", d.name.value)
            }
            Declaration::Dataset(d) => {
                store.declare_dataset(d).await?;
                if let Some(lake) = self.lake() {
                    lake.ensure_namespace(&d.name.value).await?;
                    self.mount_schema(&d.name.value).await?;
                }
                format!("DECLARE DATASET {}", d.name.value)
            }
            Declaration::Recipe(d) => {
                let admission = store.declare_recipe(d).await?;
                let (dataset, table) = (d.dataset.value.as_str(), d.table.value.as_str());
                match self.lake() {
                    None => format!("DECLARE RECIPE {table} ON {dataset}"),
                    Some(lake)
                        if admission == RecipeAdmission::Unchanged
                            && lake.table_exists(dataset, &raw_name(table)).await? =>
                    {
                        format!("DECLARE RECIPE {table} ON {dataset} (unchanged)")
                    }
                    Some(_) => {
                        let rows = self
                            .materialize(dataset, table, &d.source.value, &d.sql)
                            .await?;
                        format!("DECLARE RECIPE {table} ON {dataset} ({rows} rows)")
                    }
                }
            }
            Declaration::Relationship(d) => {
                let (left, op, right) = self.pair(&d.left, d.op, &d.right).await?;
                store
                    .declare_relationship(&left.dataset, &left.subject, op, &right.subject)
                    .await?;
                format!(
                    "DECLARE RELATIONSHIP {} {op} {}",
                    left.subject, right.subject
                )
            }
            Declaration::Aspect(d) => {
                store.declare_aspect(d).await?;
                format!("DECLARE ASPECT {}", d.name.value)
            }
            Declaration::Function(d) => {
                store.declare_function(d).await?;
                format!("DECLARE FUNCTION {}", d.name.value)
            }
            Declaration::Witness(d) => {
                store.declare_witness(d).await?;
                format!("DECLARE WITNESS {}", d.name.value)
            }
        };
        Ok(Outcome::Done(done))
    }

    async fn use_dataset(&self, name: &str) -> Result<Outcome, SessionError> {
        if !self.shared.store.dataset_exists(name).await? {
            return Err(SessionError::Store(glossql_glossary::Error::Unknown {
                what: "dataset",
                name: name.into(),
            }));
        }
        *self.shared.dataset.write().expect("state lock") = Some(name.to_string());
        if let Some(lake) = self.lake() {
            lake.ensure_namespace(name).await?;
            let schema = self.mount_schema(name).await?;
            let stale: Vec<String> = std::mem::take(&mut *self.aliased.write().expect("aliases"));
            for old in stale {
                let _ = self.ctx.deregister_table(old.as_str());
            }
            self.derived.write().expect("derived").clear();
            for table in lake.table_names(name).await? {
                self.alias(&table, &schema).await?;
            }
            self.refresh_derived().await?;
        }
        Ok(Outcome::Done(format!("USE {name}")))
    }

    /// Derivation at read (project lead, 2026-08-04): the bare table name is
    /// always a view — identity while nothing is decided, the typed
    /// projection as decisions land — with `<t>_quarantined` beside it.
    /// Nothing regenerates on write; before statements plan, the emitted SQL
    /// is recompared and `CREATE OR REPLACE VIEW` runs only on change.
    async fn refresh_derived(&self) -> Result<(), SessionError> {
        let (Some(lake), Some(dataset)) = (
            self.lake(),
            self.shared.dataset.read().expect("state lock").clone(),
        ) else {
            return Ok(());
        };
        for raw in lake.table_names(&dataset).await? {
            let Some(logical) = raw.strip_suffix("_raw") else {
                continue;
            };
            let columns = lake.table_columns(&dataset, &raw).await?;
            if columns.is_empty() {
                continue;
            }
            let decisions =
                crate::typing::decisions(&self.shared.store, &dataset, logical).await?;
            let (typed, quarantine) =
                crate::typing::pair_sql(logical, &raw, &columns, &decisions);
            let emitted = format!("{typed}\n{quarantine}");
            if self.derived.read().expect("derived").get(logical) == Some(&emitted) {
                continue;
            }
            self.ctx.sql(&typed).await?.collect().await?;
            self.ctx.sql(&quarantine).await?.collect().await?;
            self.derived
                .write()
                .expect("derived")
                .insert(logical.to_string(), emitted);
        }
        Ok(())
    }

    /// Land a recipe as its table: run it at the source, create the table
    /// through the mounted schema (live — no rebuild), append the batches
    /// through DataFusion's INSERT path, one snapshot per materialization.
    async fn materialize(
        &self,
        dataset: &str,
        table: &str,
        source: &str,
        sql: &str,
    ) -> Result<usize, SessionError> {
        const STAGED: &str = "__glossql_staged";
        let raw = raw_name(table);
        let lake = self.lake().expect("caller holds a lake");
        let settings = self.shared.store.source_settings(source).await?.ok_or(
            SessionError::Store(glossql_glossary::Error::Unknown {
                what: "source",
                name: source.into(),
            }),
        )?;
        let spec = SourceSpec::from_settings(source, &settings)?;
        let (schema, batches) = glossql_import::run_recipe(&spec, sql).await?;
        let rows: usize = batches.iter().map(|b| b.num_rows()).sum();

        lake.ensure_namespace(dataset).await?;
        let mounted = self.mount_schema(dataset).await?;
        if mounted.table_exist(&raw) {
            // a replaced recipe rebuilds its table (admission already ruled)
            mounted.deregister_table(&raw)?;
        }
        let empty = RecordBatch::new_empty(Arc::clone(&schema));
        let shape = MemTable::try_new(Arc::clone(&schema), vec![vec![empty]])?;
        mounted.register_table(raw.clone(), Arc::new(shape))?;

        if rows > 0 {
            let staged = MemTable::try_new(schema, vec![batches])?;
            self.ctx.register_table(STAGED, Arc::new(staged))?;
            let insert = format!("INSERT INTO \"{dataset}\".\"{raw}\" SELECT * FROM {STAGED}");
            let inserted = async {
                self.ctx.sql(&insert).await?.collect().await?;
                Ok::<(), DataFusionError>(())
            }
            .await;
            let _ = self.ctx.deregister_table(STAGED);
            inserted?;
        }
        if self.shared.dataset.read().expect("state lock").as_deref() == Some(dataset) {
            self.alias(&raw, &mounted).await?;
            self.refresh_derived().await?;
        }
        Ok(rows)
    }

    /// The dataset's namespace as a schema in the session's default catalog
    /// — `fin.orders` resolves, views land beside it in the default schema.
    async fn mount_schema(&self, dataset: &str) -> Result<Arc<dyn SchemaProvider>, SessionError> {
        let default = self.ctx.catalog("datafusion").expect("default catalog");
        if let Some(existing) = default.schema(dataset) {
            return Ok(existing);
        }
        let lake = self.lake().expect("caller holds a lake");
        let provider = lake.provider().await?;
        let schema = provider.schema(dataset).ok_or_else(|| {
            SessionError::Lake(glossql_catalog::Error::Workspace(format!(
                "namespace `{dataset}` is missing from the catalog"
            )))
        })?;
        default.register_schema(dataset, Arc::clone(&schema))?;
        Ok(schema)
    }

    /// Mount `dataset.table` under its bare name in the default schema.
    async fn alias(
        &self,
        table: &str,
        schema: &Arc<dyn SchemaProvider>,
    ) -> Result<(), SessionError> {
        if let Some(provider) = schema.table(table).await? {
            let _ = self.ctx.deregister_table(table);
            self.ctx.register_table(table, provider)?;
            self.aliased.write().expect("aliases").push(table.to_string());
        }
        Ok(())
    }

    /// The subject's table snapshot at write time — `None` for dataset-level
    /// subjects, pair paths, tables the lake does not hold, or no lake.
    /// Subjects are logical names; the snapshot rides on the `_raw` table.
    async fn stamp(&self, resolved: &Resolved) -> Result<Option<i64>, SessionError> {
        let Some(lake) = self.lake() else {
            return Ok(None);
        };
        if resolved.subject == resolved.dataset || resolved.subject.contains(' ') {
            return Ok(None);
        }
        let table = resolved
            .subject
            .split('.')
            .next()
            .expect("subjects are non-empty");
        Ok(lake
            .snapshot_id(&resolved.dataset, &raw_name(table))
            .await?)
    }

    async fn gloss(&self, gloss: Gloss) -> Result<Outcome, SessionError> {
        let resolved = self.subject(&gloss.subject).await?;
        let snapshot = self.stamp(&resolved).await?;
        self.shared
            .store
            .gloss(
                &resolved.dataset,
                &self.actor,
                &gloss.aspect.value,
                &resolved.subject,
                &gloss.body,
                snapshot,
            )
            .await?;
        Ok(Outcome::Done(format!(
            "GLOSS {} ON {}",
            gloss.aspect.value, resolved.subject
        )))
    }

    /// Extraction (SPEC.md §6): first run computes and caches, later runs
    /// read the cache; re-running is `DELETE FROM cache WHERE …`. The
    /// context document holds one entry per `ACCEPTS` aspect: the nearest
    /// value walking up from the subject (subject, parent, dataset), null
    /// when nothing is glossed.
    async fn extract(&self, extract: Extract) -> Result<Outcome, SessionError> {
        self.refresh_derived().await?;
        let store = self.shared.store.clone();
        let resolved = self.subject(&extract.subject).await?;
        let mut results = Vec::new();
        for call in &extract.calls {
            let name = call.value.clone();
            let function = store
                .function(&name, Some(&resolved.dataset))
                .await?
                .ok_or_else(|| SessionError::UnknownFunction(name.clone()))?;
            let cached = store
                .cache_get(&resolved.dataset, &resolved.subject, &name)
                .await?;
            let row = match cached {
                Some(row) => row,
                None => {
                    let mut context = serde_json::Map::new();
                    for aspect in &function.accepts {
                        let value =
                            context_value(&store, &resolved.dataset, &resolved.subject, aspect)
                                .await?;
                        context.insert(aspect.clone(), value);
                    }
                    let context = Value::Object(context);
                    let output = self
                        .shared
                        .runtime()
                        .invoke(
                            &function,
                            &resolved.subject,
                            &context,
                            Arc::new(self.door()),
                        )
                        .map_err(SessionError::Runtime)?;
                    schemas::validate_instance(&function.returns, &output).map_err(|detail| {
                        SessionError::OutputRejected {
                            function: name.clone(),
                            detail,
                        }
                    })?;
                    let snapshot = self.stamp(&resolved).await?;
                    store
                        .cache_put(
                            &resolved.dataset,
                            &resolved.subject,
                            &name,
                            &output.to_string(),
                            snapshot,
                        )
                        .await?;
                    store
                        .cache_get(&resolved.dataset, &resolved.subject, &name)
                        .await?
                        .expect("row just written")
                }
            };
            results.push(row);
        }
        Ok(Outcome::Rows(vec![crate::reads::extraction_batch(results)]))
    }

    async fn substrate(&self, statement: DFStatement) -> Result<Outcome, SessionError> {
        // Removal is SQL (SPEC.md §5.2, §6): deletes on the store's two
        // relations run at the store. DataFusion cannot execute DML against
        // registered providers anyway.
        if let Some((target, text)) = store_delete(&statement) {
            let affected = self.shared.store.forward_delete(&target, &text).await?;
            return Ok(Outcome::Affected(affected));
        }
        self.refresh_derived().await?;
        let plan = self.ctx.state().statement_to_plan(statement).await?;
        let frame = self.ctx.execute_logical_plan(plan).await?;
        Ok(Outcome::Rows(frame.collect().await?))
    }

    async fn subject(&self, subject: &Subject) -> Result<Resolved, SessionError> {
        let use_dataset = self.shared.dataset.read().expect("state lock").clone();
        let use_dataset = use_dataset.as_deref();
        match subject {
            Subject::Path(p) => {
                let segments: Vec<String> = p.segments.iter().map(|i| i.value.clone()).collect();
                resolve_path(&self.shared.store, use_dataset, &segments).await
            }
            Subject::Pair(pair) => {
                let (left, op, right) = self.pair(&pair.left, pair.op, &pair.right).await?;
                Ok(Resolved {
                    dataset: left.dataset.clone(),
                    subject: pair_subject(&left, op, &right),
                })
            }
        }
    }

    async fn pair(
        &self,
        left: &glossql_parser::ColumnPath,
        op: RelOp,
        right: &glossql_parser::ColumnPath,
    ) -> Result<(Resolved, &'static str, Resolved), SessionError> {
        let use_dataset = self.shared.dataset.read().expect("state lock").clone();
        let use_dataset = use_dataset.as_deref();
        let store = &self.shared.store;
        let l = resolve_endpoint(store, use_dataset, &endpoint_segments(left)).await?;
        let r = resolve_endpoint(store, use_dataset, &endpoint_segments(right)).await?;
        if l.dataset != r.dataset {
            return Err(SessionError::BadSubject(format!(
                "pair path spans datasets `{}` and `{}`",
                l.dataset, r.dataset
            )));
        }
        let op = match op {
            RelOp::ManyToOne => "->",
            RelOp::OneToOne => "<->",
        };
        Ok((l, op, r))
    }
}

/// The Iceberg table behind a logical name (project lead, 2026-08-04):
/// recipes land `<t>_raw`; the bare name is always the derived view.
pub(crate) fn raw_name(table: &str) -> String {
    format!("{table}_raw")
}

fn endpoint_segments(path: &glossql_parser::ColumnPath) -> Vec<String> {
    let mut segments = Vec::new();
    if let Some(d) = &path.dataset {
        segments.push(d.value.clone());
    }
    segments.push(path.table.value.clone());
    segments.push(path.column.value.clone());
    segments
}

/// The nearest current value of `aspect`, walking up from the subject:
/// the subject itself, its parent, then the dataset. Null when nothing is
/// glossed — scripts are deterministic and handle absence themselves.
async fn context_value(
    store: &glossql_glossary::Store,
    dataset: &str,
    subject: &str,
    aspect: &str,
) -> Result<Value, SessionError> {
    let mut level = Some(subject.to_string());
    while let Some(current) = level {
        let scope = if current == dataset {
            glossql_glossary::Scope::Dataset
        } else {
            glossql_glossary::Scope::Subject(current.clone())
        };
        let rows = store
            .collapsed_read(dataset, &scope, Some(aspect), &Default::default())
            .await?;
        let target = if current == dataset {
            dataset
        } else {
            &current
        };
        if let Some(row) = rows.iter().find(|r| r.subject == target)
            && let Some(value) = &row.value
        {
            return Ok(serde_json::from_str(value).unwrap_or_else(|_| Value::String(value.clone())));
        }
        level = parent_of(&current, dataset);
    }
    Ok(Value::Null)
}

/// `orders.amount` → `orders` → the dataset; tables and pair paths step
/// straight to the dataset.
fn parent_of(subject: &str, dataset: &str) -> Option<String> {
    if subject == dataset {
        None
    } else if subject.contains(' ') || !subject.contains('.') {
        Some(dataset.to_string())
    } else {
        Some(subject.rsplit_once('.').expect("has a dot").0.to_string())
    }
}

/// `DELETE FROM glossary … | DELETE FROM cache …` → (target, verbatim SQL).
fn store_delete(statement: &DFStatement) -> Option<(String, String)> {
    let DFStatement::Statement(inner) = statement else {
        return None;
    };
    let SQLStatement::Delete(delete) = inner.as_ref() else {
        return None;
    };
    let tables = match &delete.from {
        FromTable::WithFromKeyword(t) | FromTable::WithoutKeyword(t) => t,
    };
    let [table] = tables.as_slice() else {
        return None;
    };
    let TableFactor::Table { name, .. } = &table.relation else {
        return None;
    };
    if name.0.len() != 1 {
        return None;
    }
    let target = name.0[0].as_ident()?.value.to_lowercase();
    (target == "glossary" || target == "cache").then(|| (target, inner.to_string()))
}
