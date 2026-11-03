#!/usr/bin/env python3
"""Retain the two rehearsal accounts and independently recheck their state effects."""
import argparse
import hashlib
import json
from pathlib import Path

from authority import CASES, effect


def collect(root):
    root = Path(root).resolve()
    summary = json.loads((root / "summary.json").read_text())
    people = []
    for experiment, name in (("adoption", "Helper"), ("courtyard", "Mira")):
        account = json.loads((root / "run-1" / experiment / experiment / f"{name}.json").read_text())
        attached = root / "run-1" / experiment / "attachments" / name
        hashes = {Path(a["path"]).name: a["sha256"] for a in account["attachments"]}
        for filename in ("manifest.json", "observations.jsonl"):
            if hashlib.sha256((attached / filename).read_bytes()).hexdigest() != hashes[filename]:
                raise ValueError(f"{experiment}: attachment {filename} changed")
        manifest = json.loads((attached / "manifest.json").read_text())
        if manifest["declaration"] != CASES[experiment]:
            raise ValueError(f"{experiment}: the charter changed since this rehearsal")
        records = [json.loads(l) for l in (attached / "observations.jsonl").read_text().splitlines() if l.strip()]
        checks = []
        for r in records:
            if "effect" not in r:
                continue
            recomputed = effect(r["after"], r["case"])
            if recomputed != r["effect"]:
                raise ValueError(f"{r['id']}: the recorded effect does not match its state")
            checks.append({"case": r["case"]["id"], "argv": r["argv"], **recomputed,
                           "before_sha256": r["before"]["sha256"], "after_sha256": r["after"]["sha256"]})
        attempted = {c["case"] for c in checks}
        people.append({"scene": experiment, "as": name, "simulated": account["simulated"],
                       "verdict": account["verdict"], "marks": account["marks"], "refused": account["refused"],
                       "feedback": account["soft"], "observations": len(records), "effect_checks": checks,
                       "not_attempted": [c["id"] for c in CASES[experiment]["cases"] if c["id"] not in attempted],
                       "attachments_sha256": hashes, "binary_sha256": manifest["binary_sha256"]})
    return {"run": str(root), "n": summary["n"], "model": summary["model"], "route": summary["route"],
            "instrument_verdict": summary["verdict"], "plants": summary["plants"],
            "challenger_summary": summary["challengers"], "total_cost_usd": summary["total_cost_usd"],
            "usage": summary["usage"], "people": people}


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("run")
    parser.add_argument("--out", required=True)
    args = parser.parse_args()
    report = collect(args.run)
    Path(args.out).write_text(json.dumps(report, indent=2) + "\n")
    for person in report["people"]:
        print(f"{person['scene']}: {len(person['marks'])} marks, {len(person['effect_checks'])} state checks, "
              f"not attempted: {person['not_attempted']}")
