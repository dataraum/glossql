//! Workspace state: the seven declarations (sources, recipes, datasets,
//! relationships, aspects, functions, witnesses) plus USE binding.
//!
//! Persisted as the declaration log itself — replayed through `parser` on
//! open, so there is no second catalog format. Milestone 1 fills this in.
