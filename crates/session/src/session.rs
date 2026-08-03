//! `Session`: the per-connection statement router.

use std::sync::{Arc, RwLock};

use datafusion::arrow::record_batch::RecordBatch;
use datafusion::catalog::TableProvider;
use datafusion::common::DataFusionError;
use datafusion::execution::session_state::SessionStateBuilder;
use datafusion::prelude::{SessionConfig, SessionContext};
use datafusion::sql::parser::Statement as DFStatement;
use datafusion::sql::sqlparser::ast::{FromTable, Statement as SQLStatement, TableFactor};
use datafusion::sql::sqlparser::parser::ParserError;
use serde_json::Value;

use glossql_glossary::{Actor, FunctionRow, Store, schemas};
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
}

/// What one statement produced. `Rows` for anything that reads, `Affected`
/// for forwarded deletes, `Done` for declarations and writes.
#[derive(Debug)]
pub enum Outcome {
    Done(String),
    Rows(Vec<RecordBatch>),
    Affected(u64),
}

/// The seam scripts plug into at M4 (rhai + arrow kernels). `context` is
/// the document the server assembled from the function's `ACCEPTS` aspects
/// (SPEC.md §6). M2 ships [`NoRuntime`]; tests inject fakes.
pub trait FunctionRuntime: Send + Sync + std::fmt::Debug {
    fn invoke(
        &self,
        function: &FunctionRow,
        subject: &str,
        context: &Value,
    ) -> Result<Value, String>;
}

#[derive(Debug)]
pub struct NoRuntime;

impl FunctionRuntime for NoRuntime {
    fn invoke(&self, function: &FunctionRow, _: &str, _: &Value) -> Result<Value, String> {
        Err(format!(
            "no function runtime configured — `{}` cannot run before scripts land (M4)",
            function.name
        ))
    }
}

pub struct Session {
    ctx: SessionContext,
    shared: Arc<Shared>,
    actor: Actor,
    runtime: Arc<dyn FunctionRuntime>,
}

impl Session {
    /// Must be called inside a multi-thread tokio runtime — read planning
    /// blocks in place on store queries.
    pub fn new(store: Store, actor: Actor) -> Result<Self, SessionError> {
        let shared = Arc::new(Shared {
            store,
            dataset: RwLock::new(None),
            handle: tokio::runtime::Handle::current(),
        });
        let config = SessionConfig::new().set_str("datafusion.sql_parser.dialect", "postgres");
        let state = SessionStateBuilder::new()
            .with_default_features()
            .with_config(config)
            .with_relation_planners(vec![Arc::new(GlossqlReads {
                shared: Arc::clone(&shared),
            })])
            .build();
        let mut ctx = SessionContext::new_with_state(state);
        datafusion_functions_json::register_all(&mut ctx)?;
        Ok(Session {
            ctx,
            shared,
            actor,
            runtime: Arc::new(NoRuntime),
        })
    }

    pub fn with_runtime(mut self, runtime: Arc<dyn FunctionRuntime>) -> Self {
        self.runtime = runtime;
        self
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
                format!("DECLARE DATASET {}", d.name.value)
            }
            Declaration::Recipe(d) => {
                store.declare_recipe(d).await?;
                format!("DECLARE RECIPE {} ON {}", d.table.value, d.dataset.value)
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
        Ok(Outcome::Done(format!("USE {name}")))
    }

    async fn gloss(&self, gloss: Gloss) -> Result<Outcome, SessionError> {
        let resolved = self.subject(&gloss.subject).await?;
        self.shared
            .store
            .gloss(
                &resolved.dataset,
                &self.actor,
                &gloss.aspect.value,
                &resolved.subject,
                &gloss.body,
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
                        .runtime
                        .invoke(&function, &resolved.subject, &context)
                        .map_err(SessionError::Runtime)?;
                    schemas::validate_instance(&function.returns, &output).map_err(|detail| {
                        SessionError::OutputRejected {
                            function: name.clone(),
                            detail,
                        }
                    })?;
                    store
                        .cache_put(
                            &resolved.dataset,
                            &resolved.subject,
                            &name,
                            &output.to_string(),
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
        let rows = store.collapsed_read(dataset, &scope, Some(aspect)).await?;
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
