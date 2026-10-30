#!/usr/bin/env python3
"""canon stays a lean library: each crate's product code stays under its cap.

Product code is a line of a crate's src/ that is not blank, not a comment and
not a test: test files (tests.rs, *_tests.rs, testing.rs, a tests/ directory)
and inline `#[cfg(test)] mod` blocks are not counted. A cap is raised only by
the operator, after an economizing pass, and the commit that raises it names
the cuts and what the lines bought; git keeps that history, not this file.
Exit 0 within every cap, 1 over one. `--root <dir>` checks another tree.
"""
import re
import sys
from pathlib import Path

CAPS = {"canon-core": 4817, "canon-cli": 15297, "canon-contract": 304}

TEST_FILE = re.compile(r"(^|/)tests?(_\w+)?\.rs$|_tests?\.rs$|(^|/)testing\.rs$|(^|/)tests/")
TEST_MOD = re.compile(r"^\s*#\[cfg\(test\)\]\s*$")


def product_lines(text):
    """Code lines, with inline test modules cut by brace depth."""
    n, lines, i = 0, text.splitlines(), 0
    while i < len(lines):
        if TEST_MOD.match(lines[i]) and i + 1 < len(lines) and re.match(r"^\s*(pub\s+)?mod\s+\w+\s*\{", lines[i + 1]):
            depth, i = 0, i + 1
            while i < len(lines):
                depth += lines[i].count("{") - lines[i].count("}")
                i += 1
                if depth <= 0:
                    break
            continue
        s = lines[i].strip()
        if s and not s.startswith("//"):
            n += 1
        i += 1
    return n


def count(root, crate):
    src = root / "crates" / crate / "src"
    return sum(product_lines(f.read_text()) for f in src.rglob("*.rs") if not TEST_FILE.search(str(f.relative_to(src))))


def main():
    root = Path(sys.argv[sys.argv.index("--root") + 1]) if "--root" in sys.argv else Path(__file__).resolve().parent.parent
    over = 0
    for crate, cap in CAPS.items():
        n = count(root, crate)
        print(f"{crate:16} {n:6} of {cap}")
        if n > cap:
            over += 1
            print(f"  over by {n - cap}: cut first; a raise is the operator's, after an economizing pass")
    return 1 if over else 0


if __name__ == "__main__":
    sys.exit(main())
