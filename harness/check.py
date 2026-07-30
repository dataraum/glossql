#!/usr/bin/env python3
"""§9.1 harness checker — the standing invariant.

- Every ```sql block in SPEC.md must parse.
- Every ```glossql block in corpus/*.md must parse.
- Every ```glossql-gap block must contain >= 1 FAILING statement (it documents
  invented syntax for a grammar gap). If all its statements parse, the gap has
  closed: flip the tag to ```glossql and record the decision as a SPEC diff.

Exit 0 = invariant holds. Exit 1 = regressions or closed gaps need attention.
Usage: python3 harness/check.py [-v]
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from glossql_parser import check_source  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
FENCE_RE = re.compile(r"^```([\w-]*)\n(.*?)^```", re.MULTILINE | re.DOTALL)

verbose = "-v" in sys.argv
failures = 0
gap_closed = 0
stats = {"blocks": 0, "stmts": 0}


def report(path: Path, blockno: int, tag: str, results, expect_gap: bool) -> None:
    global failures, gap_closed
    stats["blocks"] += 1
    stats["stmts"] += len(results)
    errs = [(p, e) for p, e in results if e is not None]
    if expect_gap:
        if not errs:
            gap_closed += 1
            print(f"GAP CLOSED? {path.name} block {blockno}: every statement now "
                  f"parses — flip the tag and fold the decision into SPEC.md")
        elif verbose:
            for p, e in errs:
                print(f"  gap held   {path.name} b{blockno}: {p!r}: {e}")
    else:
        for p, e in errs:
            failures += 1
            print(f"FAIL {path.name} block {blockno} [{tag}]\n  stmt: {p}\n  err:  {e}")


def check_file(path: Path, take_tags: dict[str, bool]) -> None:
    text = path.read_text()
    for blockno, m in enumerate(FENCE_RE.finditer(text), 1):
        tag, body = m.group(1), m.group(2)
        if tag not in take_tags:
            continue
        report(path, blockno, tag, check_source(body), expect_gap=take_tags[tag])


check_file(ROOT / "SPEC.md", {"sql": False})
for f in sorted((ROOT / "corpus").glob("*.md")):
    check_file(f, {"glossql": False, "glossql-gap": True})

# --- §10 walkthrough, executed: replay + pooling + contested (SPEC §5) ------
from glossql_parser import split_statements, tokenize  # noqa: E402
from replay import replay  # noqa: E402

w10 = (ROOT / "SPEC.md").read_text().split("\n## 10.")[1]
source = "\n".join(m.group(2) for m in FENCE_RE.finditer(w10) if m.group(1) == "sql")
slot = ("orders.amount", "behavior", ())
full = replay(source)
stmts = split_statements(tokenize(source))
i_teach = next(i for i, s in enumerate(stmts)
               if [t.text for t in s[:2]] == ["DECLARE", "behavior"]
               and any(t.text.upper() == "USER" for t in s))
pre = replay(source, upto=i_teach)
again = replay(source)
walkthrough = [
    ("station 4 declaration occupies the slot", pre.slots.get(slot, {}).get("value") == "stock"),
    ("witness pools under the declared reliability", pre.posterior(slot) is not None),
    ("station 5: contested before the teach", pre.contested(slot) is True),
    ("station 6: teach supersedes to flow", full.slots.get(slot, {}).get("value") == "flow"),
    ("station 6: contested clears", full.contested(slot) is False),
    ("replay determinism (identical derived state)",
     (again.slots, again.witnesses, again.reliabilities)
     == (full.slots, full.witnesses, full.reliabilities)),
]
for name, ok in walkthrough:
    if not ok:
        failures += 1
    print(f"{'ok  ' if ok else 'FAIL'} walkthrough: {name}")

print(f"\n{stats['blocks']} blocks, {stats['stmts']} statements checked; "
      f"{len(walkthrough)} walkthrough assertions; "
      f"{failures} failures, {gap_closed} gaps closed")
sys.exit(1 if (failures or gap_closed) else 0)
