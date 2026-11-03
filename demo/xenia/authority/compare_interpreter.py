#!/usr/bin/env python3
"""Reinterpret the ten retained histories; expected outcomes stay outside the interpreter."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

import authority as a


def compare(binary, freeze=None, temporal=False):
    baseline = json.loads((a.HERE / "results/report.json").read_text())
    results, histories = [], []
    arrangements = json.loads((a.REPO / "crates/canon-institution/tests/fixtures/arrangements.json").read_text()) if temporal else []
    with tempfile.TemporaryDirectory(prefix="canon-institution-compare-") as tmp:
        roots = {e: a.prepare(e, Path(tmp) / e) for e in ("adoption", "courtyard")}
        for old in baseline["results"]:
            work = roots[old["experiment"]] / old["case"]
            meta = json.loads((work / "case.json").read_text())
            basis = (work / ".canon/acts.jsonl").read_text()
            source = basis + old["appended"]
            if hashlib.sha256(basis.encode()).hexdigest() != old["before_sha256"] or hashlib.sha256(source.encode()).hexdigest() != old["after_sha256"]:
                raise ValueError(f"{old['case']}: the retained raw history was not reconstructed exactly")
            histories.append({"experiment": old["experiment"], "case": old["case"], "basis": basis,
                              "appended": old["appended"], "before_sha256": old["before_sha256"],
                              "after_sha256": old["after_sha256"], "target": meta["target"],
                              "original": meta["original"], "native_observed": old["observed"]})
            before_file, source_file = work / "basis.jsonl", work / "source.jsonl"
            before_file.write_text(basis)
            source_file.write_text(source)
            at = max(json.loads(l)["ts_unix"] for l in source.splitlines())
            mode, initial = "--basis", before_file
            if temporal:
                manifest = next(r["arrangements"] for r in arrangements if r["experiment"] == old["experiment"] and r["case"] == old["case"])
                initial = work / "arrangements.json"
                initial.write_text(json.dumps(manifest))
                mode = "--arrangements"
            run = subprocess.run([str(binary), mode, str(initial), "--log", str(source_file), "--at", str(at)],
                                 capture_output=True, text=True, timeout=30)
            if source_file.read_text() != source or before_file.read_text() != basis:
                raise ValueError("interpreter rewrote an input record")
            if run.returncode:
                result = {"judgment": "could-not-judge", "reason": run.stderr.strip()}
            else:
                interpreted = json.loads(run.stdout)
                state = interpreted["state"]
                if temporal:
                    bindings = dict(state["scopes"])
                    for c in state["commitments"]:
                        c["scope"] = bindings.get(c["id"])
                    state["commitments"].extend(interpreted["pending"])
                result = {**a.effect({"at": at, "state": state}, meta),
                          "interpretation": interpreted["interpretation"], "admissions": interpreted["admissions"],
                          "basis_sha256": interpreted.get("basis_sha256", interpreted.get("arrangements_sha256")), "source_sha256": interpreted["source_sha256"]}
            results.append({"experiment": old["experiment"], "case": old["case"], "native_judgment": old["judgment"], **result})
            print(f"{result['judgment']:15} {old['experiment']}/{old['case']}: native {old['judgment']}")
    if freeze:
        freeze.write_text(json.dumps(histories, indent=2) + "\n")
    return {"interpretation": "shared-assent-v2" if temporal else "shared-assent-v1", "boundary": "explicit arrangements, temporal admission" if temporal else "fixed-basis admission round",
            "interpreter_binary_sha256": hashlib.sha256(Path(binary).read_bytes()).hexdigest(),
            "baseline_report_sha256": hashlib.sha256((a.HERE / "results/report.json").read_bytes()).hexdigest(),
            "summary": {"holds": sum(r["judgment"] == "holds" for r in results),
                        "fails": sum(r["judgment"] == "fails" for r in results),
                        "could_not_judge": sum(r["judgment"] == "could-not-judge" for r in results),
                        "original_controls": sum(r["native_judgment"] == "holds" for r in results),
                        "preserved_controls": sum(r["native_judgment"] == "holds" and r["judgment"] == "holds" for r in results)},
            "results": results}


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--bin", default=a.REPO / "target/debug/canon-institution", type=Path)
    parser.add_argument("--freeze", type=Path, help="export the discovered raw histories for product-owned tests")
    parser.add_argument("--temporal", action="store_true", help="replay the explicit, frozen initial arrangements through v2")
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    report = compare(args.bin.resolve(), args.freeze, args.temporal)
    args.out.write_text(json.dumps(report, indent=2) + "\n")
    raise SystemExit(int(any(r["judgment"] != "holds" for r in report["results"])))
