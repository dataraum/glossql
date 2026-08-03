//! The import path: recipes land tables — ADBC / CSV / parquet sources to
//! Arrow batches to the raw layer; the typing pass produces typed views and
//! quarantine (TRY_CAST discipline; layers are engine namespaces, not
//! grammar). Milestone 2 fills this in.
