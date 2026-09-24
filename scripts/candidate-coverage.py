#!/usr/bin/env python3
"""Measure lexical pair selection before allowing it to replace a full sweep.

This measures a *chunk-anchored upper bound* on reachability, not whether a
candidate states the anchor or a model would recognize a tension. Only pairs
with both source sections represented can be scored; missing sides are reported.
The Maple House manifest is exhaustive and its test split is never used to
choose parameters. Run with an existing, pinned draft run:

    python3 scripts/candidate-coverage.py fixtures/maple-house/runs/demo-tape/run.json
"""

import argparse
import json
import math
import re
from collections import Counter
from pathlib import Path


def tokens(text):
    return set(re.findall(r"[a-z0-9]+", text.lower()))


def neighbours(texts, k):
    """Undirected union of each rule's k strongest lexical neighbours."""
    words = [tokens(text) for text in texts]
    frequency = Counter(word for item in words for word in item)
    weights = {w: math.log((len(words) + 1) / (n + 1)) + 1 for w, n in frequency.items()}
    pairs = set()
    scores = [[] for _ in words]
    for i, left in enumerate(words):
        for j in range(i + 1, len(words)):
            right = words[j]
            shared = sum(weights[w] for w in left & right)
            total = sum(weights[w] for w in left | right)
            score = shared / total if total else 0
            scores[i].append((score, j))
            scores[j].append((score, i))
    for i in range(len(words)):
        for _, j in sorted(scores[i], key=lambda item: (-item[0], item[1]))[:k]:
            pairs.add(tuple(sorted((i, j))))
    return pairs


def anchor(chunk):
    heading = chunk.get("heading", "")
    article = re.search(r"Article ([IVX]+)(?:\b|\s)", heading)
    if article:
        return ("article", article.group(1))
    date = re.search(r"\b20\d\d-\d\d-\d\d\b", heading)
    if date:
        return ("date", date.group())
    return None


def coverage(rows, by_anchor, pairs):
    reached = []
    missing = []
    for row in rows:
        a = by_anchor.get(next(iter(row["a"].items())), [])
        b = by_anchor.get(next(iter(row["b"].items())), [])
        if not a or not b:
            missing.append(row["id"])
        elif any(tuple(sorted((i, j))) in pairs for i in a for j in b if i != j):
            reached.append(row["id"])
    return reached, missing


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run", type=Path)
    parser.add_argument("--truth", type=Path, default=Path("fixtures/maple-house/truth.json"))
    parser.add_argument("--k", type=int, default=15)
    args = parser.parse_args()
    if args.k < 1:
        parser.error("--k must be positive")
    run = json.loads(args.run.read_text())
    truth = json.loads(args.truth.read_text())
    if truth.get("corpus_id") != "maple-house":
        parser.error("only Maple House's labeled anchors are implemented")

    candidates = run["candidates"]
    kept = [candidates[i] for i in run["kept"] if candidates[i].get("kind", "rule") == "rule"]
    if not kept:
        parser.error("run contains no kept rules; use a completed draft run")
    chunks = {c["id"]: c for c in run["chunks"]}
    by_anchor = {}
    for i, candidate in enumerate(kept):
        key = anchor(chunks[candidate["chunk"]])
        if key:
            by_anchor.setdefault(key, []).append(i)

    selected = neighbours([c["text"] for c in kept], args.k)
    total = len(kept) * (len(kept) - 1) // 2
    print(f"run={args.run} k={args.k} kept_rules={len(kept)}")
    print(f"pairs={len(selected)}/{total} ({len(selected) / total:.1%} of full sweep)" if total else "pairs=0/0")
    any_reachable = False
    for label, rows in (("tensions", truth["planted_tensions"]),
                        ("compatible", truth["expected_non_tensions"])):
        # Train/dev only for development. Test stays sealed until a selector is frozen.
        rows = [r for r in rows if r["split"] != "test"]
        reached, missing = coverage(rows, by_anchor, selected)
        reachable = len(rows) - len(missing)
        any_reachable |= reachable > 0
        print(f"{label}: offered {len(reached)}/{reachable} reachable train/dev pairs; "
              f"not extracted={missing}; missed={[r['id'] for r in rows if r['id'] not in reached and r['id'] not in missing]}")
    if not any_reachable:
        parser.error("no labeled pairs are represented in this run; cannot measure coverage")
    print("Chunk-anchored reachability upper bound only; model recall and false positives are unmeasured.")


if __name__ == "__main__":
    main()
