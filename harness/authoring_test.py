#!/usr/bin/env python3
"""§9.1 harness: the constrained-decoding authoring rig.

§2.5 claims statement shapes are regular enough for constrained decoding — that
agents, not humans, are the primary authors. This rig is the verdict half of
that test: point any LLM at a real cataloguing task plus grammar.ebnf, save its
output to a file, and run

    python3 harness/authoring_test.py <statements-file>

Per-statement verdicts: parses / parse error. Semantic admission (undeclared
aspects, unresolved names) is the engine's job, not this rig's — an authoring
failure at the *syntax* level is a grammar bug, not an agent bug.

The generation half is deliberately external: the harness never calls a model.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from glossql_parser import check_source  # noqa: E402

if len(sys.argv) != 2:
    print(__doc__)
    sys.exit(2)

results = check_source(Path(sys.argv[1]).read_text())
bad = [(p, e) for p, e in results if e]
for preview, err in results:
    print(f"{'FAIL' if err else 'ok  '} {preview}" + (f"\n     {err}" if err else ""))
print(f"\n{len(results)} statements, {len(bad)} parse failures")
sys.exit(1 if bad else 0)
