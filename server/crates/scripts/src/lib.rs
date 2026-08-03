//! The Rhai host: function scripts over Arc'd Arrow column handles
//! (zero-copy per the 2026-08-03 spike — scripts orchestrate, vectorized
//! host kernels compute). ACCEPTS/RETURNS JSON-schema validation at the
//! boundary only. Milestone 3 fills this in.
