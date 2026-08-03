//! AST for the glossql statement forms (`grammar.ebnf`).
//!
//! Names are sqlparser [`Ident`]s (quoting and spans ride along). Substrate
//! SQL is not modelled here — it stays a DataFusion
//! [`DFStatement`](datafusion_sql::parser::Statement).

use datafusion_sql::parser::Statement as DFStatement;
use datafusion_sql::sqlparser::ast::Ident;
use serde_json::Value;

/// A JSON object body: the parsed value plus the verbatim dollar-quoted
/// content (byte-exact, useful for hashing and display).
#[derive(Debug, Clone, PartialEq)]
pub struct JsonBody {
    pub value: Value,
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Declare(Box<Declaration>),
    Use(Use),
    Gloss(Gloss),
    Extract(Extract),
    /// Host SQL, parsed by `DFParser` — includes `GLOSSARY()` / `ATTEST()`
    /// reads, views, and glossary/cache DELETEs.
    Substrate(Box<DFStatement>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Declaration {
    Source(SourceDecl),
    Recipe(RecipeDecl),
    Dataset(DatasetDecl),
    Relationship(RelationshipDecl),
    Aspect(AspectDecl),
    Function(FunctionDecl),
    Witness(WitnessDecl),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceDecl {
    pub name: Ident,
    pub settings: Vec<Setting>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DatasetDecl {
    pub name: Ident,
    pub settings: Vec<Setting>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Setting {
    pub key: Ident,
    pub value: SettingValue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SettingValue {
    Name(Ident),
    String(String),
    /// Decimal literal, kept verbatim.
    Number(String),
}

/// `DECLARE RECIPE r ON dataset FROM source AS $$ <sql> $$` — the SQL runs
/// at the source, in the source's dialect; it is dollar-quoted so
/// foreign-dialect text never meets the host tokenizer.
#[derive(Debug, Clone, PartialEq)]
pub struct RecipeDecl {
    pub table: Ident,
    pub dataset: Ident,
    pub source: Ident,
    pub sql: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RelationshipDecl {
    pub left: ColumnPath,
    pub op: RelOp,
    pub right: ColumnPath,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelOp {
    /// `->` — many-to-one, the FK direction.
    ManyToOne,
    /// `<->` — one-to-one.
    OneToOne,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AspectDecl {
    pub name: Ident,
    pub schema: JsonBody,
    pub kind: AspectKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AspectKind {
    Measurement,
    Fact,
    Query,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub name: Ident,
    pub scope: FunctionScope,
    /// The script reference named by `FROM`.
    pub script: String,
    /// `ACCEPTS (aspect, …)` — aspects whose current values the server
    /// hands the script as its context document; empty = no context.
    pub accepts: Vec<Ident>,
    pub returns: JsonBody,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FunctionScope {
    Dataset(Ident),
    Global,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WitnessDecl {
    pub name: Ident,
    pub aspect: Ident,
    pub speakers: Vec<Speaker>,
    pub detector: Option<Ident>,
    /// Decimal literal in 0..1, kept verbatim; range is checked at
    /// admission, not in the grammar.
    pub threshold: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Speaker {
    Function(Ident),
    Agent,
    Human,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Use {
    pub dataset: Ident,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Gloss {
    pub aspect: Ident,
    pub subject: Subject,
    pub body: JsonBody,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Subject {
    Path(Path),
    Pair(Box<PairPath>),
}

/// `dataset | dataset.table | dataset.table.column` — one to three
/// segments; unprefixed paths resolve against the USE'd dataset, which is
/// not the parser's business.
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub segments: Vec<Ident>,
}

/// `[dataset.]table.column`.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnPath {
    pub dataset: Option<Ident>,
    pub table: Ident,
    pub column: Ident,
}

/// A declared relationship, addressed by its pair path (relationships have
/// no names).
#[derive(Debug, Clone, PartialEq)]
pub struct PairPath {
    pub left: ColumnPath,
    pub op: RelOp,
    pub right: ColumnPath,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Extract {
    /// Bare function names — calls carry no arguments; settings are context.
    pub calls: Vec<Ident>,
    pub subject: Subject,
}
