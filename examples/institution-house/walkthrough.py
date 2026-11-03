#!/usr/bin/env python3
"""Run the user guide's six read-only views through the actual institution reader."""
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
BIN = Path(os.environ.get("CANON_INSTITUTION_BIN", ROOT / "target/debug/canon-institution"))
LOG = HERE / "record/acts.jsonl"
START = HERE / "starting-agreement.json"


def main():
    if not BIN.is_file():
        raise SystemExit("Build and check the preview from canon's root first: cargo test -p canon-institution")
    inputs = {p: hashlib.sha256(p.read_bytes()).hexdigest() for p in (LOG, START)}
    acts = [json.loads(line) for line in LOG.read_text().splitlines() if line.strip()]
    rule = next(a["id"] for a in acts if a["op"] == "assert")
    removal = next(a["id"] for a in acts if a["op"] == "retract")
    stages = [
        (3, "proposed", None, "Helper proposes quiet hours."),
        (4, "proposed", None, "Mira agrees; Sam has not agreed yet."),
        (5, "active", None, "Sam agrees; the rule takes effect."),
        (6, "active", "proposed", "Helper asks to remove it; the rule stays."),
        (7, "active", "proposed", "Mira agrees to removal; it still needs Sam."),
        (8, "retracted", "admitted", "Sam agrees to removal; the rule ends."),
    ]
    for day, expected, removal_status, story in stages:
        at = int(datetime(2020, 1, day, tzinfo=timezone.utc).timestamp())
        ran = subprocess.run([str(BIN), "--arrangements", str(START), "--log", str(LOG), "--at", str(at)],
                             capture_output=True, text=True, timeout=30)
        if ran.returncode:
            raise SystemExit(ran.stderr.strip() or f"reader exited {ran.returncode}")
        result = json.loads(ran.stdout)
        assert result["interpretation"] == "shared-assent-v2"
        entries = result["state"]["commitments"] + result["pending"]
        observed = next(e["status"]["status"] for e in entries if e["id"] == rule)
        assert observed == expected, (day, observed, expected)
        decision = next((d for d in result["admissions"] if d["source"] == removal), None)
        assert (decision["status"] if decision else None) == removal_status
        if day == 5:
            assert next(d["effective_at"] for d in result["admissions"] if d["source"] == rule) == at
        if day == 8:
            assert decision["effective_at"] == at
        print(f"2020-01-{day:02d}  {observed:9}  {story}")
    for path, before in inputs.items():
        assert hashlib.sha256(path.read_bytes()).hexdigest() == before, f"reader changed {path}"
    print("Six views checked. The example record and starting agreement are unchanged.")


if __name__ == "__main__":
    main()
