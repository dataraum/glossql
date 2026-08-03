#!/usr/bin/env python3
"""glossql-shaped workloads on DataFusion. Prints one JSON object of timings (s).

Same SQL shapes as sim_duckdb.py; percentiles use approx_percentile_cont to
match DuckDB's approx_quantile.
"""

from __future__ import annotations

import json
import math
import sys
from pathlib import Path
from time import perf_counter

import datafusion
import pyarrow as pa
import pyarrow.compute as pc
from datafusion import SessionContext, udf

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

ctx = SessionContext()
for t in ("requests", "deploys", "hosts", "glossary"):
    ctx.register_parquet(t, str(DATA / f"{t}.parquet"))

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
        results[name] = f"ERROR: {str(e).splitlines()[0][:200]}"


def w1_scan_aggregate() -> None:
    ctx.sql("""
      SELECT service, endpoint, count(*) AS n,
             approx_percentile_cont(latency_ms, 0.5)  AS p50,
             approx_percentile_cont(latency_ms, 0.95) AS p95,
             approx_percentile_cont(latency_ms, 0.99) AS p99
      FROM requests WHERE status_code = 200
      GROUP BY service, endpoint ORDER BY service, endpoint""").collect()


def w2_measurement_fanout() -> None:
    for c in REQ_COLS:
        ctx.sql(f"""
          SELECT count(*), count({c}), count(DISTINCT {c}), min({c}), max({c})
          FROM requests""").collect()


def w3_supersession() -> None:
    ctx.sql(LATEST_CTE + " SELECT count(*) FROM latest").collect()


def w3b_collapsed_read() -> None:
    ctx.sql(COLLAPSED).collect()


def w4_json_extract() -> None:
    # DataFusion core ships no JSON path functions (they live in the separate
    # datafusion-functions-json crate); try candidates so the gap is measured,
    # not assumed.
    last = None
    for fn in ("json_extract_string(body, '$.sql')",
               "json_get_str(body, 'sql')",
               "json_extract(body, '$.sql')"):
        try:
            ctx.sql(LATEST_CTE + f"""
              SELECT count(*) FROM (
                SELECT {fn} AS s FROM latest
                WHERE aspect = 'meaning' AND actor_kind = 'agent') t
              WHERE s LIKE 'SELECT%'""").collect()
            return
        except Exception as e:  # noqa: BLE001
            last = e
    raise RuntimeError(f"no JSON path function available ({str(last).splitlines()[0][:120]})")


def _apdex(arr: pa.Array) -> pa.Array:
    return pc.if_else(pc.less_equal(arr, 100.0), pa.scalar("satisfied"),
                      pc.if_else(pc.less_equal(arr, 400.0), pa.scalar("tolerating"),
                                 pa.scalar("frustrated")))


def w5_udf_apdex() -> None:
    ctx.sql("""
      SELECT apdex(latency_ms) AS a, count(*) FROM requests
      WHERE latency_ms IS NOT NULL GROUP BY a""").collect()


def w6_attest_sweep() -> None:
    ctx.sql(f"SELECT count(*) FROM ({COLLAPSED}) t WHERE band = 'red'").collect()


ctx.register_udf(udf(_apdex, [pa.float64()], pa.utf8(), "stable", name="apdex"))

timed("w1_scan_aggregate", w1_scan_aggregate)
timed("w2_measurement_fanout", w2_measurement_fanout)
timed("w3_supersession", w3_supersession)
timed("w3b_collapsed_read", w3b_collapsed_read)
timed("w4_json_extract", w4_json_extract)
timed("w5_udf_apdex", w5_udf_apdex)
timed("w6_attest_sweep", w6_attest_sweep)

print(json.dumps({"engine": f"datafusion {datafusion.__version__}", "timings": results}))
sys.stdout.flush()
