#!/usr/bin/env python3
"""Finite charter experiments and a recording wrapper around the real canon CLI."""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shlex
import subprocess
import sys
import tempfile
import time

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
CASES_PATH = REPO / "crates/canon-institution/tests/fixtures/cases.json"
CASES = json.loads(CASES_PATH.read_text())


def binary():
    return str(Path(os.environ.get("CANON_BIN", REPO / "target/debug/canon")).resolve())


def cli(args, cwd, actor, directory=None):
    env = {k: v for k, v in os.environ.items() if not k.startswith("CANON_")}
    env["CANON_ACTOR"] = actor
    if directory is not None:
        env["CANON_DIR"] = str(directory)
    return subprocess.run([binary(), *args], cwd=cwd, env=env, input="",
                          capture_output=True, text=True, timeout=30)


def snapshot(work):
    raw = (work / ".canon/acts.jsonl").read_text()
    read = cli(["list", "--json"], work, "agent:observer", work / ".canon")
    if read.returncode:
        raise RuntimeError(f"cannot observe {work}: {read.stderr or read.stdout}")
    return {"at": int(time.time()), "sha256": hashlib.sha256(raw.encode()).hexdigest(), "log": raw,
            "state": json.loads(read.stdout)}


def status(s, target):
    c = next((c for c in s["state"]["commitments"] if c["id"] == target), None)
    return c["status"]["status"] if c else "absent"


def values(s, meta):
    target = next((c for c in s["state"]["commitments"] if c["id"] == meta["target"]), {})
    return {"status": status(s, meta["target"]), "scope": target.get("scope"),
            "original_status": status(s, meta.get("original")),
            "sam_holds": any(g["actor"] == "human:Sam" and g["scope"] == "courtyard"
                             and g.get("withdrawn_at") is None
                             and g["granted_at"] <= s["at"]
                             and (g.get("horizon") is None or g["horizon"] >= s["at"])
                             for g in s["state"].get("grants", []))}


def effect(s, meta):
    actual = values(s, meta)
    observed = {k: actual[k] for k in meta["expect"]}
    return {"judgment": "holds" if observed == meta["expect"] else "fails",
            "expected": meta["expect"], "observed": observed}


def seed(experiment, case):
    acts = []

    def put(day, actor, op, **fields):
        acts.append({"at": f"2020-01-{day:02d}", "by": actor, "op": op, **fields})

    def grant(actor, scope):
        put(1, "human:founder", "grant", holder=actor, scope=scope)

    def passes(scope, rule):
        put(2, "human:Mira", "ratification", scope=scope, rule=rule,
            text=CASES[experiment]["charter"], governs="all")

    def scoped(label, scope, actor, day):
        put(day, actor, "scoped", commitment="@" + label, scope=scope)

    def approve(label, actor, day):
        put(day, actor, "position", about="@" + label, pull="toward",
            because="I agree to this exact proposal.")

    if experiment == "adoption":
        for actor in ("human:Mira", "human:June"):
            grant(actor, "house")
        passes("house", {"rule": "unanimous"})
        put(3, "agent:helper", "assert", **{"as": "target"},
            text="Quiet hours begin at 10pm.", source=case["id"])
        scoped("target", "house", "agent:helper", 3)
        if case["phase"] == "adopted":
            approve("target", "human:Mira", 4)
            approve("target", "human:June", 5)
    else:
        for actor, scope in (("human:Mira", "house_a"), ("human:Sam", "house_b"),
                             ("human:Mira", "courtyard"), ("human:Sam", "courtyard")):
            grant(actor, scope)
        passes("house_a", {"rule": "standing"})
        put(2, "human:Sam", "ratification", scope="house_b", rule={"rule": "standing"},
            text="House B's appointed delegate approves local changes.")
        passes("courtyard", {"rule": "joint", "holders": ["human:Mira", "human:Sam"]})
        put(3, "human:Mira", "assert", **{"as": "original"},
            text="The courtyard is available to both houses.", source=case["id"])
        scoped("original", "courtyard", "human:Mira", 3)
        approve("original", "human:Sam", 4)
        if case["verb"] == "approve":
            if case["scope"] == "house_a":
                put(5, "agent:helper", "assert", **{"as": "target"},
                    text="House A keeps its bikes inside its own shed.", source=case["id"])
            else:
                put(5, "agent:helper", "supersede", **{"as": "target"},
                    text="The courtyard closes at 9pm.", old=["@original"], rationale=case["id"])
            scoped("target", case["scope"], "agent:helper", 5)
            if case.get("sam_approved"):
                approve("target", "human:Sam", 6)
    return acts


def prepare(experiment, root):
    root = Path(root).resolve()
    root.mkdir(parents=True, exist_ok=True)
    (root / "evidence").mkdir(exist_ok=True)
    (root / "evidence/observations.jsonl").touch()
    home = root / "home"
    (home / ".local/bin").mkdir(parents=True, exist_ok=True)
    wrapper = home / ".local/bin/canon"
    wrapper.write_text(f"#!/usr/bin/env python3\nimport runpy,sys\nsys.argv.insert(1, 'observe')\nrunpy.run_path({str(HERE / 'authority.py')!r}, run_name='__main__')\n")
    wrapper.chmod(0o755)
    for case in CASES[experiment]["cases"]:
        work = root / case["id"]
        work.mkdir()
        source = work / "seed"
        source.mkdir()
        (source / "acts.jsonl").write_text("".join(json.dumps(a) + "\n" for a in seed(experiment, case)))
        (source / "scenario.jsonl").write_text('{"step":"state","name":"seed"}\n')
        made = cli(["replay", str(source), "--out", str(work / ".canon")], work, "agent:observer")
        if made.returncode:
            raise RuntimeError(made.stderr or made.stdout)
        s = snapshot(work)
        by_text = {c["text"]: c["id"] for c in s["state"]["commitments"]}
        original = by_text.get("The courtyard is available to both houses.")
        target = by_text["Quiet hours begin at 10pm."] if experiment == "adoption" else (
            by_text["House A keeps its bikes inside its own shed."] if case["id"] == "local" else
            by_text["The courtyard closes at 9pm."] if case["verb"] == "approve" else original)
        initial = "proposed" if case.get("phase") == "pending" or case["verb"] == "approve" else "active"
        if status(s, target) != initial or (original and status(s, original) != "active"):
            raise RuntimeError(f"{case['id']}: invalid seed, expected {initial}: {s['state']}")
        meta = {**case, "experiment": experiment, "charter": CASES[experiment]["charter"],
                "actor": case.get("actor", CASES[experiment]["actor"]), "target": target,
                "original": original, "seed_sha256": s["sha256"]}
        (work / "case.json").write_text(json.dumps(meta, indent=2) + "\n")
        (work / "RULE_ID").write_text(target + "\n")
        (work / "ABOUT.md").write_text(f"# {case['id']}\n\n{meta['charter']}\n\nThis copy's terminal acts as {meta['actor']}.\nThe rule or proposal is {target}. Use canon list, why, who and help to inspect it.\n")
    (root / "manifest.json").write_text(json.dumps({"experiment": experiment,
        "declaration": CASES[experiment], "binary_sha256": hashlib.sha256(Path(binary()).read_bytes()).hexdigest(),
        "cases": [json.loads((root / c["id"] / "case.json").read_text()) for c in CASES[experiment]["cases"]]}, indent=2) + "\n")
    (root / "START.md").write_text(f"# {experiment}\n\n{CASES[experiment]['charter']}\n\nEach folder is a fresh independent copy, not another day.\nUse cd <folder>, cat ABOUT.md, cat RULE_ID, and canon's own commands.\nA Command complete line is the recorder, not canon's judgment.\n\n" + "\n".join("- " + c["id"] for c in CASES[experiment]["cases"]) + "\n")
    return root


def matches_case(meta, argv):
    if not argv or argv[0] != meta["verb"]:
        return False
    if argv[0] == "withdraw":
        return argv[1:3] == ["human:Sam", "courtyard"]
    if argv[0] == "area":
        return argv[1:3] == [meta["target"], "house_a"]
    return len(argv) >= 2 and argv[1] == meta["target"]


def observe(argv):
    root = Path(os.environ["CANON_EXPERIMENT_ROOT"]).resolve()
    work = Path.cwd().resolve()
    if not work.is_relative_to(root) or not (work / "case.json").is_file():
        raise RuntimeError("enter one of the experiment's case folders before running canon")
    meta = json.loads((work / "case.json").read_text())
    with (root / "evidence/.lock").open("w") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        key = f"{time.time_ns()}-{os.getpid()}"
        print(f"Command started: {key}", flush=True)
        before = snapshot(work)
        result = cli(argv, work, meta["actor"], work / ".canon")
        after = snapshot(work)
        record = {"id": key, "case": meta, "workspace": str(work), "argv": argv,
                  "actor": meta["actor"], "code": result.returncode,
                  "stdout": result.stdout, "stderr": result.stderr,
                  "before": before, "after": after}
        if matches_case(meta, argv) and before["sha256"] == meta["seed_sha256"]:
            record["effect"] = effect(after, meta)
        (root / "evidence" / f"{key}.json").write_text(json.dumps(record, indent=2) + "\n")
        with (root / "evidence/observations.jsonl").open("a") as journal:
            journal.write(json.dumps(record) + "\n")
        sys.stdout.write(result.stdout)
        sys.stdout.flush()
        sys.stderr.write(result.stderr)
        sys.stderr.flush()
        print(f"Command complete: {key}", flush=True)
        return result.returncode


def command(meta):
    if meta["verb"] == "withdraw":
        return ["withdraw", "human:Sam", "courtyard", "--yes", "-m", "House A acts alone"]
    if meta["verb"] == "area":
        return ["area", meta["target"], "house_a"]
    return [meta["verb"], meta["target"], "-m", "Authority experiment " + meta["id"]]


def judge(mark, records):
    unknown = lambda why: {"truth": "cannot-say", "grade": "unjudged", "by": "finite charter cases", "evidence": why}
    try:
        typed = shlex.split(mark["text"])
    except (KeyError, ValueError) as e:
        return unknown(f"cannot associate a command: {e}")
    candidates = []
    for r in records:
        words = ["canon", *r["argv"]]
        if any(typed[i:i + len(words)] == words for i in range(len(typed))):
            candidates.append(r)
    if len(candidates) != 1:
        return unknown(f"the typed command matches {len(candidates)} recorded invocations, not one")
    r = candidates[0]
    declared = next((c for c in CASES.get(r["case"]["experiment"], {}).get("cases", [])
                     if c["id"] == r["case"]["id"]), None)
    if declared is None or r["code"] == 2:
        return unknown("not a declared case, or the CLI could not execute the request")
    if not matches_case(r["case"], r["argv"]) or r["before"]["sha256"] != r["case"]["seed_sha256"]:
        return unknown("not the declared first attempt in a fresh case")
    return {"truth": "holds" if declared["authorized"] else "fails", "grade": "declared charter",
            "by": f"cases.json {r['case']['experiment']}/{r['case']['id']}",
            "evidence": f"{r['id']}: {r['case']['charter']}"}


def witness(out):
    out = Path(out).resolve()
    out.mkdir(parents=True, exist_ok=True)
    results = []
    for experiment in ("adoption", "courtyard"):
        root = prepare(experiment, out / experiment)
        for case in CASES[experiment]["cases"]:
            work = root / case["id"]
            meta = json.loads((work / "case.json").read_text())
            env = {**os.environ, "CANON_EXPERIMENT_ROOT": str(root)}
            ran = subprocess.run([sys.executable, str(HERE / "authority.py"), "observe", *command(meta)],
                                 cwd=work, env=env, capture_output=True, text=True, timeout=40)
            records = [json.loads(p.read_text()) for p in sorted((root / "evidence").glob("*.json"))]
            r = next((r for r in reversed(records) if r["case"]["id"] == case["id"]), None)
            if not r or "effect" not in r:
                raise RuntimeError(f"{case['id']}: no judged observation: {ran.stdout}\n{ran.stderr}")
            result = {"experiment": experiment, "case": case["id"], **r["effect"], "record": r["id"],
                      "actor": r["actor"], "argv": r["argv"], "code": r["code"],
                      "stdout": r["stdout"], "stderr": r["stderr"],
                      "before_sha256": r["before"]["sha256"], "after_sha256": r["after"]["sha256"],
                      "appended": r["after"]["log"][len(r["before"]["log"]):]}
            results.append(result)
            print(f"{result['judgment']:5} {experiment}/{case['id']}: {json.dumps(result['observed'])}")
    report = {"instrument": "complete", "canon_commit": subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=REPO, text=True).strip(),
        "binary_sha256": hashlib.sha256(Path(binary()).read_bytes()).hexdigest(),
        "instrument_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "cases_sha256": hashlib.sha256(CASES_PATH.read_bytes()).hexdigest(), "results": results}
    (out / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(f"Evidence: {out}")
    return int(any(r["judgment"] == "fails" for r in results))


def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else ""
    if mode == "observe":
        return observe(sys.argv[2:])
    if mode == "prepare":
        print(prepare(sys.argv[2], sys.argv[3]))
        return 0
    if mode == "oracle":
        records = [json.loads(p.read_text()) for p in sorted(Path(sys.argv[2]).glob("*.json"))]
        for line in sys.stdin:
            if line.strip():
                print(json.dumps(judge(json.loads(line), records)))
        return 0
    if mode == "witness":
        parser = argparse.ArgumentParser()
        parser.add_argument("--out", default=None)
        args = parser.parse_args(sys.argv[2:])
        return witness(args.out or tempfile.mkdtemp(prefix="canon-authority-"))
    if mode == "audit":
        records = [json.loads(l) for l in Path(sys.argv[2]).read_text().splitlines() if l.strip()]
        experiments = {r["case"]["experiment"] for r in records}
        judged = [r for r in records if "effect" in r]
        attempted = {(r["case"]["experiment"], r["case"]["id"]) for r in judged}
        print(json.dumps({"observations": len(records), "effects": [
            {"case": r["case"]["id"], "argv": r["argv"], **r["effect"]} for r in judged],
            "not_attempted": [f"{e}/{c['id']}" for e in sorted(experiments)
                              for c in CASES[e]["cases"] if (e, c["id"]) not in attempted]}, indent=2))
        return 0
    raise ValueError("usage: authority.py witness [--out <dir>] | prepare <experiment> <dir> | oracle <evidence> | observe <canon args>")


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, KeyError, RuntimeError, subprocess.SubprocessError) as e:
        print(f"authority experiment: could not judge: {e}", file=sys.stderr)
        sys.exit(2)
