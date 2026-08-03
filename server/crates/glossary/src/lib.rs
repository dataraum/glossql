//! The glossary store: append-only gloss and measurement rows under the
//! supersession key (subject, aspect, actor kind), relational via sqlx
//! (SQLite in the workspace, Postgres by connection string). Rows carry the
//! data snapshot id they were computed against. `GLOSSARY()` / `ATTEST()`
//! read over the latest/collapsed views. Milestone 1 fills this in.
