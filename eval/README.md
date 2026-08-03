# stack evaluation rig

Disposable machinery behind `reports/2026-08-03-stack-eval.md` — DataFusion
vs DuckDB on glossql-shaped workloads over synthetic HTTP-telemetry data.

```sh
python3 -m venv .venv && .venv/bin/pip install duckdb datafusion pyarrow polars numpy
.venv/bin/python run.py                      # 5M rows
GLOSSQL_EVAL_N=50000000 .venv/bin/python run.py   # 50M rows (delete data/ first)
```
