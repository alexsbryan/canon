#!/usr/bin/env python3
"""Measured inventory for the mechanism/institution split, using the lean gate's counter."""
import argparse
import json
from pathlib import Path

from lean_check import TEST_FILE, product_lines

ROOT = Path(__file__).resolve().parent.parent
PATHS = [
    "crates/canon-core/src",
    "crates/canon-cli/src",
    "demo/xenia/authority/interpreter/src",
    "crates/canon-institution/src",
]


def inventory():
    rows = []
    for path in PATHS:
        root = ROOT / path
        if not root.exists():
            continue
        files, tests = [], []
        for p in sorted(root.rglob("*.rs")):
            text = p.read_text()
            if TEST_FILE.search(str(p.relative_to(root))):
                tests.append({"path": str(p.relative_to(ROOT)), "code": product_lines(text, include_tests=True)})
                continue
            product = product_lines(text)
            inline_tests = product_lines(text, include_tests=True) - product
            if inline_tests:
                tests.append({"path": str(p.relative_to(ROOT)), "code": inline_tests})
            files.append({"path": str(p.relative_to(ROOT)), "product": product,
                          "physical": len(text.splitlines())})
        if not files:
            continue
        for p in sorted((root.parent / "tests").rglob("*.rs")):
            tests.append({"path": str(p.relative_to(ROOT)), "code": product_lines(p.read_text(), include_tests=True)})
        rows.append({"path": path, "product": sum(f["product"] for f in files), "files": files,
                     "test_code": sum(t["code"] for t in tests), "tests": tests})
    return rows


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", type=Path)
    args = parser.parse_args()
    rows = inventory()
    if args.out:
        args.out.write_text(json.dumps(rows, indent=2) + "\n")
    for row in rows:
        print(f"{row['path']}: {row['product']} product lines")
        if "canon-core" in row["path"] or "interpreter" in row["path"] or "canon-institution" in row["path"]:
            for f in row["files"]:
                print(f"  {f['product']:5} {f['path']}")
            for t in row["tests"]:
                if t["path"].endswith("effects.rs") or "canon-institution" in t["path"]:
                    print(f"  test {t['code']:5} {t['path']}")
