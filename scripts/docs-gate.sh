#!/usr/bin/env bash
# docs-gate.sh — every repository path a narrative document links to must
# resolve.
#
# This tree's documents cite each other and the code heavily: the README
# sends you to six other files, PROVENANCE files name fixture paths, and
# CONTRIBUTING points at scripts and tests. A link that rots is worse than
# no link, because it sends a new contributor looking for something that
# moved and leaves them assuming the rest of the map is wrong too.
#
# Scope, deliberately narrow — this is a gate, so a false positive costs
# more than a missed one:
#
#   * Markdown links only: `[text](target)`. Bare backticked paths in prose
#     are NOT checked; too many of them are illustrative (`~/house-docs`).
#   * Fenced code blocks are skipped. A link inside ``` is example output.
#   * http(s), mailto and pure `#anchor` targets are skipped. Anchor
#     fragments are stripped from local paths and not verified — a heading
#     that moves is a smaller lie than a file that does.
#   * Only files git tracks are read, so a scratch note never fails a build.
#
# Runs in about a second. `./scripts/docs-gate.sh` on its own, and from
# scripts/pre-push.sh and CI.
set -euo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

python3 - "$@" <<'PY'
import re, subprocess, sys, urllib.parse
from pathlib import Path

LINK = re.compile(r'\[[^\]]*\]\(\s*<?([^)\s>]+)>?(?:\s+"[^"]*")?\s*\)')
FENCE = re.compile(r'^\s*(```|~~~)')

files = subprocess.run(
    ["git", "ls-files", "*.md", "**/*.md"],
    capture_output=True, text=True, check=True,
).stdout.split()

broken, checked = [], 0
for f in sorted(set(files)):
    p = Path(f)
    try:
        lines = p.read_text(encoding="utf-8").splitlines()
    except (OSError, UnicodeDecodeError):
        continue
    fenced = False
    for n, line in enumerate(lines, 1):
        if FENCE.match(line):
            fenced = not fenced
            continue
        if fenced:
            continue
        for target in LINK.findall(line):
            if target.startswith(("http://", "https://", "mailto:", "#")):
                continue
            path = urllib.parse.unquote(target.split("#", 1)[0])
            if not path:
                continue
            checked += 1
            if not (p.parent / path).exists():
                broken.append((f, n, target))

if broken:
    print(f"docs-gate: {len(broken)} link(s) point at nothing\n", file=sys.stderr)
    for f, n, target in broken:
        print(f"  {f}:{n}  ->  {target}", file=sys.stderr)
    print(
        "\nFix the path, or drop the link. If the file moved on purpose, the\n"
        "document that pointed at it is part of the move.",
        file=sys.stderr,
    )
    sys.exit(1)

print(f"docs-gate: {checked} local link(s) across {len(files)} document(s), all resolve")
PY
