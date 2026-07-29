# glossql

A declarative extension of SQL for the analytics context of a dataset: the assertions
made about data, the evidence measured from data, the policies for judging that
evidence, and the serving of all three to analytics agents. A context is a pair of
stores — an append-only **log** of authored statements (text) and a **lake** of data
plus bulk observation results (columnar) — with one invariant: state = f(log, lake).

The full definition is **[SPEC.md](./SPEC.md)** — the single normative document of
this repository.

The name: a *gloss* is a marginal annotation explaining a text's meaning; a glossary
is a collection of them.

## Status

- v0 draft specification, under review. No implementation.
- Decided: the language comes first; Apache DataFusion is the implementation
  substrate for the server that follows.
- Held open: persistence backend, the DataFusion mapping, governance.

## Relationship to dataraum-context

The sibling repository [`dataraum-context`](../dataraum-context) is the current
production system (v0.3). Its pipeline, detectors, and teach mechanisms are the
fieldwork that determined this language's statement vocabulary — SPEC.md §2 maps
every artifact of that system onto a construct here. As this repository becomes the
context server, dataraum-context moves to the system layer above it (agents,
orchestration, UI).

## Prior art

- [ggsql](https://github.com/posit-dev/ggsql) — grammar-of-graphics clauses as a SQL
  extension; the pattern for a declarative tail on SELECT.
- [Snowflake semantic views](https://docs.snowflake.com/en/sql-reference/sql/create-semantic-view) —
  declared semantics as SQL objects consumed by agents; declarations only, no
  evidence model.
- [Open Semantic Interchange / Apache Ossie](https://github.com/open-semantic-interchange/osi) —
  vendor-neutral interchange for semantic models; a possible export mapping for the
  vocabulary tier.
- [Extending SQL in DataFusion](https://datafusion.apache.org/blog/2026/01/12/extending-sql/) —
  the extension points the server will build on.
- [DuckLake](https://ducklake.select/) — lakehouse design: parquet data files, all
  metadata in a transactional SQL database. The parquet+DB pattern is the leading
  persistence candidate (SPEC.md §1.1); the
  [datafusion-ducklake](https://github.com/hotdata-dev/datafusion-ducklake) crate is
  reference material, not a dependency.

What none of these carry — and this language treats as first-class — is evidence and
adjudication: declarations here have provenance and confidence, are witnessed by
detectors, and can be *contested*.
