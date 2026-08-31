#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Ostrom reachability — leg 2 of the CPR study.

Leg 1 shows the eight design principles compose out of the primitives for any
common-pool resource. That is settled and needs no model. This asks the harder
half: **point canon at a real community's real documents, cold — is the
material for the eight principles in there, and does extraction reach it?**

Reachability is a CEILING, not a score. It says the passage a principle turns
on survived into the candidate set, so a community reviewing that set would
have something to build the principle from. It does not say the proposal was
well worded, and it does not say the community would keep it.

Scores the SAME run artifacts `draft-bar.sh` already produces, so a sweep paid
for once answers both questions.

    scripts/ostrom-reach.py maple-house
    scripts/ostrom-reach.py des-moines-noise fixtures/des-moines-noise/runs/qwen-27b
"""

from __future__ import annotations

import argparse
import json
import re
import statistics
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MARK = {"present": "present", "partial": "partial", "absent": "absent "}


def norm(s: str) -> str:
    return re.sub(r"\s+", " ", s).strip().lower()


def hit(anchor: dict, candidates: list[dict], chunks: dict[int, str], field: str) -> bool:
    """Did any candidate from this anchor's section carry one of its phrases?

    `field` is the whole measurement. `quote` is cut out of the document by
    canon itself, so a match there says the passage SURVIVED into the
    candidate set and nothing more — that is the ceiling. `text` is the rule
    the model proposed in its own words, and a match there says the principle
    is in the proposal a person would actually review. Reporting only the
    first number would be reporting that chunking works.
    """
    head = norm(anchor["heading"])
    for c in candidates:
        if head not in norm(chunks.get(c.get("chunk", -1), "")):
            continue
        body = norm(c.get("text", "") if field == "text"
                    else f"{c.get('text', '')} {c.get('quote', '')}")
        for alternatives in anchor["must"]:
            if any(norm(a) in body for a in alternatives):
                return True
    return False


def score_run(path: Path, manifest: dict) -> dict:
    run = json.loads(path.read_text())
    chunks = {c["id"]: c.get("heading", "") for c in run["chunks"]}
    cands = run["candidates"]
    out = {}
    for num, p in manifest["principles"].items():
        anchors = p["anchors"]
        out[num] = {"anchored": len(anchors)}
        for field, tag in (("both", "ceiling"), ("text", "own")):
            hits = [hit(a, cands, chunks, field) for a in anchors]
            out[num][f"hit_{tag}"] = sum(hits)
            out[num][f"reached_{tag}"] = any(hits)
    covered = {c.get("chunk") for c in cands}
    out["_meta"] = {"model": run.get("model"), "endpoint": run.get("endpoint"),
                    "candidates": len(cands), "chunks": len(run["chunks"]),
                    "covered": len(covered)}
    return out


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("corpus")
    ap.add_argument("runs", nargs="?")
    a = ap.parse_args()

    d = ROOT / "fixtures" / a.corpus
    mf = d / "ostrom-anchors.json"
    if not mf.exists():
        sys.exit(f"no ostrom-anchors.json for `{a.corpus}`")
    manifest = json.loads(mf.read_text())

    if a.runs:
        runs_dir = Path(a.runs)
    else:
        found = sorted(p for p in (d / "runs").glob("*") if p.is_dir())
        if len(found) != 1:
            sys.exit(f"{len(found)} run directories under {d/'runs'} — name one:\n  "
                     + "\n  ".join(str(f) for f in found))
        runs_dir = found[0]
    files = sorted(runs_dir.glob("run-*.json"))
    if len(files) < 3:
        # One run is an anecdote. The spread between runs over the same
        # document is the noise floor any published number has to clear.
        sys.exit(f"{len(files)} run(s) in {runs_dir}; the bar refuses fewer than 3")

    scored = [score_run(f, manifest) for f in files]
    meta = scored[0]["_meta"]

    print(f"corpus     {a.corpus}  ({manifest['kind']})")
    print(f"runs       {len(files)} from {runs_dir}")
    print(f"model      {meta['model']}")
    print(f"endpoint   {meta['endpoint']}")
    print(f"candidates {', '.join(str(s['_meta']['candidates']) for s in scored)}")
    cov = meta["covered"] / meta["chunks"]
    print(f"chunks     {meta['covered']}/{meta['chunks']} yielded a candidate ({cov:.2f})")
    if cov > 0.95:
        print("           ^ at this coverage the CEILING column is near-guaranteed: canon cuts")
        print("             the quote out of your file, so an anchor phrase that is in the")
        print("             section is in the quote. Read the OWN WORDS column instead.")
    print()
    print(f"{'#':<2} {'principle':<38} {'in corpus':<9} {'ceiling':<8} {'own words':<10} anchors")
    counts = {"present": 0, "partial": 0, "absent": 0}
    rate = {"ceiling": [], "own": []}
    partial_reached = [0, 0]
    tot = {"ceiling": [0, 0], "own": [0, 0]}
    for num, p in manifest["principles"].items():
        st = p["status"]
        per = [s[num] for s in scored]
        n = per[0]["anchored"] * len(per)
        counts[st] += 1
        if st == "absent":
            print(f"{num:<2} {p['name']:<38} {MARK[st]:<9} {'—':<8} {'—':<10} "
                  f"nothing in the document to reach")
            continue
        shown = {}
        for tag in ("ceiling", "own"):
            r = statistics.mean(1.0 if x[f"reached_{tag}"] else 0.0 for x in per)
            # A `partial` principle is NOT scored with the rest. The manifest
            # says the document carries only part of what the principle needs,
            # so counting a hit there towards "reached N of 8" would report a
            # principle as found that the ground truth says is half absent.
            if st == "partial":
                if tag == "own":
                    partial_reached[0] += int(r == 1)
                    partial_reached[1] += 1
            else:
                rate[tag].append(r)
                tot[tag][0] += sum(x[f"hit_{tag}"] for x in per)
                tot[tag][1] += n
            shown[tag] = "yes" if r == 1 else ("no" if r == 0 else f"{r:.2f}")
        print(f"{num:<2} {p['name']:<38} {MARK[st]:<9} {shown['ceiling']:<8} "
              f"{shown['own']:<10} {sum(x['hit_own'] for x in per)}/{n} own")

    full = counts["present"]
    print()
    print(f"{'principles fully carried by this document':<41} {full} of 8")
    print(f"{'  partly carried':<41} {counts['partial']}")
    print(f"{'  not carried at all':<41} {counts['absent']}")
    print()
    for tag, label in (("ceiling", "reached, ceiling (text or quote)"),
                       ("own", "reached, in the proposal's own words")):
        m = statistics.mean(rate[tag]) if rate[tag] else 0.0
        print(f"{label:<41} {m * full:.2f} of {full}   ({m:.2f})")
        print(f"{'  anchor recall':<41} {tot[tag][0]}/{tot[tag][1]}"
              f"   ({tot[tag][0] / tot[tag][1]:.2f})")
    if partial_reached[1]:
        print(f"{'partly-carried principles reached':<41} "
              f"{partial_reached[0]} of {partial_reached[1]}   "
              f"(scored apart; see the manifest for what is missing)")
    print()
    print("Scored over the principles the document FULLY carries. A `partial`")
    print("principle is reported on its own line and never folded into that")
    print("number: the manifest says the material is incomplete, and a hit")
    print("there is a hit on the part that is present.")
    print()
    print("Neither column says the community would keep the proposal. Reviewing")
    print("one at a time is the answer to that, and it is not automatable.")


if __name__ == "__main__":
    main()
