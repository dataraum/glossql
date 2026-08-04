//! `GLOSSARY()` / `ATTEST()` and the `glossary` / `cache` relations, planned
//! through DataFusion's `RelationPlanner` seam. The planner sees the raw
//! `TableFactor` before default planning, so named arguments (`all => true`),
//! zero-argument sweeps, and pair paths (`a.b <-> c.d`) all decode here —
//! structurally, from the sqlparser AST, which is also why the JSON `->`
//! operator (datafusion-functions-json) never collides with pair paths:
//! inside these factors `->` never reaches expression planning.

use std::sync::{Arc, RwLock};

use datafusion::arrow::array::{ArrayRef, Float64Array, StringArray};
use datafusion::arrow::datatypes::{DataType, Field, Schema, SchemaRef};
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::common::{DataFusionError, Result as DFResult};
use datafusion::datasource::{MemTable, provider_as_source};
use datafusion::logical_expr::LogicalPlanBuilder;
use datafusion::logical_expr::planner::{
    PlannedRelation, RelationPlanner, RelationPlannerContext, RelationPlanning,
};
use datafusion::sql::sqlparser::ast::{
    BinaryOperator, DataType as SQLDataType, Expr as SQLExpr, FunctionArg, FunctionArgExpr,
    TableFactor, Value as SQLValue,
};

use glossql_catalog::Lake;
use glossql_glossary::{AttestRow, CollapsedRow, RawRow, ReadContext, Scope, Store, schemas};
use serde_json::{Value, json};

use crate::session::{FunctionRuntime, SessionError, SqlDoor};
use crate::subject::{pair_subject, resolve_endpoint, resolve_path};

/// State the planner shares with the router: the `USE`'d dataset, the data
/// plane, and the script runtime (reads run detectors).
#[derive(Debug)]
pub(crate) struct Shared {
    pub store: Store,
    pub dataset: RwLock<Option<String>>,
    pub handle: tokio::runtime::Handle,
    pub lake: RwLock<Option<Lake>>,
    pub runtime: RwLock<Arc<dyn FunctionRuntime>>,
    /// The read context is rebuilt from Iceberg metadata only when the data
    /// plane changed — materialization and `USE` clear it; reads reuse it.
    pub read_cache: RwLock<Option<ReadContext>>,
}

impl Shared {
    pub fn lake(&self) -> Option<Lake> {
        self.lake.read().expect("lake lock").clone()
    }

    pub fn runtime(&self) -> Arc<dyn FunctionRuntime> {
        Arc::clone(&self.runtime.read().expect("runtime lock"))
    }

    /// What the store cannot know (SPEC.md §5.3): the subjects that exist —
    /// the recipe tables and their columns — and each table's current
    /// snapshot. The disclosure grid and the staleness comparison ride on
    /// this.
    pub async fn read_context(&self) -> Result<ReadContext, SessionError> {
        if let Some(cached) = self.read_cache.read().expect("read cache").clone() {
            return Ok(cached);
        }
        let mut ctx = ReadContext::default();
        let (Some(lake), Some(dataset)) = (
            self.lake(),
            self.dataset.read().expect("state lock").clone(),
        ) else {
            return Ok(ctx);
        };
        for table in lake.table_names(&dataset).await? {
            if let Some(snapshot) = lake.snapshot_id(&dataset, &table).await? {
                ctx.snapshots.insert(table.clone(), snapshot);
            }
            for column in lake.table_columns(&dataset, &table).await? {
                ctx.universe.push(format!("{table}.{column}"));
            }
            ctx.universe.push(table);
        }
        *self.read_cache.write().expect("read cache") = Some(ctx.clone());
        Ok(ctx)
    }
}

/// What a detector gets instead of a SQL door: a refusal (SPEC.md §7.1 — a
/// detector receives the witness's slots and threshold, never table data).
struct DeniedDoor;

impl SqlDoor for DeniedDoor {
    fn sql(&self, _query: &str) -> Result<Vec<RecordBatch>, String> {
        Err("a detector sees slots and threshold, never table data (SPEC.md §7.1)".into())
    }
}

/// Detector freshness at read (project lead, 2026-08-04): a verdict missing
/// or older than the newest slot write recomputes here, is cached like any
/// function result, and `DELETE FROM cache` still forces it.
async fn ensure_verdicts(
    shared: &Shared,
    dataset: &str,
    scope: &Scope,
    aspect: Option<&str>,
) -> Result<(), SessionError> {
    for w in shared.store.witnesses_all().await? {
        if let Some(a) = aspect
            && w.aspect != a
        {
            continue;
        }
        let Some(detector) = w.detector.clone() else {
            continue;
        };
        let slots = shared.store.raw_read(dataset, scope, Some(&w.aspect)).await?;
        let mut newest: std::collections::BTreeMap<&str, &str> = Default::default();
        for s in &slots {
            let t = newest.entry(s.subject.as_str()).or_insert(&s.written_at);
            if s.written_at.as_str() > *t {
                *t = &s.written_at;
            }
        }
        for (subject, newest) in newest {
            let fresh = shared
                .store
                .cache_get(dataset, subject, &detector)
                .await?
                .is_some_and(|c| c.computed_at.as_str() >= newest);
            if fresh {
                continue;
            }
            let function = shared
                .store
                .function(&detector, Some(dataset))
                .await?
                .ok_or_else(|| SessionError::UnknownFunction(detector.clone()))?;
            let doc: Vec<Value> = slots
                .iter()
                .filter(|s| s.subject == subject)
                .map(|s| {
                    json!({
                        "speaker": s.speaker,
                        "actor": s.actor,
                        "body": serde_json::from_str::<Value>(&s.body)
                            .unwrap_or_else(|_| Value::String(s.body.clone())),
                        "written_at": s.written_at,
                    })
                })
                .collect();
            let context = json!({
                "subject": subject,
                "aspect": w.aspect,
                "witness": w.name,
                "slots": doc,
                "threshold": w.threshold,
            });
            let output = shared
                .runtime()
                .invoke(&function, subject, &context, Arc::new(DeniedDoor))
                .map_err(SessionError::Runtime)?;
            schemas::validate_instance(&function.returns, &output).map_err(|detail| {
                SessionError::OutputRejected {
                    function: detector.clone(),
                    detail,
                }
            })?;
            let snapshot = match (shared.lake(), glossary_table_of(subject)) {
                (Some(lake), Some(table)) => {
                    lake.snapshot_id(dataset, &format!("{table}_raw")).await?
                }
                _ => None,
            };
            shared
                .store
                .cache_put(dataset, subject, &detector, &output.to_string(), snapshot)
                .await?;
        }
    }
    Ok(())
}

/// The subject's table: its first path segment; pair paths have none.
fn glossary_table_of(subject: &str) -> Option<&str> {
    if subject.contains(' ') {
        return None;
    }
    subject.split('.').next()
}

#[derive(Debug)]
pub(crate) struct GlossqlReads {
    pub shared: Arc<Shared>,
}

impl RelationPlanner for GlossqlReads {
    fn plan_relation(
        &self,
        relation: TableFactor,
        _context: &mut dyn RelationPlannerContext,
    ) -> DFResult<RelationPlanning> {
        let TableFactor::Table {
            name, alias, args, ..
        } = &relation
        else {
            return Ok(RelationPlanning::Original(Box::new(relation)));
        };
        if name.0.len() != 1 {
            return Ok(RelationPlanning::Original(Box::new(relation)));
        }
        let Some(fname) = name.0[0].as_ident().map(|i| i.value.to_lowercase()) else {
            return Ok(RelationPlanning::Original(Box::new(relation)));
        };

        let batch = match (fname.as_str(), args) {
            ("glossary", Some(a)) => self.run(glossary_read(&self.shared, &a.args))?,
            ("attest", Some(a)) => self.run(attest_read(&self.shared, &a.args))?,
            // The store's relations, readable as plain tables; snapshot at
            // plan time, like every other read here.
            ("glossary" | "cache" | "imports", None) => {
                let table = fname.clone();
                self.run(async {
                    let rows = self.shared.store.relation_rows(&table).await?;
                    Ok(relation_batch(&table, rows))
                })?
            }
            _ => return Ok(RelationPlanning::Original(Box::new(relation))),
        };

        let provider = MemTable::try_new(batch.schema(), vec![vec![batch]])?;
        let plan = LogicalPlanBuilder::scan(
            format!("{fname}()"),
            provider_as_source(Arc::new(provider)),
            None,
        )?
        .build()?;
        Ok(RelationPlanning::Planned(Box::new(PlannedRelation::new(
            plan,
            alias.clone(),
        ))))
    }
}

impl GlossqlReads {
    /// Planning is sync; the store is async. Callers run inside the session's
    /// multi-thread runtime, so blocking in place is safe.
    fn run(
        &self,
        fut: impl Future<Output = Result<RecordBatch, SessionError>>,
    ) -> DFResult<RecordBatch> {
        tokio::task::block_in_place(|| self.shared.handle.block_on(fut))
            .map_err(|e| DataFusionError::External(Box::new(e)))
    }
}

// -- argument decoding ---------------------------------------------------

async fn glossary_read(shared: &Shared, args: &[FunctionArg]) -> Result<RecordBatch, SessionError> {
    let (subject, all) = split_args(args, true)?;
    let ((dataset, scope), aspect) = decode_scope(shared, subject).await?;
    let aspect = aspect.as_deref();
    if all {
        Ok(raw_batch(
            shared.store.raw_read(&dataset, &scope, aspect).await?,
        ))
    } else {
        ensure_verdicts(shared, &dataset, &scope, aspect).await?;
        Ok(collapsed_batch(
            shared
                .store
                .collapsed_read(&dataset, &scope, aspect, &shared.read_context().await?)
                .await?,
        ))
    }
}

async fn attest_read(shared: &Shared, args: &[FunctionArg]) -> Result<RecordBatch, SessionError> {
    let (subject, _) = split_args(args, false)?;
    let ((dataset, scope), aspect) = decode_scope(shared, subject).await?;
    ensure_verdicts(shared, &dataset, &scope, aspect.as_deref()).await?;
    Ok(attest_batch(
        shared
            .store
            .attest_read(&dataset, &scope, aspect.as_deref())
            .await?,
    ))
}

/// Split a read's argument list into (optional subject, `all` flag).
fn split_args(
    args: &[FunctionArg],
    allow_all: bool,
) -> Result<(Option<&SQLExpr>, bool), SessionError> {
    let mut subject = None;
    let mut all = false;
    for arg in args {
        match arg {
            FunctionArg::Unnamed(FunctionArgExpr::Expr(e)) if subject.is_none() => {
                subject = Some(e);
            }
            FunctionArg::ExprNamed {
                name: SQLExpr::Identifier(n),
                arg: FunctionArgExpr::Expr(v),
                ..
            } if allow_all && n.value.eq_ignore_ascii_case("all") && is_true(v) => {
                all = true;
            }
            FunctionArg::Named { name, arg, .. }
                if allow_all
                    && name.value.eq_ignore_ascii_case("all")
                    && matches!(arg, FunctionArgExpr::Expr(v) if is_true(v)) =>
            {
                all = true;
            }
            other => {
                return Err(SessionError::BadSubject(format!(
                    "unsupported read argument `{other}`"
                )));
            }
        }
    }
    Ok((subject, all))
}

fn is_true(e: &SQLExpr) -> bool {
    matches!(e, SQLExpr::Value(v) if v.value == SQLValue::Boolean(true))
}

/// Decode a read subject into ((dataset, scope), aspect-filter). No subject
/// sweeps the `USE`'d dataset. `subject::aspect` narrows the read to one
/// declared aspect — the postgres cast spelling, so it arrives as
/// `Expr::Cast` and never collides with path segments.
async fn decode_scope(
    shared: &Shared,
    subject: Option<&SQLExpr>,
) -> Result<((String, Scope), Option<String>), SessionError> {
    let use_dataset = shared.dataset.read().expect("state lock").clone();
    let use_dataset = use_dataset.as_deref();
    let store = &shared.store;
    let Some(expr) = subject else {
        let dataset = use_dataset.ok_or(SessionError::NoDataset)?;
        return Ok(((dataset.to_string(), Scope::Dataset), None));
    };

    // `subject::aspect` — on the whole subject (`(a.b <-> c.d)::x` included).
    let (expr, aspect) = match expr {
        SQLExpr::Cast {
            expr: inner,
            data_type,
            ..
        } => (unnest(inner), Some(aspect_name(store, data_type).await?)),
        other => (other, None),
    };

    if let Some(segments) = path_segments(expr) {
        // A bare aspect name is a common mistake (`GLOSSARY(dso)`): it would
        // silently read an empty table named like the aspect.
        if let [only] = segments.as_slice()
            && aspect.is_none()
            && !store.dataset_exists(only).await?
            && store.aspect(only).await?.is_some()
        {
            return Err(SessionError::BadSubject(format!(
                "`{only}` names an aspect, not a subject — read it as `subject::{only}`"
            )));
        }
        let resolved = resolve_path(store, use_dataset, &segments).await?;
        return Ok(((resolved.dataset.clone(), resolved.scope()), aspect));
    }

    if let SQLExpr::BinaryOp { left, op, right } = expr
        && let Some(op) = rel_op(op)
    {
        // `::` binds tighter than `<->`, so `a.b <-> c.d::x` carries the
        // aspect on the right endpoint; it belongs to the pair.
        let (right, aspect) = match (right.as_ref(), aspect) {
            (
                SQLExpr::Cast {
                    expr: inner,
                    data_type,
                    ..
                },
                None,
            ) => (unnest(inner), Some(aspect_name(store, data_type).await?)),
            (other, aspect) => (other, aspect),
        };
        let left_segments = path_segments(left)
            .ok_or_else(|| SessionError::BadSubject(format!("`{left}` in a pair path")))?;
        let right_segments = path_segments(right)
            .ok_or_else(|| SessionError::BadSubject(format!("`{right}` in a pair path")))?;
        let l = resolve_endpoint(store, use_dataset, &left_segments).await?;
        let r = resolve_endpoint(store, use_dataset, &right_segments).await?;
        let pair = pair_subject(&l, op, &r);
        return Ok(((l.dataset, Scope::Subject(pair)), aspect));
    }

    Err(SessionError::BadSubject(format!(
        "`{expr}` is not a subject"
    )))
}

/// The `::aspect` name: a bare custom "type" naming a declared aspect.
async fn aspect_name(store: &Store, data_type: &SQLDataType) -> Result<String, SessionError> {
    let SQLDataType::Custom(name, _) = data_type else {
        return Err(SessionError::BadSubject(format!(
            "`::{data_type}` — the part after `::` must name a declared aspect"
        )));
    };
    let ident = match name.0.as_slice() {
        [part] => part.as_ident(),
        _ => None,
    };
    let Some(aspect) = ident.map(|i| i.value.clone()) else {
        return Err(SessionError::BadSubject(format!(
            "`::{name}` — the part after `::` must name a declared aspect"
        )));
    };
    if store.aspect(&aspect).await?.is_none() {
        return Err(SessionError::Store(glossql_glossary::Error::Unknown {
            what: "aspect",
            name: aspect,
        }));
    }
    Ok(aspect)
}

fn unnest(e: &SQLExpr) -> &SQLExpr {
    match e {
        SQLExpr::Nested(inner) => unnest(inner),
        other => other,
    }
}

fn path_segments(e: &SQLExpr) -> Option<Vec<String>> {
    match e {
        SQLExpr::Identifier(i) => Some(vec![i.value.clone()]),
        SQLExpr::CompoundIdentifier(parts) => Some(parts.iter().map(|i| i.value.clone()).collect()),
        _ => None,
    }
}

fn rel_op(op: &BinaryOperator) -> Option<&'static str> {
    match op {
        BinaryOperator::Arrow => Some("->"),
        BinaryOperator::LtDashGt => Some("<->"),
        _ => None,
    }
}

// -- batch shapes --------------------------------------------------------

fn utf8(name: &str) -> Field {
    Field::new(name, DataType::Utf8, true)
}

fn batch(schema: SchemaRef, columns: Vec<ArrayRef>) -> RecordBatch {
    RecordBatch::try_new(schema, columns).expect("column shapes match the schema")
}

/// `(subject, aspect, value, band, score, state)` — SPEC.md §5.3, collapsed.
fn collapsed_batch(rows: Vec<CollapsedRow>) -> RecordBatch {
    let schema = Arc::new(Schema::new(vec![
        utf8("subject"),
        utf8("aspect"),
        utf8("value"),
        utf8("band"),
        Field::new("score", DataType::Float64, true),
        utf8("state"),
    ]));
    batch(
        schema,
        vec![
            Arc::new(StringArray::from_iter_values(
                rows.iter().map(|r| r.subject.as_str()),
            )),
            Arc::new(StringArray::from_iter_values(
                rows.iter().map(|r| r.aspect.as_str()),
            )),
            Arc::new(StringArray::from_iter(
                rows.iter().map(|r| r.value.as_deref()),
            )),
            Arc::new(StringArray::from_iter(
                rows.iter().map(|r| r.band.as_deref()),
            )),
            Arc::new(Float64Array::from_iter(rows.iter().map(|r| r.score))),
            Arc::new(StringArray::from_iter_values(
                rows.iter().map(|r| r.state.as_str()),
            )),
        ],
    )
}

/// `(subject, aspect, kind, witness, actor, body, written_at)` — §5.3, raw.
fn raw_batch(rows: Vec<RawRow>) -> RecordBatch {
    let schema = Arc::new(Schema::new(vec![
        utf8("subject"),
        utf8("aspect"),
        utf8("kind"),
        utf8("witness"),
        utf8("actor"),
        utf8("body"),
        utf8("written_at"),
    ]));
    batch(
        schema,
        vec![
            Arc::new(StringArray::from_iter_values(
                rows.iter().map(|r| r.subject.as_str()),
            )),
            Arc::new(StringArray::from_iter_values(
                rows.iter().map(|r| r.aspect.as_str()),
            )),
            Arc::new(StringArray::from_iter_values(
                rows.iter().map(|r| r.kind.as_str()),
            )),
            Arc::new(StringArray::from_iter(
                rows.iter().map(|r| r.witness.as_deref()),
            )),
            Arc::new(StringArray::from_iter_values(
                rows.iter().map(|r| r.actor.as_str()),
            )),
            Arc::new(StringArray::from_iter_values(
                rows.iter().map(|r| r.body.as_str()),
            )),
            Arc::new(StringArray::from_iter_values(
                rows.iter().map(|r| r.written_at.as_str()),
            )),
        ],
    )
}

/// `(subject, aspect, witness, band, score, computed_at)` — §7.2.
fn attest_batch(rows: Vec<AttestRow>) -> RecordBatch {
    let schema = Arc::new(Schema::new(vec![
        utf8("subject"),
        utf8("aspect"),
        utf8("witness"),
        utf8("band"),
        Field::new("score", DataType::Float64, false),
        utf8("computed_at"),
    ]));
    batch(
        schema,
        vec![
            Arc::new(StringArray::from_iter_values(
                rows.iter().map(|r| r.subject.as_str()),
            )),
            Arc::new(StringArray::from_iter_values(
                rows.iter().map(|r| r.aspect.as_str()),
            )),
            Arc::new(StringArray::from_iter_values(
                rows.iter().map(|r| r.witness.as_str()),
            )),
            Arc::new(StringArray::from_iter_values(
                rows.iter().map(|r| r.band.as_str()),
            )),
            Arc::new(Float64Array::from_iter_values(rows.iter().map(|r| r.score))),
            Arc::new(StringArray::from_iter_values(
                rows.iter().map(|r| r.computed_at.as_str()),
            )),
        ],
    )
}

/// What an extraction statement returns: one row per call, served from the
/// cache (whether this run computed it or a previous one did).
pub(crate) fn extraction_batch(rows: Vec<glossql_glossary::CacheRow>) -> RecordBatch {
    let schema = Arc::new(Schema::new(vec![
        utf8("function"),
        utf8("subject"),
        utf8("body"),
        utf8("computed_at"),
    ]));
    batch(
        schema,
        vec![
            Arc::new(StringArray::from_iter_values(
                rows.iter().map(|r| r.function.as_str()),
            )),
            Arc::new(StringArray::from_iter_values(
                rows.iter().map(|r| r.subject.as_str()),
            )),
            Arc::new(StringArray::from_iter_values(
                rows.iter().map(|r| r.body.as_str()),
            )),
            Arc::new(StringArray::from_iter_values(
                rows.iter().map(|r| r.computed_at.as_str()),
            )),
        ],
    )
}

fn relation_batch(table: &str, rows: Vec<Vec<Option<String>>>) -> RecordBatch {
    let names: &[&str] = match table {
        "glossary" => &[
            "dataset",
            "subject",
            "aspect",
            "actor_kind",
            "actor_id",
            "body",
            "written_at",
            "snapshot_id",
        ],
        "imports" => &[
            "dataset",
            "table_name",
            "source_rows",
            "landed_rows",
            "dropped_rows_count",
            "imported_at",
        ],
        _ => &[
            "dataset",
            "subject",
            "function",
            "body",
            "computed_at",
            "snapshot_id",
        ],
    };
    let schema = Arc::new(Schema::new(
        names.iter().map(|n| utf8(n)).collect::<Vec<_>>(),
    ));
    let columns = (0..names.len())
        .map(|i| Arc::new(StringArray::from_iter(rows.iter().map(|r| r[i].as_deref()))) as ArrayRef)
        .collect();
    batch(schema, columns)
}
