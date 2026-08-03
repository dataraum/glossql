#!/usr/bin/env python3
"""Synthetic performance-telemetry sample data for the stack evaluation.

Domain: HTTP request telemetry (services, endpoints, deploys, hosts) —
deliberately not finance. Columns are derived from the row id by
multiplicative hashing: deterministic, fast, skewed enough to be interesting.

Outputs (eval/data/):
  requests.parquet   N rows (default 5M)   the fact table
  deploys.parquet    200 rows              requests.deploy_id -> deploys.deploy_id
  hosts.parquet      80 rows               requests.host_id  -> hosts.host_id
  glossary.parquet   ~60k rows             glosses with JSON bodies, several
                                           versions per (subject, aspect, actor_kind)
"""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path

import polars as pl

DATA = Path(__file__).parent / "data"
N = int(os.environ.get("GLOSSQL_EVAL_N", 5_000_000))

SERVICES = ["checkout", "search", "catalog", "auth", "cart", "shipping", "profile", "recs"]
ENDPOINTS = ["GET /items", "POST /order", "GET /status", "PUT /update", "GET /list", "DELETE /item"]
REGIONS = ["eu-west", "eu-central", "us-east", "us-west", "ap-south"]


def h(col: str, a: int, m: int) -> pl.Expr:
    return (pl.col(col) * a) % m


def pick(col: str, a: int, values: list[str]) -> pl.Expr:
    return h(col, a, len(values)).replace_strict(dict(enumerate(values)), return_dtype=pl.Utf8)


def gen_requests() -> None:
    df = pl.select(pl.int_range(0, N, dtype=pl.Int64).alias("id")).with_columns(
        (pl.lit(1_722_600_000_000) + (pl.col("id") // 12) * 1000).cast(pl.Datetime("ms")).alias("ts"),
        pick("id", 2654435761, SERVICES).alias("service"),
        pick("id", 40503, ENDPOINTS).alias("endpoint"),
        pick("id", 97, REGIONS).alias("region"),
        pl.when(h("id", 613, 100) < 96).then(200)
          .when(h("id", 613, 100) < 98).then(404)
          .when(h("id", 613, 100) < 99).then(500).otherwise(503)
          .cast(pl.Int16).alias("status_code"),
        # skewed latency: bulk 5-500ms, a slow tail up to ~10s, 2% nulls (lost spans)
        pl.when(h("id", 50, 100) == 0).then(None)
          .when(h("id", 887, 100) < 90).then(5.0 + h("id", 7919, 4950) / 10.0)
          .otherwise(500.0 + h("id", 104729, 95000) / 10.0)
          .alias("latency_ms"),
        (h("id", 31, 65536) * 17).alias("bytes_out"),
        h("id", 131, 200).alias("deploy_id"),
        h("id", 1543, 80).alias("host_id"),
    ).drop("id")
    df.write_parquet(DATA / "requests.parquet")


def gen_dims() -> None:
    pl.select(pl.int_range(0, 200, dtype=pl.Int64).alias("deploy_id")).with_columns(
        pick("deploy_id", 7, SERVICES).alias("service"),
        pl.format("v1.{}.{}", h("deploy_id", 3, 40), h("deploy_id", 11, 10)).alias("version"),
        (pl.lit(1_722_000_000_000) + pl.col("deploy_id") * 3_600_000).cast(pl.Datetime("ms")).alias("deployed_at"),
    ).write_parquet(DATA / "deploys.parquet")

    pl.select(pl.int_range(0, 80, dtype=pl.Int64).alias("host_id")).with_columns(
        pick("host_id", 13, REGIONS).alias("region"),
        pick("host_id", 5, ["m5.large", "m5.xlarge", "c5.2xlarge"]).alias("instance_type"),
        (2 + h("host_id", 3, 3) * 2).alias("cpu_cores"),
    ).write_parquet(DATA / "hosts.parquet")


ASPECTS = ["meaning", "unit", "behavior", "threshold"]
KINDS = ["measurement", "agent", "human"]
VALUES = {"meaning": ["duration", "size", "count"],
          "unit": ["ms", "s", "bytes"],
          "behavior": ["flow", "stock", "gauge"],
          "threshold": ["strict", "lenient", "default"]}


def gen_glossary() -> None:
    rows = []
    written = 0
    for s in range(2000):  # subjects: telemetry.<table>.<col>-style paths
        subject = f"telemetry.requests.col_{s}"
        for aspect in ASPECTS:
            for kind in KINDS:
                versions = 1 + (s * 31 + len(aspect)) % 4  # 1-4 versions per slot
                for v in range(versions):
                    value = VALUES[aspect][(s + v + len(kind)) % 3]
                    body = {"value": value, "note": f"rev {v}"}
                    if aspect == "meaning" and kind == "agent":
                        value = "grounding"
                        body = {"sql": f"SELECT latency_ms FROM requests WHERE service = 'svc{s % 8}'",
                                "assumptions": [{"assumption": "spans complete", "confidence": 0.9}]}
                    written += 1
                    rows.append((subject, aspect, kind, f"{kind}_1", value,
                                 json.dumps(body), written))
    pl.DataFrame(rows, schema=["subject", "aspect", "actor_kind", "actor",
                               "value", "body", "written_at"], orient="row"
                 ).write_parquet(DATA / "glossary.parquet")


if __name__ == "__main__":
    DATA.mkdir(exist_ok=True)
    gen_requests()
    gen_dims()
    gen_glossary()
    sizes = {p.name: f"{p.stat().st_size / 1e6:.1f}MB" for p in sorted(DATA.glob("*.parquet"))}
    print(json.dumps({"rows": N, "files": sizes}), file=sys.stdout)
