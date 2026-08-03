#!/usr/bin/env python3
"""glossql-shaped workloads on DuckDB. Prints one JSON object of timings (s)."""

from __future__ import annotations

import json
import math
import sys
from pathlib import Path
from time import perf_counter

import duckdb
import pyarrow as pa
import pyarrow.compute as pc
from duckdb.sqltypes import DOUBLE, VARCHAR

DATA = Path(__file__).parent / "data"
REQ_COLS = ["ts", "service", "endpoint", "region", "status_code",
            "latency_ms", "bytes_out", "deploy_id", "host_id"]

LATEST_CTE = """WITH latest AS (
  SELECT * FROM (
    SELECT *, row_number() OVER (
      PARTITION BY subject, aspect, actor_kind ORDER BY written_at DESC) AS rn
    FROM glossary) t WHERE rn = 1
)"""

COLLAPSED = LATEST_CTE + """
SELECT subject, aspect,
       (count(DISTINCT value) - 1) / 2.0 AS score,
       CASE count(DISTINCT value) WHEN 1 THEN 'green' WHEN 2 THEN 'orange'
            ELSE 'red' END AS band
FROM latest GROUP BY subject, aspect"""

con = duckdb.connect()
for t in ("requests", "deploys", "hosts", "glossary"):
    con.execute(f"CREATE VIEW {t} AS SELECT * FROM read_parquet('{DATA}/{t}.parquet')")

results: dict[str, float | str] = {}


def timed(name: str, fn, repeat: int = 2) -> None:
    best = math.inf
    try:
        for _ in range(repeat):
            t0 = perf_counter()
            fn()
            best = min(best, perf_counter() - t0)
        results[name] = round(best, 3)
    except Exception as e:  # noqa: BLE001 — report the failure, keep running
        results[name] = f"ERROR: {e}"


def w1_scan_aggregate() -> None:
    con.execute("""
      SELECT service, endpoint, count(*) AS n,
             approx_quantile(latency_ms, 0.5)  AS p50,
             approx_quantile(latency_ms, 0.95) AS p95,
             approx_quantile(latency_ms, 0.99) AS p99
      FROM requests WHERE status_code = 200
      GROUP BY service, endpoint ORDER BY service, endpoint""").fetchall()


def w2_measurement_fanout() -> None:
    for c in REQ_COLS:
        con.execute(f"""
          SELECT count(*), count({c}), count(DISTINCT {c}), min({c}), max({c})
          FROM requests""").fetchall()


def w3_supersession() -> None:
    con.execute(LATEST_CTE + " SELECT count(*) FROM latest").fetchall()


def w3b_collapsed_read() -> None:
    con.execute(COLLAPSED).fetchall()


def w4_json_extract() -> None:
    con.execute(LATEST_CTE + """
      SELECT count(*) FROM (
        SELECT json_extract_string(body, '$.sql') AS s FROM latest
        WHERE aspect = 'meaning' AND actor_kind = 'agent') t
      WHERE s LIKE 'SELECT%'""").fetchall()


def _apdex(arr):
    return pc.if_else(pc.less_equal(arr, 100.0), pa.scalar("satisfied"),
                      pc.if_else(pc.less_equal(arr, 400.0), pa.scalar("tolerating"),
                                 pa.scalar("frustrated")))


def w5_udf_apdex() -> None:
    con.execute("""
      SELECT apdex(latency_ms) AS a, count(*) FROM requests
      WHERE latency_ms IS NOT NULL GROUP BY a""").fetchall()


def w6_attest_sweep() -> None:
    con.execute(f"SELECT count(*) FROM ({COLLAPSED}) t WHERE band = 'red'").fetchall()


con.create_function("apdex", _apdex, [DOUBLE], VARCHAR, type="arrow")

timed("w1_scan_aggregate", w1_scan_aggregate)
timed("w2_measurement_fanout", w2_measurement_fanout)
timed("w3_supersession", w3_supersession)
timed("w3b_collapsed_read", w3b_collapsed_read)
timed("w4_json_extract", w4_json_extract)
timed("w5_udf_apdex", w5_udf_apdex)
timed("w6_attest_sweep", w6_attest_sweep)

print(json.dumps({"engine": f"duckdb {duckdb.__version__}", "timings": results}))
sys.stdout.flush()
