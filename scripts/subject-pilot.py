#!/usr/bin/env python3
"""Select reproducible short notes for the entry-12 extraction trial.

The selection is independent of whether a rule reads well: fewest recorded
chunks, then source name, among notes with an extracted rule; eight
invariants, six attempts, six migrated memories. Print the baseline sentences
for human scoring before changing the prompt. Without --draft this makes no
model call or canon write; --draft writes an ignored extraction checkpoint.

    python3 scripts/subject-pilot.py <draft-run.json> <source-root>
"""

import argparse
import json
import os
import subprocess
import time
from collections import Counter, defaultdict
from pathlib import Path


def select(run, root):
    chunks = run["chunks"]
    by_source = defaultdict(list)
    for chunk in chunks:
        name = chunk["source"].rsplit(":", 1)[0]
        by_source[name].append(chunk["id"])
    by_chunk = defaultdict(list)
    for candidate in run["candidates"]:
        if candidate.get("kind", "rule") == "rule":
            by_chunk[candidate["chunk"]].append(candidate["text"])

    selected = []
    for kind, target in (("invariant", 8), ("attempt", 6), ("memory", 6)):
        eligible = sorted(
            (len(ids), name) for name, ids in by_source.items()
            if name.startswith(kind + "/")
            and any(by_chunk[i] for i in ids) and (root / name).is_file()
        )
        if len(eligible) < target:
            raise ValueError(f"{kind}: only {len(eligible)} sources with rules (need {target})")
        for _, name in eligible[:target]:
            selected.append((name, [text for i in by_source[name] for text in by_chunk[i]]))
    return selected


def compare(before_path, after_path):
    before = json.loads(before_path.read_text())
    after = json.loads(after_path.read_text())
    if [(c["source"], c["text"]) for c in before["chunks"]] != [
        (c["source"], c["text"]) for c in after["chunks"]
    ]:
        raise ValueError("chunks differ; this is not a paired prompt trial")
    old, new = before["candidates"], after["candidates"]
    print(f"before={before_path} after={after_path}")
    print(f"same inputs: {len(before['chunks'])} chunks; model {before['model']} → {after['served_model']}")
    print(f"candidates: {len(old)} → {len(new)}, citation drops: {len(before['dropped'])} → {len(after['dropped'])}")
    unchanged = sum((a["chunk"], a["text"]) in {(b["chunk"], b["text"]) for b in new} for a in old)
    print(f"unchanged texts at same chunk: {unchanged}/{len(old)}")
    for chunk in before["chunks"]:
        i = chunk["id"]
        prior = [c["text"] for c in old if c["chunk"] == i and c.get("kind", "rule") == "rule"]
        latest = [c["text"] for c in new if c["chunk"] == i and c.get("kind", "rule") == "rule"]
        if prior != latest:
            print(f"\n{chunk['source']}")
            for text in prior:
                print(f"  before: {text}")
            for text in latest:
                print(f"  after:  {text}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run", type=Path)
    parser.add_argument("source_root", type=Path)
    parser.add_argument("--draft", action="store_true", help="run a dry draft, stopping at its extract checkpoint")
    parser.add_argument("--compare", type=Path, help="compare this checkpoint against the named after-checkpoint")
    args = parser.parse_args()
    if args.compare:
        compare(args.run, args.compare)
        return
    run = json.loads(args.run.read_text())
    chosen = select(run, args.source_root)
    chunks = run["chunks"]
    for name, _ in chosen:
        text = (args.source_root / name).read_text()
        if not all(c["text"] in text for c in chunks if c["source"].rsplit(":", 1)[0] == name):
            parser.error(f"{name}: source differs from the recorded baseline; cannot compare")
    if args.draft:
        canon_dir = args.source_root.parent.parent.resolve()
        if canon_dir.name != ".canon" or not (canon_dir / "draft-runs").is_dir():
            parser.error("source root must be <canon>/.canon/sources/notes with draft-runs present")
        env = os.environ.copy()
        env["CANON_DIR"] = str(canon_dir)
        command = ["cargo", "run", "--quiet", "-p", "canon-cli", "--", "draft",
                   "--dry-run", "--yes", "--from",
                   *[str(args.source_root / name) for name, _ in chosen]]
        runs_dir = canon_dir / "draft-runs"
        before = set(runs_dir.glob("*.partial.json"))
        process = subprocess.Popen(command, env=env, cwd=Path(__file__).resolve().parent.parent)
        deadline = time.monotonic() + 720
        try:
            while time.monotonic() < deadline:
                for path in sorted(set(runs_dir.glob("*.partial.json")) - before):
                    try:
                        stage = json.loads(path.read_text()).get("checkpoint")
                    except (OSError, json.JSONDecodeError):
                        continue
                    if stage == "extract":
                        process.terminate()
                        process.wait(timeout=20)
                        print(f"Extraction-only checkpoint: {path}", flush=True)
                        return
                if process.poll() is not None:
                    raise RuntimeError(f"draft exited {process.returncode} before extraction checkpoint")
                time.sleep(1)
            raise TimeoutError("no extraction checkpoint after 12 minutes")
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=20)
        return
    print(f"model={run['model']} at={run['at']} 20 short notes; extraction baseline only")
    print("by class:", dict(Counter(name.split("/", 1)[0] for name, _ in chosen)))
    print("matched source paths:")
    for name, rules in chosen:
        print(f"  {args.source_root / name}")
        for text in rules:
            print(f"    - {text}")
    print("Score each rule for whether a reader without its passage knows the subject.")


if __name__ == "__main__":
    main()
