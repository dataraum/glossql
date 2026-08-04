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
            })
            // The first row's value in a named column — the one-row
            // aggregate read every script does; () for NULL or no rows.
            .register_fn("cell", |t: &mut Table, name: &str| -> ScriptResult<Dynamic> {
                let Some(first) = t.0.first() else {
                    return fail(format!("no rows carry a column `{name}`"));
                };
                let Some((index, _)) = first.schema().column_with_name(name) else {
                    return fail(format!("no column `{name}` in the result"));
                };
                let Some(batch) = t.0.iter().find(|b| b.num_rows() > 0) else {
                    return Ok(Dynamic::UNIT);
                };
                let column = batch.column(index);
                if column.is_null(0) {
                    return Ok(Dynamic::UNIT);
                }
                Ok(Dynamic::from(
                    array_value_to_string(column, 0).map_err(|e| e.to_string())?,
                ))
            });

        engine
            .register_type_with_name::<Col>("Col")
            // The column's Arrow type name ("Float64", "Date32", …) — the
            // door's empty results still carry schema, so a LIMIT 0 query
            // types a column without scanning it.
            .register_fn("dtype", |c: &mut Col| -> String {
                c.0.data_type().to_string()
            })
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
                if !numeric_like(c.0.data_type()) {
                    return Ok(Dynamic::UNIT);
                }
                let floats = as_floats(&c.0)?;
                Ok(aggregate::sum(&floats).map(Dynamic::from).unwrap_or(Dynamic::UNIT))
            })
            .register_fn("mean", |c: &mut Col| -> ScriptResult<Dynamic> {
                let v = valid_floats(&c.0)?;
                if v.is_empty() {
                    return Ok(Dynamic::UNIT);
                }
                Ok(Dynamic::from(v.iter().sum::<f64>() / v.len() as f64))
            })
            .register_fn("stddev", |c: &mut Col| -> ScriptResult<Dynamic> {
                // Sample standard deviation, matching SQL STDDEV.
                let v = valid_floats(&c.0)?;
                if v.len() < 2 {
                    return Ok(Dynamic::UNIT);
                }
                let mean = v.iter().sum::<f64>() / v.len() as f64;
                let var =
                    v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (v.len() as f64 - 1.0);
                Ok(Dynamic::from(var.sqrt()))
            })
            .register_fn("percentile", |c: &mut Col, p: f64| -> ScriptResult<Dynamic> {
                // Linear interpolation, matching SQL PERCENTILE_CONT.
                if !(0.0..=1.0).contains(&p) {
                    return fail("percentile wants p in [0, 1]");
                }
                let mut v = valid_floats(&c.0)?;
                if v.is_empty() {
                    return Ok(Dynamic::UNIT);
                }
                v.sort_by(f64::total_cmp);
                Ok(Dynamic::from(interpolate(&v, p)))
            })
            .register_fn("mad", |c: &mut Col| -> ScriptResult<Dynamic> {
                // Median absolute deviation — the robust spread the modified
                // Z-score fences ride on.
                let mut v = valid_floats(&c.0)?;
                if v.is_empty() {
                    return Ok(Dynamic::UNIT);
                }
                v.sort_by(f64::total_cmp);
                let median = interpolate(&v, 0.5);
                let mut deviations: Vec<f64> = v.iter().map(|x| (x - median).abs()).collect();
                deviations.sort_by(f64::total_cmp);
                Ok(Dynamic::from(interpolate(&deviations, 0.5)))
            })
            .register_fn("top_k", |c: &mut Col, k: i64| -> ScriptResult<rhai::Array> {
                let mut counts: HashMap<String, i64> = HashMap::new();
                for i in 0..c.0.len() {
                    if c.0.is_null(i) {
                        continue;
                    }
                    let value =
                        array_value_to_string(&c.0, i).map_err(|e| e.to_string())?;
                    *counts.entry(value).or_insert(0) += 1;
                }
                let mut pairs: Vec<(String, i64)> = counts.into_iter().collect();
                pairs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                pairs.truncate(k.max(0) as usize);
                Ok(pairs
                    .into_iter()
                    .map(|(value, count)| {
                        let mut row = rhai::Map::new();
                        row.insert("value".into(), Dynamic::from(value));
                        row.insert("count".into(), Dynamic::from(count));
                        Dynamic::from_map(row)
                    })
                    .collect())
            })
            .register_fn("len_stats", |c: &mut Col| -> ScriptResult<Dynamic> {
                let Some(values) = c.0.as_any().downcast_ref::<StringArray>() else {
                    return Ok(Dynamic::UNIT);
                };
                let (mut min, mut max, mut total, mut n) = (i64::MAX, 0i64, 0i64, 0i64);
                for i in 0..values.len() {
                    if values.is_null(i) {
                        continue;
                    }
                    let len = values.value(i).chars().count() as i64;
                    min = min.min(len);
                    max = max.max(len);
                    total += len;
                    n += 1;
                }
                if n == 0 {
                    return Ok(Dynamic::UNIT);
                }
                let mut stats = rhai::Map::new();
                stats.insert("min".into(), Dynamic::from(min));
                stats.insert("max".into(), Dynamic::from(max));
                stats.insert("avg".into(), Dynamic::from(total as f64 / n as f64));
                Ok(Dynamic::from_map(stats))
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
                        format!("`{target}` is not a cast target the substrate accepts")
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

/// What the float kernels may read as numbers: numeric types themselves,
/// booleans, and strings (the safe-cast reading on a raw column). Temporal
/// columns are deliberately out — a date has an order but no mean, exactly
/// v0.3's gate (numeric stats behind `is_numeric(resolved_type)`).
fn numeric_like(dt: &DataType) -> bool {
    dt.is_numeric()
        || matches!(
            dt,
            DataType::Boolean | DataType::Utf8 | DataType::LargeUtf8 | DataType::Utf8View | DataType::Null
        )
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
    if numeric_like(c.0.data_type()) {
        let floats = as_floats(&c.0)?;
        let v = if min {
            aggregate::min(&floats)
        } else {
            aggregate::max(&floats)
        };
        return Ok(v.map(Dynamic::from).unwrap_or(Dynamic::UNIT));
    }
    // Dates, timestamps, and the rest order by their display form — ISO
    // spellings sort chronologically, so min/max stay truthful.
    let mut best: Option<String> = None;
    for i in 0..c.0.len() {
        if c.0.is_null(i) {
            continue;
        }
        let value = array_value_to_string(&c.0, i).map_err(|e| e.to_string())?;
        best = Some(match best {
            None => value,
            Some(b) => {
                if (value < b) == min {
                    value
                } else {
                    b
                }
            }
        });
    }
    Ok(best.map(Dynamic::from).unwrap_or(Dynamic::UNIT))
}

/// The parseable values as floats — safe-cast semantics, so on a raw
/// VARCHAR column this is "every value that reads as a number". Empty for
/// column types with no numeric reading (dates, timestamps): the kernels
/// answer UNIT there instead of arithmetic on an epoch encoding.
fn valid_floats(array: &ArrayRef) -> ScriptResult<Vec<f64>> {
    if !numeric_like(array.data_type()) {
        return Ok(Vec::new());
    }
    let floats = as_floats(array)?;
    Ok((0..floats.len())
        .filter(|&i| !floats.is_null(i))
        .map(|i| floats.value(i))
        .collect())
}

/// PERCENTILE_CONT over an already-sorted slice.
fn interpolate(sorted: &[f64], p: f64) -> f64 {
    let rank = p * (sorted.len() - 1) as f64;
    let low = rank.floor() as usize;
    let high = rank.ceil() as usize;
    sorted[low] + (sorted[high] - sorted[low]) * (rank - low as f64)
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

/// The SQL cast-target spellings the substrate accepts, mapped to arrow for
/// trial casts — one arm per family of datafusion-sql 53.1's planner
/// (planner.rs:662-763, decimal defaults utils.rs:289-317), so a trial and
/// the served `TRY_CAST` agree on what exists. One dialect, one list:
/// spellings DataFusion rejects (`TIMESTAMP_NS`, `DATETIME`, `DEC`,
/// `HUGEINT`) are refused here too, loudly, instead of trialing a type the
/// view could never serve. Format-parsed types trial through SQL
/// (`try_to_date`), not here.
fn sql_type(spelling: &str) -> Option<DataType> {
    let upper = spelling.trim().to_uppercase();
    // `HEAD(params)TAIL` — family words around the parameters, so
    // `DECIMAL(18,2)`, `TIMESTAMP(6)`, and `TIMESTAMP(3) WITH TIME ZONE`
    // all resolve.
    let (head, params, tail) = match upper.split_once('(') {
        Some((head, rest)) => {
            let (inside, tail) = rest.split_once(')')?;
            let params: Vec<u64> = inside
                .split(',')
                .map(|p| p.trim().parse().ok())
                .collect::<Option<_>>()?;
            (head, params, tail)
        }
        None => (upper.as_str(), Vec::new(), ""),
    };
    let family = format!("{head} {tail}");
    let family = family.split_whitespace().collect::<Vec<_>>().join(" ");
    Some(match family.as_str() {
        "BOOLEAN" | "BOOL" => DataType::Boolean,
        "TINYINT" => DataType::Int8,
        "SMALLINT" | "INT2" => DataType::Int16,
        "INT" | "INTEGER" | "INT4" => DataType::Int32,
        "BIGINT" | "INT8" => DataType::Int64,
        "TINYINT UNSIGNED" => DataType::UInt8,
        "SMALLINT UNSIGNED" | "INT2 UNSIGNED" => DataType::UInt16,
        "INT UNSIGNED" | "INTEGER UNSIGNED" | "INT4 UNSIGNED" => DataType::UInt32,
        "BIGINT UNSIGNED" | "INT8 UNSIGNED" => DataType::UInt64,
        "FLOAT" | "REAL" | "FLOAT4" => DataType::Float32,
        "DOUBLE" | "DOUBLE PRECISION" | "FLOAT8" => DataType::Float64,
        "CHAR" | "VARCHAR" | "TEXT" | "STRING" => DataType::Utf8,
        "DATE" => DataType::Date32,
        "TIME" | "TIME WITHOUT TIME ZONE" => DataType::Time64(TimeUnit::Nanosecond),
        "TIMESTAMP" | "TIMESTAMP WITHOUT TIME ZONE" | "TIMESTAMP WITH TIME ZONE" => {
            // The session's zone rides the real cast; parseability, which is
            // all a trial measures, does not depend on it.
            DataType::Timestamp(
                match params.first() {
                    Some(0) => TimeUnit::Second,
                    Some(3) => TimeUnit::Millisecond,
                    Some(6) => TimeUnit::Microsecond,
                    None | Some(9) => TimeUnit::Nanosecond,
                    _ => return None,
                },
                None,
            )
        }
        "DECIMAL" | "NUMERIC" => {
            let (precision, scale) = match (params.first(), params.get(1)) {
                (Some(&p), Some(&s)) => (p, s),
                (Some(&p), None) => (p, 0),
                (None, _) => (38, 10),
            };
            if precision == 0 || precision > 76 || scale > precision {
                return None;
            }
            if precision > 38 {
                DataType::Decimal256(precision as u8, scale as i8)
            } else {
                DataType::Decimal128(precision as u8, scale as i8)
            }
        }
        "BYTEA" => DataType::Binary,
        "INTERVAL" => DataType::Interval(datafusion::arrow::datatypes::IntervalUnit::MonthDayNano),
        _ => return None,
    })
}
