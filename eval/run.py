#!/usr/bin/env python3
"""Run the stack evaluation: generate data if missing, run both sims, print a
comparison table. Usage: <venv-python> eval/run.py  (GLOSSQL_EVAL_N overrides
the row count, default 5M)."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).parent


def run(script: str) -> dict:
    p = subprocess.run([sys.executable, str(HERE / script)],
                       capture_output=True, text=True, check=False)
    if p.returncode != 0:
        print(p.stderr, file=sys.stderr)
        raise SystemExit(f"{script} failed")
    return json.loads(p.stdout.strip().splitlines()[-1])


if not (HERE / "data" / "requests.parquet").exists():
    print(f"generating data: {run('gen_data.py')}", file=sys.stderr)

duck = run("sim_duckdb.py")
df = run("sim_datafusion.py")

names = list(duck["timings"])
print(f"| workload | {duck['engine']} | {df['engine']} |")
print("|---|---|---|")
for n in names:
    a, b = duck["timings"][n], df["timings"].get(n, "-")
    print(f"| {n} | {a} | {b} |")
