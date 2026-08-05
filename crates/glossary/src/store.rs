//! The sqlx-backed store. One SQLite file per workspace (`:memory:` in
//! tests); Postgres later is a connection string, so every query here stays
//! in portable SQL. Admission (SPEC.md §5.2, §7.1) happens on the write
//! paths; supersession is the `NOT EXISTS` read predicate, never an update.

use serde_json::Value;
use sqlx::Row as _;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

use glossql_parser::{
    AspectDecl, AspectKind, DatasetDecl, FunctionDecl, FunctionScope, JsonBody, RecipeDecl,
    SourceDecl, Speaker, WitnessDecl,
};

use crate::schemas::grounding_schema;
use crate::types::{
    Actor, ActorKind, AttestRow, CacheRow, CollapsedRow, Error, FunctionRow, RawRow,
    RecipeAdmission, RecipeRow, Result, WitnessRow,
};

/// What a read sweeps over (SPEC.md §5.3, §7.2): the whole dataset, or a
/// subject and everything under it (columns of a table, relationships rooted
/// at it).
#[derive(Debug, Clone)]
pub enum Scope {
    Dataset,
    Subject(String),
}

/// What the session knows and the store cannot: the subjects that exist
/// (tables and columns from the data plane — the disclosure grid enumerates
/// them so absence shows as a row, never as omission) and each table's
/// current snapshot (the staleness comparison). Empty context still collapses
/// correctly; it just cannot show `unassessed` subjects nobody wrote about
/// or mark snapshot staleness.
#[derive(Debug, Clone, Default)]
pub struct ReadContext {
    pub universe: Vec<String>,
    pub snapshots: std::collections::HashMap<String, i64>,
}

/// One current slot under (subject, aspect): a gloss (human or agent) or a
/// witness-bound function's cached output. The collapse and the raw read
/// both build from these.
#[derive(Debug, Clone)]
struct Slot {
    subject: String,
    aspect: String,
    /// 0 = human, 1 = agent, 2 = function — the precedence order.
    rank: u8,
    actor: String,
    witness: Option<String>,
    body: String,
    written_at: String,
    snapshot_id: Option<i64>,
}

impl Scope {
    /// Predicate over a `subject` column: exact, descendant (`s.…`), or a
    /// pair path the subject participates in — from either side (`s -> …`,
    /// `s <-> …`, `… -> s`, `… -> s.…`). The far endpoint's own context is
    /// never pulled in.
    fn predicate(&self, column: &str) -> (String, Vec<String>) {
        match self {
            Scope::Dataset => ("1 = 1".into(), vec![]),
            Scope::Subject(s) => (
                format!(
                    "({column} = ? OR {column} LIKE ? OR {column} LIKE ? \
                      OR {column} LIKE ? OR {column} LIKE ?)"
                ),
                vec![
                    s.clone(),
                    format!("{s}.%"),
                    format!("{s} %"),
                    format!("%> {s}"),
                    format!("%> {s}.%"),
                ],
            ),
        }
    }
}

const MIGRATION: &str = r#"
CREATE TABLE IF NOT EXISTS sources (
  name TEXT PRIMARY KEY,
  settings TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS datasets (
  name TEXT PRIMARY KEY,
  settings TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS recipes (
  dataset TEXT NOT NULL,
  table_name TEXT NOT NULL,
  source TEXT NOT NULL,
  sql TEXT NOT NULL,
  PRIMARY KEY (dataset, table_name)
);
CREATE TABLE IF NOT EXISTS relationships (
  dataset TEXT NOT NULL,
  left_path TEXT NOT NULL,
  op TEXT NOT NULL,
  right_path TEXT NOT NULL,
  PRIMARY KEY (dataset, left_path, op, right_path)
);
CREATE TABLE IF NOT EXISTS aspects (
  name TEXT PRIMARY KEY,
  schema TEXT NOT NULL,
  kind TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS functions (
  name TEXT PRIMARY KEY,
  scope_dataset TEXT,
  script TEXT NOT NULL,
  accepts TEXT,
  returns TEXT
);
CREATE TABLE IF NOT EXISTS witnesses (
  name TEXT PRIMARY KEY,
  aspect TEXT NOT NULL,
  speakers TEXT NOT NULL,
  detector TEXT,
  threshold REAL
);
CREATE TABLE IF NOT EXISTS glossary (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  dataset TEXT NOT NULL,
  subject TEXT NOT NULL,
  aspect TEXT NOT NULL,
  actor_kind TEXT NOT NULL,
  actor_id TEXT NOT NULL,
  body TEXT NOT NULL,
  written_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  snapshot_id INTEGER
);
CREATE TABLE IF NOT EXISTS cache (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  dataset TEXT NOT NULL,
  subject TEXT NOT NULL,
  function TEXT NOT NULL,
  body TEXT NOT NULL,
  computed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  snapshot_id INTEGER
);
CREATE TABLE IF NOT EXISTS imports (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  dataset TEXT NOT NULL,
  table_name TEXT NOT NULL,
  source_rows INTEGER NOT NULL,
  landed_rows INTEGER NOT NULL,
  imported_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
"#;

/// A store relation readable as a plain table through the doors. The
/// one place its shape lives: `columns` names exactly what `sql`
/// selects, in order — every other list (the session's read planner,
/// the doors' cap policy) derives from here.
pub struct Relation {
    pub name: &'static str,
    pub columns: &'static [&'static str],
    sql: &'static str,
}

/// Every relation [`Store::relation_rows`] serves — the glossary, the
/// evidence, and the declarations, so an agent lists what exists
/// instead of being told.
pub const RELATIONS: &[Relation] = &[
    Relation {
        name: "glossary",
        columns: &[
            "dataset",
            "subject",
            "aspect",
            "actor_kind",
            "actor_id",
            "body",
            "written_at",
            "snapshot_id",
        ],
        sql: "SELECT dataset, subject, aspect, actor_kind, actor_id, body, written_at, \
                     CAST(snapshot_id AS TEXT) AS snapshot_id \
              FROM glossary ORDER BY id",
    },
    Relation {
        name: "cache",
        columns: &["dataset", "subject", "function", "body", "computed_at", "snapshot_id"],
        sql: "SELECT dataset, subject, function, body, computed_at, \
                     CAST(snapshot_id AS TEXT) AS snapshot_id \
              FROM cache ORDER BY id",
    },
    Relation {
        name: "imports",
        columns: &[
            "dataset",
            "table_name",
            "source_rows",
            "landed_rows",
            "dropped_rows_count",
            "imported_at",
        ],
        sql: "SELECT dataset, table_name, CAST(source_rows AS TEXT) AS source_rows, \
                     CAST(landed_rows AS TEXT) AS landed_rows, \
                     CAST(source_rows - landed_rows AS TEXT) AS dropped_rows_count, \
                     imported_at \
              FROM imports ORDER BY id",
    },
    Relation {
        name: "functions",
        columns: &["name", "scope", "script", "accepts", "returns"],
        sql: "SELECT name, COALESCE(scope_dataset, 'GLOBAL') AS scope, script, \
                     accepts, returns \
              FROM functions ORDER BY name",
    },
    Relation {
        name: "aspects",
        columns: &["name", "kind", "schema"],
        sql: "SELECT name, kind, schema FROM aspects ORDER BY name",
    },
    Relation {
        name: "witnesses",
        columns: &["name", "aspect", "speakers", "detector", "threshold"],
        sql: "SELECT name, aspect, speakers, detector, \
                     CAST(threshold AS TEXT) AS threshold \
              FROM witnesses ORDER BY name",
    },
    Relation {
        name: "sources",
        columns: &["name", "settings"],
        sql: "SELECT name, settings FROM sources ORDER BY name",
    },
    Relation {
        name: "relationships",
        columns: &["dataset", "left_path", "op", "right_path"],
        sql: "SELECT dataset, left_path, op, right_path FROM relationships \
              ORDER BY dataset, left_path, op, right_path",
    },
];

/// The column shape of a readable store relation, `None` for any other
/// name — the lookup the session's planner and the doors share.
pub fn relation_columns(name: &str) -> Option<&'static [&'static str]> {
    RELATIONS
        .iter()
        .find(|r| r.name == name)
        .map(|r| r.columns)
}

#[derive(Debug, Clone)]
pub struct Store {
    pool: SqlitePool,
}

impl Store {
    pub async fn open(url: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new().connect(url).await?;
        sqlx::raw_sql(MIGRATION).execute(&pool).await?;
        Ok(Store { pool })
    }

    /// In-memory store. One connection, or every pool checkout would see a
    /// different empty database.
    pub async fn open_memory() -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;
        sqlx::raw_sql(MIGRATION).execute(&pool).await?;
        Ok(Store { pool })
    }

    // -- declarations ----------------------------------------------------

    pub async fn declare_source(&self, decl: &SourceDecl) -> Result<()> {
        sqlx::query("INSERT OR REPLACE INTO sources (name, settings) VALUES (?, ?)")
            .bind(decl.name.value.as_str())
            .bind(settings_json(&decl.settings))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn declare_dataset(&self, decl: &DatasetDecl) -> Result<()> {
        sqlx::query("INSERT OR REPLACE INTO datasets (name, settings) VALUES (?, ?)")
            .bind(decl.name.value.as_str())
            .bind(settings_json(&decl.settings))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Statement identity is content (SPEC.md §3): an unchanged recipe is a
    /// no-op; a changed one is refused outright (project lead, 2026-08-04 —
    /// replacement is postponed until the deletion cascade exists). A
    /// different SQL is a different table: declare it under another name,
    /// or `DROP TABLE` first while the table is still empty and unglossed.
    pub async fn declare_recipe(&self, decl: &RecipeDecl) -> Result<RecipeAdmission> {
        let dataset = decl.dataset.value.as_str();
        let table = decl.table.value.as_str();
        self.require("dataset", "datasets", dataset).await?;
        self.require("source", "sources", decl.source.value.as_str())
            .await?;
        let existing = self.recipe(dataset, table).await?;
        let admission = match &existing {
            None => RecipeAdmission::Created,
            Some(prior) if prior.source == decl.source.value && prior.sql == decl.sql => {
                return Ok(RecipeAdmission::Unchanged);
            }
            Some(_) => {
                return Err(Error::RecipeChanged {
                    table: table.into(),
                });
            }
        };
        sqlx::query(
            "INSERT OR REPLACE INTO recipes (dataset, table_name, source, sql) VALUES (?, ?, ?, ?)",
        )
        .bind(dataset)
        .bind(table)
        .bind(decl.source.value.as_str())
        .bind(decl.sql.as_str())
        .execute(&self.pool)
        .await?;
        Ok(admission)
    }

    pub async fn recipe(&self, dataset: &str, table: &str) -> Result<Option<RecipeRow>> {
        let row = sqlx::query(
            "SELECT source, sql FROM recipes WHERE dataset = ? AND table_name = ?",
        )
        .bind(dataset)
        .bind(table)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| RecipeRow {
            source: r.get("source"),
            sql: r.get("sql"),
        }))
    }

    pub async fn source_settings(&self, name: &str) -> Result<Option<Value>> {
        let row = sqlx::query("SELECT settings FROM sources WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;
        row.map(|r| {
            serde_json::from_str(&r.get::<String, _>("settings"))
                .map_err(|e| Error::Corrupt(e.to_string()))
        })
        .transpose()
    }

    /// Endpoints arrive canonical (dataset-relative `table.column`); the
    /// session resolves prefixes first.
    pub async fn declare_relationship(
        &self,
        dataset: &str,
        left: &str,
        op: &str,
        right: &str,
    ) -> Result<()> {
        sqlx::query(
            "INSERT OR REPLACE INTO relationships (dataset, left_path, op, right_path) \
             VALUES (?, ?, ?, ?)",
        )
        .bind(dataset)
        .bind(left)
        .bind(op)
        .bind(right)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Content-identical re-declaration is a no-op; changing an aspect while
    /// glosses under it exist is refused — delete them first (SPEC.md §5.1).
    pub async fn declare_aspect(&self, decl: &AspectDecl) -> Result<()> {
        if let Err(e) = jsonschema::validator_for(&decl.schema.value) {
            return Err(Error::BadAspectSchema {
                name: decl.name.value.clone(),
                detail: e.to_string(),
            });
        }
        let name = decl.name.value.as_str();
        if let Some((schema, kind)) = self.aspect(name).await? {
            if schema == decl.schema.value && kind == kind_str(decl.kind) {
                return Ok(());
            }
            let glosses: i64 = sqlx::query("SELECT count(*) AS n FROM glossary WHERE aspect = ?")
                .bind(name)
                .fetch_one(&self.pool)
                .await?
                .get("n");
            if glosses > 0 {
                return Err(Error::AspectInUse {
                    name: name.into(),
                    glosses,
                });
            }
        }
        sqlx::query("INSERT OR REPLACE INTO aspects (name, schema, kind) VALUES (?, ?, ?)")
            .bind(decl.name.value.as_str())
            .bind(decl.schema.raw.as_str())
            .bind(kind_str(decl.kind))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// `ACCEPTS` names declared aspects — the context the server assembles
    /// for the script (SPEC.md §6); each must exist.
    pub async fn declare_function(&self, decl: &FunctionDecl) -> Result<()> {
        for aspect in &decl.accepts {
            self.require("aspect", "aspects", aspect.value.as_str())
                .await?;
        }
        // RETURNS mirrors ACCEPTS (ruled 2026-08-04): it names the aspect
        // the output fills. A MEASUREMENT aspect has one producer; a FACT
        // aspect's returning functions are voices; a QUERY aspect is never
        // function-filled. No RETURNS declares a detector.
        if let Some(aspect) = &decl.returns {
            let aspect = aspect.value.as_str();
            let (_, kind) = self.aspect(aspect).await?.ok_or_else(|| Error::Unknown {
                what: "aspect",
                name: aspect.into(),
            })?;
            match kind.as_str() {
                "query" => {
                    return Err(Error::ReturnsQueryAspect {
                        function: decl.name.value.clone(),
                        aspect: aspect.into(),
                    });
                }
                "measurement" => {
                    let taken: Option<String> =
                        sqlx::query("SELECT name FROM functions WHERE returns = ? AND name != ?")
                            .bind(aspect)
                            .bind(decl.name.value.as_str())
                            .fetch_optional(&self.pool)
                            .await?
                            .map(|r| r.get("name"));
                    if let Some(existing) = taken {
                        return Err(Error::MeasurementProducerTaken {
                            aspect: aspect.into(),
                            existing,
                        });
                    }
                }
                _fact => {}
            }
        }
        let accepts = if decl.accepts.is_empty() {
            None
        } else {
            let names: Vec<Value> = decl
                .accepts
                .iter()
                .map(|a| Value::String(a.value.clone()))
                .collect();
            Some(Value::Array(names).to_string())
        };
        let scope = match &decl.scope {
            FunctionScope::Dataset(d) => Some(d.value.clone()),
            FunctionScope::Global => None,
        };
        sqlx::query(
            "INSERT OR REPLACE INTO functions \
             (name, scope_dataset, script, accepts, returns) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(decl.name.value.as_str())
        .bind(scope)
        .bind(decl.script.as_str())
        .bind(accepts)
        .bind(decl.returns.as_ref().map(|a| a.value.clone()))
        .execute(&self.pool)
        .await?;
        // A re-declared function is a different function; its cached results
        // no longer describe anything.
        sqlx::query("DELETE FROM cache WHERE function = ?")
            .bind(decl.name.value.as_str())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn declare_witness(&self, decl: &WitnessDecl) -> Result<()> {
        let aspect = decl.aspect.value.as_str();
        let kind = self
            .aspect(aspect)
            .await?
            .ok_or_else(|| Error::Unknown {
                what: "aspect",
                name: aspect.into(),
            })?
            .1;

        // BY gates actor glosses only — function voices arrive via RETURNS
        // (ruled 2026-08-04). Measurements are never glossed, so a witness
        // on one carries only a detector; a witness naming neither BY nor
        // DETECTOR declares nothing.
        let speakers: Vec<Value> = decl
            .speakers
            .iter()
            .map(|s| match s {
                Speaker::Agent => Value::String("agent".into()),
                Speaker::Human => Value::String("human".into()),
            })
            .collect();
        if kind == "measurement" && !speakers.is_empty() {
            return Err(Error::MeasurementWitnessSpeakers(aspect.into()));
        }
        if speakers.is_empty() && decl.detector.is_none() {
            return Err(Error::WitnessNamesNothing(decl.name.value.clone()));
        }

        if let Some(detector) = &decl.detector {
            let name = detector.value.clone();
            let f = self
                .function(&name, None)
                .await?
                .ok_or_else(|| Error::Unknown {
                    what: "function",
                    name: name.clone(),
                })?;
            // Role by shape: a detector is a function without RETURNS.
            if f.returns.is_some() {
                return Err(Error::DetectorNotEligible { function: name });
            }
        }
        // THRESHOLD range is admission's job, not the grammar's.
        let threshold = match &decl.threshold {
            None => None,
            Some(t) => {
                let v: f64 = t
                    .parse()
                    .map_err(|_| Error::Corrupt(format!("threshold `{t}` is not a number")))?;
                if !(0.0..=1.0).contains(&v) {
                    return Err(Error::Corrupt(format!("threshold `{t}` is outside 0..1")));
                }
                Some(v)
            }
        };

        sqlx::query(
            "INSERT OR REPLACE INTO witnesses (name, aspect, speakers, detector, threshold) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(decl.name.value.as_str())
        .bind(aspect)
        .bind(Value::Array(speakers).to_string())
        .bind(decl.detector.as_ref().map(|d| d.value.clone()))
        .bind(threshold)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // -- glosses ---------------------------------------------------------

    /// Admission by aspect kind (SPEC.md §5.2), then a plain insert; the
    /// supersession key (subject, aspect, actor kind) is applied by reads.
    /// `snapshot_id` is the subject's table snapshot at write time — `None`
    /// when the subject has no table (dataset-level, pair paths) or no data
    /// plane is attached.
    pub async fn gloss(
        &self,
        dataset: &str,
        actor: &Actor,
        aspect: &str,
        subject: &str,
        body: &JsonBody,
        snapshot_id: Option<i64>,
    ) -> Result<()> {
        let (schema, kind) = self.aspect(aspect).await?.ok_or_else(|| Error::Unknown {
            what: "aspect",
            name: aspect.into(),
        })?;
        match kind.as_str() {
            "measurement" => return Err(Error::MeasurementGloss(aspect.into())),
            "fact" => validate(&schema, &body.value, format!("aspect `{aspect}` WITH"))?,
            _query => validate(
                &grounding_schema(),
                &body.value,
                "standard grounding".into(),
            )?,
        }
        // Where a witness exists, its BY list is the speaker gate (§7.1).
        let witnesses = self.witnesses_on(aspect).await?;
        if !witnesses.is_empty() {
            let admitted = witnesses.iter().any(|w| match actor.kind {
                ActorKind::Agent => w.admits_agent,
                ActorKind::Human => w.admits_human,
            });
            if !admitted {
                return Err(Error::SpeakerNotAdmitted {
                    aspect: aspect.into(),
                    kind: actor.kind,
                });
            }
        }
        sqlx::query(
            "INSERT INTO glossary (dataset, subject, aspect, actor_kind, actor_id, body, snapshot_id) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(dataset)
        .bind(subject)
        .bind(aspect)
        .bind(actor.kind.as_str())
        .bind(actor.id.as_str())
        .bind(body.raw.as_str())
        .bind(snapshot_id)
        .execute(&self.pool)
        .await?;
        self.invalidate(dataset, aspect, subject).await?;
        Ok(())
    }

    /// Writes invalidate, reads recompute, judgment only supersedes (project
    /// lead, 2026-08-04). A new value for an aspect kills the cached output
    /// of every function that `ACCEPTS` it, at and under the subject — the
    /// declared dependency edge, the only definition-level invalidation
    /// there is. Data freshness is snapshot staleness, marked at read.
    async fn invalidate(&self, dataset: &str, aspect: &str, subject: &str) -> Result<()> {
        let dependents = self.functions_accepting(aspect).await?;
        if !dependents.is_empty() {
            let scope = if subject == dataset {
                Scope::Dataset
            } else {
                Scope::Subject(subject.into())
            };
            let (pred, binds) = scope.predicate("subject");
            let marks = vec!["?"; dependents.len()].join(", ");
            let sql = format!(
                "DELETE FROM cache WHERE dataset = ? AND function IN ({marks}) AND {pred}"
            );
            let mut q = sqlx::query(&sql).bind(dataset);
            for f in &dependents {
                q = q.bind(f.as_str());
            }
            for b in &binds {
                q = q.bind(b);
            }
            q.execute(&self.pool).await?;
        }

        Ok(())
    }

    /// One import, recorded (project lead, 2026-08-04): rows the recipe
    /// filtered away are the author's to judge, on the files — the engine
    /// keeps the counts, readable through the `imports` relation.
    pub async fn import_put(
        &self,
        dataset: &str,
        table: &str,
        source_rows: i64,
        landed_rows: i64,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO imports (dataset, table_name, source_rows, landed_rows) \
             VALUES (?, ?, ?, ?)",
        )
        .bind(dataset)
        .bind(table)
        .bind(source_rows)
        .bind(landed_rows)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Glosses at or under a table — what `DROP TABLE` refuses on.
    pub async fn glosses_under(&self, dataset: &str, table: &str) -> Result<i64> {
        let (pred, binds) = Scope::Subject(table.to_string()).predicate("subject");
        let sql = format!("SELECT count(*) AS n FROM glossary WHERE dataset = ? AND {pred}");
        let mut q = sqlx::query(&sql).bind(dataset);
        for b in &binds {
            q = q.bind(b);
        }
        Ok(q.fetch_one(&self.pool).await?.get("n"))
    }

    /// The store side of `DROP TABLE` (PoC: caller has already refused on
    /// data and glosses): the recipe row, the cached evidence under the
    /// table, and the import records go together.
    pub async fn drop_table_records(&self, dataset: &str, table: &str) -> Result<()> {
        sqlx::query("DELETE FROM recipes WHERE dataset = ? AND table_name = ?")
            .bind(dataset)
            .bind(table)
            .execute(&self.pool)
            .await?;
        let (pred, binds) = Scope::Subject(table.to_string()).predicate("subject");
        let sql = format!("DELETE FROM cache WHERE dataset = ? AND {pred}");
        let mut q = sqlx::query(&sql).bind(dataset);
        for b in &binds {
            q = q.bind(b);
        }
        q.execute(&self.pool).await?;
        sqlx::query("DELETE FROM imports WHERE dataset = ? AND table_name = ?")
            .bind(dataset)
            .bind(table)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn functions_accepting(&self, aspect: &str) -> Result<Vec<String>> {
        let rows = sqlx::query("SELECT name, accepts FROM functions WHERE accepts IS NOT NULL")
            .fetch_all(&self.pool)
            .await?;
        let mut names = Vec::new();
        for r in rows {
            let accepts: Vec<String> = serde_json::from_str(&r.get::<String, _>("accepts"))
                .map_err(|e| Error::Corrupt(format!("ACCEPTS: {e}")))?;
            if accepts.iter().any(|a| a == aspect) {
                names.push(r.get("name"));
            }
        }
        Ok(names)
    }

    /// `(function, aspect)` for every function whose `RETURNS` names an
    /// aspect — the producer of a MEASUREMENT, the voices of a FACT. This
    /// binding is what wires a function's cache into the slots (ruled
    /// 2026-08-04; the function witnesses it replaced were ceremony).
    async fn returning(&self, aspect: Option<&str>) -> Result<Vec<(String, String)>> {
        let q = match aspect {
            Some(a) => {
                sqlx::query("SELECT name, returns FROM functions WHERE returns = ?").bind(a)
            }
            None => sqlx::query("SELECT name, returns FROM functions WHERE returns IS NOT NULL"),
        };
        Ok(q.fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|r| (r.get("name"), r.get("returns")))
            .collect())
    }

    /// The newest write into (subject, aspect) across all slots — what a
    /// detector's verdict must be at least as fresh as.
    pub async fn newest_slot_write(
        &self,
        dataset: &str,
        subject: &str,
        aspect: &str,
    ) -> Result<Option<String>> {
        let mut newest: Option<String> = sqlx::query(
            "SELECT MAX(written_at) AS t FROM glossary \
             WHERE dataset = ? AND subject = ? AND aspect = ?",
        )
        .bind(dataset)
        .bind(subject)
        .bind(aspect)
        .fetch_one(&self.pool)
        .await?
        .get("t");
        for (f, _) in self.returning(Some(aspect)).await? {
            let t: Option<String> = sqlx::query(
                "SELECT MAX(computed_at) AS t FROM cache \
                 WHERE dataset = ? AND subject = ? AND function = ?",
            )
            .bind(dataset)
            .bind(subject)
            .bind(f.as_str())
            .fetch_one(&self.pool)
            .await?
            .get("t");
            if let Some(t) = t
                && newest.as_deref().is_none_or(|n| t.as_str() > n)
            {
                newest = Some(t);
            }
        }
        Ok(newest)
    }

    // -- reads -----------------------------------------------------------

    /// The current slots under a scope: gloss slots by supersession (one per
    /// actor kind), plus the measurement slot of every witness-bound
    /// function, from the cache. Both read shapes build from these.
    async fn slots(
        &self,
        dataset: &str,
        scope: &Scope,
        aspect: Option<&str>,
    ) -> Result<Vec<Slot>> {
        let (pred, binds) = scope.predicate("g.subject");
        let aspect_clause = if aspect.is_some() {
            "AND g.aspect = ? "
        } else {
            ""
        };
        let sql = format!(
            "SELECT g.subject, g.aspect, g.actor_kind, g.actor_id, g.body, g.written_at, \
                    g.snapshot_id \
             FROM glossary g \
             WHERE g.dataset = ? AND {pred} {aspect_clause}AND NOT EXISTS (\
               SELECT 1 FROM glossary n \
               WHERE n.dataset = g.dataset AND n.subject = g.subject \
                 AND n.aspect = g.aspect AND n.actor_kind = g.actor_kind AND n.id > g.id)"
        );
        let mut q = sqlx::query(&sql).bind(dataset);
        for b in &binds {
            q = q.bind(b);
        }
        if let Some(a) = aspect {
            q = q.bind(a);
        }
        let witnesses = self.witnesses_all().await?;
        let witness_on = |aspect: &str| {
            witnesses
                .iter()
                .find(|w| w.aspect == aspect)
                .map(|w| w.name.clone())
        };
        let mut rows: Vec<Slot> = q
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|r| {
                let aspect: String = r.get("aspect");
                Slot {
                    subject: r.get("subject"),
                    rank: match r.get::<String, _>("actor_kind").as_str() {
                        "human" => 0,
                        _ => 1,
                    },
                    actor: r.get("actor_id"),
                    witness: witness_on(&aspect),
                    aspect,
                    body: r.get("body"),
                    written_at: r.get("written_at"),
                    snapshot_id: r.get("snapshot_id"),
                }
            })
            .collect();

        let (cpred, cbinds) = scope.predicate("c.subject");
        for (f, a) in self.returning(aspect).await? {
            let sql = format!(
                "SELECT c.subject, c.body, c.computed_at, c.snapshot_id FROM cache c \
                 WHERE c.dataset = ? AND c.function = ? AND {cpred} AND NOT EXISTS (\
                   SELECT 1 FROM cache n \
                   WHERE n.dataset = c.dataset AND n.subject = c.subject \
                     AND n.function = c.function AND n.id > c.id)"
            );
            let mut q = sqlx::query(&sql).bind(dataset).bind(f.as_str());
            for b in &cbinds {
                q = q.bind(b);
            }
            for c in q.fetch_all(&self.pool).await? {
                rows.push(Slot {
                    subject: c.get("subject"),
                    aspect: a.clone(),
                    rank: 2,
                    actor: f.clone(),
                    witness: witness_on(&a),
                    body: c.get("body"),
                    written_at: c.get("computed_at"),
                    snapshot_id: c.get("snapshot_id"),
                });
            }
        }
        rows.sort_by(|a, b| {
            (&a.subject, &a.aspect, &a.actor).cmp(&(&b.subject, &b.aspect, &b.actor))
        });
        Ok(rows)
    }

    /// The raw read (SPEC.md §5.3): every current slot, one row each;
    /// precedence is the reader's business here. `kind` is the aspect's kind.
    pub async fn raw_read(
        &self,
        dataset: &str,
        scope: &Scope,
        aspect: Option<&str>,
    ) -> Result<Vec<RawRow>> {
        let kinds = self.aspect_kinds().await?;
        Ok(self
            .slots(dataset, scope, aspect)
            .await?
            .into_iter()
            .map(|s| RawRow {
                kind: kinds.get(&s.aspect).cloned().unwrap_or_default(),
                speaker: match s.rank {
                    0 => "human".into(),
                    1 => "agent".into(),
                    _ => "function".into(),
                },
                subject: s.subject,
                aspect: s.aspect,
                witness: s.witness,
                actor: s.actor,
                body: s.body,
                written_at: s.written_at,
            })
            .collect())
    }

    async fn aspect_kinds(&self) -> Result<std::collections::HashMap<String, String>> {
        let rows = sqlx::query("SELECT name, kind FROM aspects")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| (r.get("name"), r.get("kind")))
            .collect())
    }

    /// The collapsed read (SPEC.md §5.3): value by precedence (human over
    /// agent over function), withheld only when the detector's score exceeds
    /// the witness threshold; `state` makes every gap visible — see
    /// [`CollapsedRow`]. The `ReadContext` universe adds `unassessed` rows
    /// for witnessed aspects nobody spoke to.
    pub async fn collapsed_read(
        &self,
        dataset: &str,
        scope: &Scope,
        aspect: Option<&str>,
        ctx: &ReadContext,
    ) -> Result<Vec<CollapsedRow>> {
        let slots = self.slots(dataset, scope, aspect).await?;
        let mut grouped: std::collections::BTreeMap<(String, String), Vec<&Slot>> =
            std::collections::BTreeMap::new();
        for s in &slots {
            grouped
                .entry((s.subject.clone(), s.aspect.clone()))
                .or_default()
                .push(s);
        }

        let witnesses = self.witnesses_all().await?;
        // The detector's verdicts, per subject: (band, score) from its
        // latest cache rows.
        let mut verdicts: std::collections::HashMap<(String, String), (String, f64)> =
            std::collections::HashMap::new();
        for w in &witnesses {
            if let Some(a) = aspect
                && w.aspect != a
            {
                continue;
            }
            let Some(detector) = &w.detector else { continue };
            for c in self.latest_cache(dataset, scope, detector).await? {
                let body: Value = serde_json::from_str(&c.body)
                    .map_err(|e| Error::Corrupt(format!("attest body for `{detector}`: {e}")))?;
                if let (Some(band), Some(score)) = (
                    body.pointer("/band").and_then(Value::as_str),
                    body.pointer("/score").and_then(Value::as_f64),
                ) {
                    verdicts.insert((c.subject.clone(), w.aspect.clone()), (band.into(), score));
                }
            }
        }
        let threshold_of = |aspect: &str| {
            witnesses
                .iter()
                .find(|w| w.aspect == aspect)
                .and_then(|w| w.threshold)
        };

        let mut rows = Vec::new();
        for ((subject, aspect), mut group) in grouped {
            let verdict = verdicts.get(&(subject.clone(), aspect.clone()));
            let (band, score) = match verdict {
                Some((b, s)) => (Some(b.clone()), Some(*s)),
                None => (None, None),
            };
            let contested = matches!(
                (verdict, threshold_of(&aspect)),
                (Some((_, s)), Some(t)) if *s > t
            );
            if contested {
                rows.push(CollapsedRow {
                    subject,
                    aspect,
                    value: None,
                    band,
                    score,
                    state: "contested".into(),
                });
                continue;
            }
            group.sort_by_key(|s| s.rank);
            let serving = group[0];
            // Serve-and-mark (project lead, 2026-08-04): staleness never
            // suppresses a value, it shows beside it.
            let snapshot_moved = serving.snapshot_id.is_some_and(|seen| {
                table_of(&subject)
                    .and_then(|t| ctx.snapshots.get(t))
                    .is_some_and(|current| *current != seen)
            });
            rows.push(CollapsedRow {
                subject,
                aspect,
                value: Some(serving.body.clone()),
                band,
                score,
                state: if snapshot_moved {
                    "stale".into()
                } else {
                    "current".into()
                },
            });
        }

        // Disclosure (fixture 09's benchmark): an aspect somebody is bound
        // to speak to — witnessed, or produced by a function's RETURNS —
        // that nobody spoke to is a visible row, not an omission.
        let mut witnessed: std::collections::BTreeSet<String> = witnesses
            .iter()
            .filter(|w| aspect.is_none_or(|a| w.aspect == a))
            .map(|w| w.aspect.clone())
            .collect();
        witnessed.extend(self.returning(aspect).await?.into_iter().map(|(_, a)| a));
        let present: std::collections::HashSet<(String, String)> = rows
            .iter()
            .map(|r| (r.subject.clone(), r.aspect.clone()))
            .collect();
        for subject in &ctx.universe {
            let in_scope = match scope {
                Scope::Dataset => true,
                Scope::Subject(s) => subject == s || subject.starts_with(&format!("{s}.")),
            };
            if !in_scope {
                continue;
            }
            for a in &witnessed {
                if !present.contains(&(subject.clone(), a.clone())) {
                    rows.push(CollapsedRow {
                        subject: subject.clone(),
                        aspect: a.clone(),
                        value: None,
                        band: None,
                        score: None,
                        state: "unassessed".into(),
                    });
                }
            }
        }
        rows.sort_by(|a, b| (&a.subject, &a.aspect).cmp(&(&b.subject, &b.aspect)));
        Ok(rows)
    }

    /// `ATTEST(...)` (SPEC.md §7.2): detector outputs, served from the
    /// detector function's cache rows in the fixed attest shape.
    pub async fn attest_read(
        &self,
        dataset: &str,
        scope: &Scope,
        aspect: Option<&str>,
    ) -> Result<Vec<AttestRow>> {
        let mut rows = Vec::new();
        for w in self.witnesses_all().await? {
            if let Some(a) = aspect
                && w.aspect != a
            {
                continue;
            }
            let Some(detector) = &w.detector else {
                continue;
            };
            for c in self.latest_cache(dataset, scope, detector).await? {
                let body: Value = serde_json::from_str(&c.body)
                    .map_err(|e| Error::Corrupt(format!("attest body for `{detector}`: {e}")))?;
                let band = body
                    .pointer("/band")
                    .and_then(Value::as_str)
                    .ok_or_else(|| Error::Corrupt(format!("`{detector}` output has no band")))?;
                let score = body
                    .pointer("/score")
                    .and_then(Value::as_f64)
                    .ok_or_else(|| Error::Corrupt(format!("`{detector}` output has no score")))?;
                rows.push(AttestRow {
                    subject: c.subject,
                    aspect: w.aspect.clone(),
                    witness: w.name.clone(),
                    band: band.into(),
                    score,
                    computed_at: c.computed_at,
                });
            }
        }
        rows.sort_by(|a, b| (&a.subject, &a.aspect).cmp(&(&b.subject, &b.aspect)));
        Ok(rows)
    }

    // -- the cache -------------------------------------------------------

    pub async fn cache_get(
        &self,
        dataset: &str,
        subject: &str,
        function: &str,
    ) -> Result<Option<CacheRow>> {
        let row = sqlx::query(
            "SELECT subject, function, body, computed_at FROM cache \
             WHERE dataset = ? AND subject = ? AND function = ? \
             ORDER BY id DESC LIMIT 1",
        )
        .bind(dataset)
        .bind(subject)
        .bind(function)
        .fetch_optional(&self.pool)
        .await?;
        row.map(cache_row).transpose()
    }

    pub async fn cache_put(
        &self,
        dataset: &str,
        subject: &str,
        function: &str,
        body: &str,
        snapshot_id: Option<i64>,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO cache (dataset, subject, function, body, snapshot_id) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(dataset)
        .bind(subject)
        .bind(function)
        .bind(body)
        .bind(snapshot_id)
        .execute(&self.pool)
        .await?;
        // A function's new value invalidates like a gloss would: through
        // the aspect its RETURNS names. Detectors return no aspect, so
        // their verdicts invalidate nothing.
        let returns: Option<String> = sqlx::query("SELECT returns FROM functions WHERE name = ?")
            .bind(function)
            .fetch_optional(&self.pool)
            .await?
            .and_then(|r| r.get("returns"));
        if let Some(aspect) = returns {
            self.invalidate(dataset, &aspect, subject).await?;
        }
        Ok(())
    }

    async fn latest_cache(
        &self,
        dataset: &str,
        scope: &Scope,
        function: &str,
    ) -> Result<Vec<CacheRow>> {
        let (pred, binds) = scope.predicate("c.subject");
        let sql = format!(
            "SELECT c.subject, c.function, c.body, c.computed_at FROM cache c \
             WHERE c.dataset = ? AND c.function = ? AND {pred} AND NOT EXISTS (\
               SELECT 1 FROM cache n \
               WHERE n.dataset = c.dataset AND n.subject = c.subject \
                 AND n.function = c.function AND n.id > c.id) \
             ORDER BY c.subject"
        );
        let mut q = sqlx::query(&sql).bind(dataset).bind(function);
        for b in &binds {
            q = q.bind(b);
        }
        q.fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(cache_row)
            .collect()
    }

    // -- SQL forwarded from the session ----------------------------------

    /// `DELETE FROM glossary … / DELETE FROM cache …` — removal is SQL
    /// (SPEC.md §5.2, §6). The session routes only these two relations here;
    /// the target is re-checked because this executes verbatim.
    ///
    /// A glossary delete also drops every detector verdict (2026-08-05):
    /// verdict freshness is a timestamp comparison against the newest slot,
    /// and deletion makes the slot set smaller, never newer — without this a
    /// contested verdict would keep withholding a value after the disputed
    /// slot was struck. The SQL runs verbatim, so which subjects changed is
    /// unknown here; verdicts recompute at the next read.
    pub async fn forward_delete(&self, target: &str, sql: &str) -> Result<u64> {
        if target != "glossary" && target != "cache" {
            return Err(Error::ForwardRejected(target.into()));
        }
        let done = sqlx::raw_sql(sql).execute(&self.pool).await?;
        if target == "glossary" && done.rows_affected() > 0 {
            sqlx::query(
                "DELETE FROM cache WHERE function IN \
                 (SELECT name FROM functions WHERE returns IS NULL)",
            )
            .execute(&self.pool)
            .await?;
        }
        Ok(done.rows_affected())
    }

    /// Full relation dump for substrate `SELECT`s over the store's
    /// relations — the names and column shapes live in [`RELATIONS`].
    pub async fn relation_rows(&self, table: &str) -> Result<Vec<Vec<Option<String>>>> {
        let Some(relation) = RELATIONS.iter().find(|r| r.name == table) else {
            return Err(Error::ForwardRejected(table.into()));
        };
        let rows = sqlx::query(relation.sql).fetch_all(&self.pool).await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                (0..r.len())
                    .map(|i| r.get::<Option<String>, _>(i))
                    .collect()
            })
            .collect())
    }

    // -- lookups the session needs ---------------------------------------

    pub async fn dataset_exists(&self, name: &str) -> Result<bool> {
        Ok(sqlx::query("SELECT 1 FROM datasets WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?
            .is_some())
    }

    pub async fn aspect(&self, name: &str) -> Result<Option<(Value, String)>> {
        let Some(row) = sqlx::query("SELECT schema, kind FROM aspects WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?
        else {
            return Ok(None);
        };
        let schema: Value = serde_json::from_str(&row.get::<String, _>("schema"))
            .map_err(|e| Error::Corrupt(format!("aspect `{name}` schema: {e}")))?;
        Ok(Some((schema, row.get("kind"))))
    }

    /// Resolve a function visible from `dataset` (`FOR` scope or GLOBAL,
    /// SPEC.md §6). `None` skips the visibility check.
    pub async fn function(&self, name: &str, dataset: Option<&str>) -> Result<Option<FunctionRow>> {
        let Some(row) = sqlx::query(
            "SELECT name, scope_dataset, script, accepts, returns \
             FROM functions WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?
        else {
            return Ok(None);
        };
        let scope_dataset: Option<String> = row.get("scope_dataset");
        if let (Some(d), Some(scope)) = (dataset, &scope_dataset)
            && scope != d
        {
            return Ok(None);
        }
        let returns: Option<String> = row.get("returns");
        let accepts = match row.get::<Option<String>, _>("accepts") {
            None => Vec::new(),
            Some(text) => serde_json::from_str::<Vec<String>>(&text)
                .map_err(|e| Error::Corrupt(format!("function `{name}` ACCEPTS: {e}")))?,
        };
        Ok(Some(FunctionRow {
            name: row.get("name"),
            scope_dataset,
            script: row.get("script"),
            accepts,
            returns,
        }))
    }

    pub async fn witnesses_on(&self, aspect: &str) -> Result<Vec<WitnessRow>> {
        Ok(self
            .witnesses_all()
            .await?
            .into_iter()
            .filter(|w| w.aspect == aspect)
            .collect())
    }

    pub async fn witnesses_all(&self) -> Result<Vec<WitnessRow>> {
        let rows = sqlx::query(
            "SELECT name, aspect, speakers, detector, threshold FROM witnesses ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|r| {
                let speakers: Value = serde_json::from_str(&r.get::<String, _>("speakers"))
                    .map_err(|e| Error::Corrupt(format!("witness speakers: {e}")))?;
                let list = speakers.as_array().cloned().unwrap_or_default();
                Ok(WitnessRow {
                    name: r.get("name"),
                    aspect: r.get("aspect"),
                    admits_agent: list.iter().any(|s| s.as_str() == Some("agent")),
                    admits_human: list.iter().any(|s| s.as_str() == Some("human")),
                    detector: r.get("detector"),
                    threshold: r.get("threshold"),
                })
            })
            .collect()
    }

    async fn require(&self, what: &'static str, table: &str, name: &str) -> Result<()> {
        let sql = format!("SELECT 1 FROM {table} WHERE name = ?");
        if sqlx::query(&sql)
            .bind(name)
            .fetch_optional(&self.pool)
            .await?
            .is_none()
        {
            return Err(Error::Unknown {
                what,
                name: name.into(),
            });
        }
        Ok(())
    }
}

/// The table a subject's snapshot rides on: its first path segment. Pair
/// paths (they contain spaces) have none.
fn table_of(subject: &str) -> Option<&str> {
    if subject.contains(' ') {
        return None;
    }
    Some(subject.split('.').next().unwrap_or(subject))
}

fn settings_json(settings: &[glossql_parser::Setting]) -> String {
    use glossql_parser::SettingValue;
    let map: serde_json::Map<String, Value> = settings
        .iter()
        .map(|s| {
            let v = match &s.value {
                SettingValue::Name(n) => Value::String(n.value.clone()),
                SettingValue::String(t) => Value::String(t.clone()),
                SettingValue::Number(n) => {
                    serde_json::from_str(n).unwrap_or_else(|_| Value::String(n.clone()))
                }
            };
            (s.key.value.clone(), v)
        })
        .collect();
    Value::Object(map).to_string()
}

fn kind_str(kind: AspectKind) -> &'static str {
    match kind {
        AspectKind::Measurement => "measurement",
        AspectKind::Fact => "fact",
        AspectKind::Query => "query",
    }
}

fn validate(schema: &Value, instance: &Value, which: String) -> Result<()> {
    crate::schemas::validate_instance(schema, instance)
        .map_err(|detail| Error::BodyRejected { which, detail })
}

fn cache_row(r: sqlx::sqlite::SqliteRow) -> Result<CacheRow> {
    Ok(CacheRow {
        subject: r.get("subject"),
        function: r.get("function"),
        body: r.get("body"),
        computed_at: r.get("computed_at"),
    })
}
