# 2026-08-03 — tech stack evaluation: DataFusion vs DuckDB extension

Question: which substrate carries the glossql PoC —

- **DataFusion** (Rust): ADBC/Arrow imports, parquet backend, Rhai for
  function scripts, ndarray/candle for tensors, polars/arrow interop.
- **DuckDB extension** (C++/Python): PEG parser for the grammar extension,
  Python function scripts over Arrow.

## What was simulated

Python harness in `eval/` (`<venv-python> eval/run.py`; deps: duckdb,
datafusion, pyarrow, polars, numpy; `GLOSSQL_EVAL_N` sets the row count).
Sample domain: HTTP performance telemetry — request rows (service, endpoint,
region, status, latency_ms, bytes, deploy, host), deploy and host dimensions,
and a 60k-row glossary with JSON bodies and 1–4 versions per
(subject, aspect, actor kind). Machine: Apple M5 Pro, 48 GB. Engines:
duckdb 1.5.5, datafusion 54.0.0, both via their Python bindings; identical
SQL per workload (both sides use approx percentiles); best of 2 runs.

| workload | what it stands for | duckdb 5M | datafusion 5M | duckdb 50M | datafusion 50M |
|---|---|---|---|---|---|
| w1 scan-aggregate | extract/recipe queries (p50/p95/p99 by service × endpoint) | 0.025 | 0.017 | 0.203 | 0.137 |
| w2 measurement fan-out | per-column profile sweep, 9 columns | 0.038 | 0.033 | 0.248 | 0.260 |
| w3 supersession | latest per (subject, aspect, actor kind), window fn | 0.002 | 0.006 | 0.002 | 0.006 |
| w3b collapsed read | GLOSSARY() default: distinct-value score + band | 0.006 | 0.008 | 0.006 | 0.008 |
| w4 JSON extract | pull `$.sql` out of grounding bodies | 0.001 | unsupported | 0.001 | unsupported |
| w5 Arrow UDF | 50M rows through a registered script-boundary function | 1.14 | 0.119 | 11.3 | 1.11 |
| w6 attest sweep | WHERE band = 'red' over the collapsed relation | 0.004 | 0.008 | 0.005 | 0.008 |

## Reading the numbers

- **The query plane is a tie.** Scan/aggregate and the profile fan-out are
  within tens of milliseconds of each other at both scales. Neither engine is
  a bottleneck for the extract/measurement side.
- **The glossary plane is trivial for both.** The context store is small
  data; supersession, collapse, and sweeps are single-digit milliseconds.
  Engine choice will never be decided here.
- **The script boundary is the discriminator.** w5 pushes 50M rows through
  the *same* pyarrow kernel registered as a UDF in both engines: DataFusion
  1.1 s, DuckDB 11.3 s — a ~10× marshalling cost on DuckDB's Python-UDF path,
  linear in both engines. Caveat: this measures the function *boundary*, not
  the task (a native SQL CASE does the same job in ~0.1 s on either engine) —
  but the boundary is exactly where glossql's functions-as-scripts live.
- **JSON: batteries vs toolkit, confirmed empirically.** DuckDB ships JSON
  path functions; the DataFusion Python wheel has none (they live in the
  separate `datafusion-functions-json` crate). In a Rust server the gap
  closes by adding the crate; it still says something true about how much
  assembly each option needs.

## What Python cannot measure — and weighs more than the numbers

1. **The grammar extension path.** The grammar is the product. DataFusion's
   extension surface (custom statements through the parser, logical/physical
   plan hooks) is documented and usable today. DuckDB's PEG parser is not
   official until v2, ~2–3 months out — building the core of the language on
   it puts the critical path on someone else's roadmap, with rework risk if
   the pre-release surface shifts.
2. **The script model.** Rhai embeds in-process, is sandboxed by default (no
   IO unless the host registers it), and is deterministic by construction —
   which is a *design assumption* of the language (deterministic functions
   underpin the no-negative-forms decision and the witness replay story).
   Python scripts offer the bigger ecosystem — the running system's phases
   are Python — but no determinism or sandbox guarantee, plus the measured
   10× boundary cost.
3. **Storage.** DuckDB brings a storage engine; DataFusion is a query engine
   over storage you choose. The DataFusion option already names parquet as
   the backend, which fits; the persistence decision itself stays deferred.
4. **Language cost.** Rust (readable, but Rust) vs a C++ extension shell
   around Python scripting ("kind of muddly"). This one is a preference the
   numbers can't settle.

## Matrix

| dimension | DataFusion | DuckDB extension |
|---|---|---|
| query performance | tie | tie |
| glossary ops | tie | tie |
| script boundary | ~10× faster measured; Rhai sandboxed + deterministic | Python ecosystem; no guarantees, slower boundary |
| grammar extension | available + documented now | PEG official in ~2–3 months |
| JSON handling | add a crate | built in |
| storage | bring your own (parquet named) | built in |
| implementation language | Rust | C++ shell + Python |

## Recommendation

**DataFusion.** The two dimensions that dominate — extending the grammar now
rather than in 2–3 months, and a script model that matches the language's
determinism assumption while being an order of magnitude cheaper at the
boundary — both point the same way, and the query-plane numbers show nothing
is given up in exchange. The costs are named: Rust, JSON via a crate, and a
storage story to assemble (parquet, already named). A DuckDB PoC would be
faster to *start* and slower to *finish*: the muddle arrives exactly at the
two places glossql is unusual — grammar and scripts.
