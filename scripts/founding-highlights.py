#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Pull a few of the founding run's own proposals out for the stage.

283 tensions is not a demo. This prints four of them, by candidate index, from
the artifact of the completed sweep — the model's real words, not a summary.

**The fourth one is wrong on purpose.** A demo that only shows the hits is
selling; the honest beat is the good one, then the bad one, then the reason
review is one at a time and there is no `--accept-all`.
"""
import json, sys, glob

RUN = sys.argv[1] if len(sys.argv) > 1 else \
    "fixtures/founding/runs/pod-27b/sweep/run-1788143950.json"

SHOW = [
    ((1, 89),    "The one everybody feels"),
    ((3, 4),     "The Declaration, arguing with itself"),
    ((24, 94),   "The Articles against the Constitution that replaced them"),
    ((288, 161), "AND ONE IT GOT WRONG — read the reason against the two rules"),
]

d = json.load(open(RUN))
c = {i: x["text"] for i, x in enumerate(d["candidates"])}
by = {(t.get("a"), t.get("b")): t for t in d["tensions"]}

print(f"\n{len(d['candidates'])} rules read from the founding documents. "
      f"{len(d['tensions'])} contradictions proposed. Four of them:\n")
for (a, b), label in SHOW:
    t = by.get((a, b)) or by.get((b, a))
    if not t:
        continue
    print(f"\033[1m{label}\033[0m")
    print(f"    {c[a][:150]}")
    print(f"    {c[b][:150]}")
    print(f"  \033[2mwhy: {(t.get('reason') or '')[:230]}\033[0m\n")
