//! glossql front parser on DataFusion's parser machinery.
//!
//! [`GlossqlParser`] wraps `DFParser` — the in-tree custom-statement
//! pattern: one stock-sqlparser token stream (postgres dialect), glossql
//! heads hand-parsed with `Parser` primitives, every other statement
//! delegated and carried as [`Statement::Substrate`]. The substrate is
//! DataFusion's to plan; this crate never re-parses SQL, and the
//! `GLOSSARY()` / `ATTEST()` reads live in the substrate as ordinary table
//! functions.
//!
//! The corpus under `corpus/` is this crate's acceptance suite — the
//! standing invariant. Spelling constraints (dollar-quoted bodies, string
//! schema pointers, dollar-quoted recipe tails) and their rationale:
//! `reports/2026-08-03-sqlparser-respell.md`.

mod ast;
mod parser;

pub use ast::*;
pub use parser::GlossqlParser;
