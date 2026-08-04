//! The rhai runtime behind [`FunctionRuntime`] (SPEC.md §6): measurements
//! and detectors as scripts, composing vectorized kernels on zero-copy
//! column handles — scripts orchestrate, they never iterate rows
//! (reports/2026-08-03-poc-substrate.md, the spike).
//!
//! Invocation contract: the script file evaluates with three scope
//! constants — `subject` (the extraction's subject path), `context` (the
//! `ACCEPTS` document, or slots + threshold for a detector), `db` (the SQL
//! door; a detector's door refuses) — and its final expression is the
//! result, converted to JSON and validated against `RETURNS` by the session.

use std::collections::HashMap;
use std::path::{Component, PathBuf};
use std::sync::{Arc, RwLock};

use datafusion::arrow::array::{Array, ArrayRef, Float64Array, RecordBatch, StringArray};
use datafusion::arrow::compute::kernels::aggregate;
use datafusion::arrow::compute::{CastOptions, cast_with_options};
use datafusion::arrow::datatypes::{DataType, TimeUnit};
use datafusion::arrow::util::display::array_value_to_string;
use glossql_glossary::FunctionRow;
use glossql_session::{FunctionRuntime, SqlDoor};
use rhai::{AST, Dynamic, Engine, EvalAltResult, Scope};
use serde_json::Value;

/// One engine, configured once; compiled ASTs cached per script (recompiled
/// when the file's text changes). Both are shareable because the crate is
/// built with rhai's `sync` feature.
pub struct RhaiRuntime {
    root: PathBuf,
    engine: Engine,
    asts: RwLock<HashMap<String, (String, Arc<AST>)>>,
}

impl std::fmt::Debug for RhaiRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RhaiRuntime")
            .field("root", &self.root)
            .finish()
    }
}

/// A table of batches, from the door.
#[derive(Debug, Clone)]
pub struct Table(Arc<Vec<RecordBatch>>);

/// A zero-copy column handle: cloning bumps the Arc, never the buffer.
#[derive(Debug, Clone)]
pub struct Col(ArrayRef);

#[derive(Clone)]
struct Door(Arc<dyn SqlDoor>);

type ScriptResult<T> = Result<T, Box<EvalAltResult>>;

fn fail<T>(message: impl Into<String>) -> ScriptResult<T> {
    Err(message.into().into())
}

impl RhaiRuntime {
    /// `root` is the workspace's functions directory; `FROM` paths resolve
    /// under it, fenced like import paths (no absolute, no `..`).
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let mut engine = Engine::new_raw();
        engine.register_global_module(
            rhai::packages::Package::as_shared_module(&rhai::packages::StandardPackage::new()),
        );
        // The default file resolver reads the filesystem and its base path
        // is not a jail (rhai-1.25.1 file.rs:272-290); no imports until a
        // corpus script needs them.
        engine.set_module_resolver(rhai::module_resolvers::DummyModuleResolver);
        // Runaway backstop, not a sandbox — scripts are workspace-trusted
        // (M2 ruling); every other limit keeps its default.
        engine.set_max_operations(50_000_000);

        engine
            .register_type_with_name::<Table>("Table")
            .register_fn("num_rows", |t: &mut Table| -> i64 {
                t.0.iter().map(|b| b.num_rows() as i64).sum()
            })
            .register_fn("columns", |t: &mut Table| -> rhai::Array {
                match t.0.first() {
                    Some(b) => b
                        .schema()
                        .fields()
                        .iter()
                        .map(|f| Dynamic::from(f.name().clone()))
                        .collect(),
                    None => rhai::Array::new(),
                }
            })
            .register_fn("col", |t: &mut Table, name: &str| -> ScriptResult<Col> {
                let Some(first) = t.0.first() else {
                    return fail(format!("no rows carry a column `{name}`"));
                };
                let Some((index, _)) = first.schema().column_with_name(name) else {
                    return fail(format!("no column `{name}` in the result"));
                };
                if t.0.len() == 1 {
                    return Ok(Col(Arc::clone(first.column(index))));
                }
                let arrays: Vec<&dyn Array> =
                    t.0.iter().map(|b| b.column(index).as_ref()).collect();
                datafusion::arrow::compute::concat(&arrays)
                    .map(Col)
                    .map_err(|e| e.to_string().into())
            });

        engine
            .register_type_with_name::<Col>("Col")
            .register_fn("count", |c: &mut Col| -> i64 { c.0.len() as i64 })
            .register_fn("null_count", |c: &mut Col| -> i64 {
                c.0.null_count() as i64
            })
            .register_fn("distinct", |c: &mut Col| -> ScriptResult<i64> {
                let mut seen = std::collections::HashSet::new();
                for i in 0..c.0.len() {
                    if c.0.is_null(i) {
                        continue;
                    }
                    seen.insert(
                        array_value_to_string(&c.0, i).map_err(|e| e.to_string())?,
                    );
                }
                Ok(seen.len() as i64)
            })
            .register_fn("min", |c: &mut Col| -> ScriptResult<Dynamic> { extremum(c, true) })
            .register_fn("max", |c: &mut Col| -> ScriptResult<Dynamic> { extremum(c, false) })
            .register_fn("sum", |c: &mut Col| -> ScriptResult<Dynamic> {
                let floats = as_floats(&c.0)?;
                Ok(aggregate::sum(&floats).map(Dynamic::from).unwrap_or(Dynamic::UNIT))
            })
            .register_fn(
                "match_rate",
                |c: &mut Col, pattern: &str| -> ScriptResult<f64> {
                    let re = regex::Regex::new(pattern).map_err(|e| e.to_string())?;
                    let Some(values) = c.0.as_any().downcast_ref::<StringArray>() else {
                        return fail("match_rate reads a string column");
                    };
                    let mut total = 0u64;
                    let mut matched = 0u64;
                    for i in 0..values.len() {
                        if values.is_null(i) {
                            continue;
                        }
                        total += 1;
                        if re.is_match(values.value(i)) {
                            matched += 1;
                        }
                    }
                    Ok(if total == 0 { 0.0 } else { matched as f64 / total as f64 })
                },
            )
            .register_fn(
                "parse_rate",
                |c: &mut Col, target: &str| -> ScriptResult<f64> {
                    let to = sql_type(target).ok_or_else(|| {
                        format!("parse_rate does not know the type `{target}`")
                    })?;
                    let non_null = (c.0.len() - c.0.null_count()) as f64;
                    if non_null == 0.0 {
                        return Ok(1.0);
                    }
                    let cast = cast_with_options(
                        &c.0,
                        &to,
                        &CastOptions { safe: true, ..Default::default() },
                    )
                    .map_err(|e| e.to_string())?;
                    let parsed = (cast.len() - cast.null_count()) as f64;
                    Ok(parsed / non_null)
                },
            )
            .register_fn("value_at", |c: &mut Col, i: i64| -> ScriptResult<Dynamic> {
                let i = i as usize;
                if i >= c.0.len() || c.0.is_null(i) {
                    return Ok(Dynamic::UNIT);
                }
                Ok(Dynamic::from(
                    array_value_to_string(&c.0, i).map_err(|e| e.to_string())?,
                ))
            });

        engine
            .register_type_with_name::<Door>("Door")
            .register_fn("query", |d: &mut Door, sql: &str| -> ScriptResult<Table> {
                d.0.sql(sql).map(|b| Table(Arc::new(b))).map_err(Into::into)
            });

        RhaiRuntime {
            root: root.into(),
            engine,
            asts: RwLock::new(HashMap::new()),
        }
    }

    fn ast(&self, script: &str) -> Result<Arc<AST>, String> {
        let relative = PathBuf::from(script);
        if relative.is_absolute()
            || relative
                .components()
                .any(|c| matches!(c, Component::ParentDir))
        {
            return Err(format!(
                "script `{script}` must stay under the functions root — relative, no `..`"
            ));
        }
        let path = self.root.join(relative);
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("script `{script}`: {e}"))?;
        if let Some((cached_text, ast)) = self.asts.read().expect("asts").get(script)
            && *cached_text == text
        {
            return Ok(Arc::clone(ast));
        }
        let ast = Arc::new(
            self.engine
                .compile(&text)
                .map_err(|e| format!("script `{script}`: {e}"))?,
        );
        self.asts
            .write()
            .expect("asts")
            .insert(script.to_string(), (text, Arc::clone(&ast)));
        Ok(ast)
    }
}

impl FunctionRuntime for RhaiRuntime {
    fn invoke(
        &self,
        function: &FunctionRow,
        subject: &str,
        context: &Value,
        door: Arc<dyn SqlDoor>,
    ) -> Result<Value, String> {
        let ast = self.ast(&function.script)?;
        let mut scope = Scope::new();
        scope.push_constant("subject", subject.to_string());
        scope.push_constant(
            "context",
            rhai::serde::to_dynamic(context).map_err(|e| e.to_string())?,
        );
        scope.push_constant("db", Door(door));
        let result: Dynamic = self
            .engine
            .eval_ast_with_scope(&mut scope, &ast)
            .map_err(|e| format!("`{}`: {e}", function.name))?;
        serde_json::to_value(&result)
            .map_err(|e| format!("`{}` returned something JSON cannot carry: {e}", function.name))
    }
}

fn extremum(c: &mut Col, min: bool) -> ScriptResult<Dynamic> {
    if let Some(values) = c.0.as_any().downcast_ref::<StringArray>() {
        let v = if min {
            aggregate::min_string(values)
        } else {
            aggregate::max_string(values)
        };
        return Ok(v.map(|s| Dynamic::from(s.to_string())).unwrap_or(Dynamic::UNIT));
    }
    let floats = as_floats(&c.0)?;
    let v = if min {
        aggregate::min(&floats)
    } else {
        aggregate::max(&floats)
    };
    Ok(v.map(Dynamic::from).unwrap_or(Dynamic::UNIT))
}

fn as_floats(array: &ArrayRef) -> ScriptResult<Float64Array> {
    let cast = cast_with_options(
        array,
        &DataType::Float64,
        &CastOptions { safe: true, ..Default::default() },
    )
    .map_err(|e| e.to_string())?;
    Ok(cast
        .as_any()
        .downcast_ref::<Float64Array>()
        .expect("cast to Float64 yields Float64")
        .clone())
}

/// The SQL spellings a decision's `value` uses, mapped to arrow types for
/// trial casts. Format-parsed types trial through SQL (`try_to_date`), not
/// here.
fn sql_type(spelling: &str) -> Option<DataType> {
    let upper = spelling.trim().to_uppercase();
    if let Some(rest) = upper.strip_prefix("DECIMAL") {
        let inner = rest.trim().trim_start_matches('(').trim_end_matches(')');
        let mut parts = inner.split(',').map(str::trim);
        let p: u8 = parts.next()?.parse().ok()?;
        let s: i8 = parts.next().unwrap_or("0").parse().ok()?;
        return Some(DataType::Decimal128(p, s));
    }
    Some(match upper.as_str() {
        "BIGINT" | "INT8" => DataType::Int64,
        "INTEGER" | "INT" | "INT4" => DataType::Int32,
        "DOUBLE" | "DOUBLE PRECISION" | "FLOAT8" => DataType::Float64,
        "REAL" | "FLOAT4" => DataType::Float32,
        "BOOLEAN" | "BOOL" => DataType::Boolean,
        "DATE" => DataType::Date32,
        "TIMESTAMP" => DataType::Timestamp(TimeUnit::Microsecond, None),
        "VARCHAR" | "TEXT" => DataType::Utf8,
        _ => return None,
    })
}
