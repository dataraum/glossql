//! The derived pair (project-lead rulings, 2026-08-04): the bare table name
//! is a view — identity while nothing is decided, the typed projection as
//! decisions land — and `<t>_quarantined` is the complement in raw shape.
//! Decisions are the collapsed `type` aspect per column: the inference
//! function's pick by default, agent and human glosses superseding it.
//! Every projection is wrapped in `TRY_CAST`, so a bad decision produces
//! NULLs and quarantine rows, never a broken view — v0.3's `TRY_` rewrite
//! rule in gloss form.

use std::any::Any;
use std::sync::Arc;

use datafusion::arrow::array::{Array, ArrayRef, Date32Array, StringArray, TimestampMicrosecondArray};
use datafusion::arrow::datatypes::{DataType, TimeUnit};
use datafusion::common::{Result as DFResult, exec_err};
use datafusion::logical_expr::{
    ColumnarValue, ScalarFunctionArgs, ScalarUDF, ScalarUDFImpl, Signature, Volatility,
};
use datafusion::prelude::SessionContext;
use glossql_glossary::{ReadContext, Scope, Store, TYPE_ASPECT};
use serde_json::Value;

use crate::session::SessionError;

/// One column's current typing decision, whoever made it.
#[derive(Debug, Clone)]
pub(crate) struct Decision {
    pub column: String,
    pub target: String,
    pub expr: Option<String>,
}

/// The current decisions for a table: collapsed `type` values per column.
/// A contested decision (value withheld) leaves the column raw.
pub(crate) async fn decisions(
    store: &Store,
    dataset: &str,
    table: &str,
) -> Result<Vec<Decision>, SessionError> {
    let rows = store
        .collapsed_read(
            dataset,
            &Scope::Subject(table.to_string()),
            Some(TYPE_ASPECT),
            &ReadContext::default(),
        )
        .await?;
    let mut decisions = Vec::new();
    for row in rows {
        let Some(column) = row
            .subject
            .strip_prefix(&format!("{table}."))
            .map(String::from)
        else {
            continue;
        };
        let Some(body) = row.value else { continue };
        let Ok(body) = serde_json::from_str::<Value>(&body) else {
            continue;
        };
        let Some(target) = body.pointer("/value").and_then(Value::as_str) else {
            continue;
        };
        decisions.push(Decision {
            column,
            target: target.to_string(),
            expr: body
                .pointer("/expr")
                .and_then(Value::as_str)
                .map(String::from),
        });
    }
    Ok(decisions)
}

/// The emitted pair, as SQL text — the inspectable record. v0.3 semantics
/// (grounded 2026-08-04): typed keeps every row, a failed cast is a NULL
/// cell; quarantine is the complement copy in raw shape, any column failing
/// on a non-NULL value.
pub(crate) fn pair_sql(
    logical: &str,
    raw: &str,
    columns: &[String],
    decisions: &[Decision],
) -> (String, String) {
    let of = |c: &str| decisions.iter().find(|d| d.column == c);
    let cast = |d: &Decision| {
        let inner = d
            .expr
            .clone()
            .unwrap_or_else(|| format!("\"{}\"", d.column));
        format!("TRY_CAST({inner} AS {})", d.target)
    };
    let projection: Vec<String> = columns
        .iter()
        .map(|c| match of(c) {
            Some(d) => format!("{} AS \"{c}\"", cast(d)),
            None => format!("\"{c}\""),
        })
        .collect();
    let typed = format!(
        "CREATE OR REPLACE VIEW \"{logical}\" AS SELECT {} FROM \"{raw}\"",
        projection.join(", ")
    );
    let checks: Vec<String> = columns
        .iter()
        .filter_map(|c| {
            of(c).map(|d| format!("(\"{c}\" IS NOT NULL AND {} IS NULL)", cast(d)))
        })
        .collect();
    let quarantine = format!(
        "CREATE OR REPLACE VIEW \"{logical}_quarantined\" AS SELECT * FROM \"{raw}\" WHERE {}",
        if checks.is_empty() {
            "FALSE".to_string()
        } else {
            checks.join(" OR ")
        }
    );
    (typed, quarantine)
}

/// `try_to_date` / `try_to_timestamp`: format parsing that yields NULL on
/// failure. DataFusion 53's own `to_date`/`to_timestamp` abort the whole
/// scan on one dirty value and ship no `try_` variants (verified in source,
/// 2026-08-04), so the session registers these — usable in recipes, views,
/// scripts, and user SQL alike.
pub(crate) fn register_try_functions(ctx: &SessionContext) {
    ctx.register_udf(ScalarUDF::from(TryParse::date()));
    ctx.register_udf(ScalarUDF::from(TryParse::timestamp()));
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct TryParse {
    name: &'static str,
    returns: DataType,
    signature: Signature,
}

impl TryParse {
    fn date() -> Self {
        TryParse {
            name: "try_to_date",
            returns: DataType::Date32,
            signature: Signature::exact(
                vec![DataType::Utf8, DataType::Utf8],
                Volatility::Immutable,
            ),
        }
    }

    fn timestamp() -> Self {
        TryParse {
            name: "try_to_timestamp",
            returns: DataType::Timestamp(TimeUnit::Microsecond, None),
            signature: Signature::exact(
                vec![DataType::Utf8, DataType::Utf8],
                Volatility::Immutable,
            ),
        }
    }
}

impl ScalarUDFImpl for TryParse {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn name(&self) -> &str {
        self.name
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, _args: &[DataType]) -> DFResult<DataType> {
        Ok(self.returns.clone())
    }

    fn invoke_with_args(&self, args: ScalarFunctionArgs) -> DFResult<ColumnarValue> {
        let arrays = ColumnarValue::values_to_arrays(&args.args)?;
        let Some(values) = arrays[0].as_any().downcast_ref::<StringArray>() else {
            return exec_err!("{} expects a string column", self.name);
        };
        let Some(formats) = arrays[1].as_any().downcast_ref::<StringArray>() else {
            return exec_err!("{} expects a string format", self.name);
        };
        let format_at = |i: usize| {
            if formats.is_null(i) { None } else { Some(formats.value(i)) }
        };
        let out: ArrayRef = match self.returns {
            DataType::Date32 => {
                let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).expect("epoch");
                Arc::new(Date32Array::from_iter((0..values.len()).map(|i| {
                    let f = format_at(i)?;
                    if values.is_null(i) {
                        return None;
                    }
                    chrono::NaiveDate::parse_from_str(values.value(i), f)
                        .ok()
                        .map(|d| (d - epoch).num_days() as i32)
                })))
            }
            _ => Arc::new(TimestampMicrosecondArray::from_iter((0..values.len()).map(
                |i| {
                    let f = format_at(i)?;
                    if values.is_null(i) {
                        return None;
                    }
                    chrono::NaiveDateTime::parse_from_str(values.value(i), f)
                        .ok()
                        .map(|t| t.and_utc().timestamp_micros())
                },
            ))),
        };
        Ok(ColumnarValue::Array(out))
    }
}
