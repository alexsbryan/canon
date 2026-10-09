#!/usr/bin/env python3
"""Compute compatibility against a pinned, git-owned standard.

Candidate tests, fixtures, and expectations are never used as the baseline.
CI runs this evaluator and its Rust inspector from the PR's base checkout.
"""

import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import io
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import tempfile
import tomllib


TOOL_ROOT = Path(__file__).resolve().parents[1]


def command(args, cwd, env=None):
    return subprocess.run(args, cwd=cwd, env=env, text=True,
                          stdout=subprocess.PIPE, stderr=subprocess.STDOUT)


def checked(args, cwd, env=None):
    result = command(args, cwd, env)
    if result.returncode:
        raise RuntimeError(result.stdout)
    return result.stdout


def laws(text):
    result = {}
    for line in text.splitlines():
        match = re.match(r"^\| `([a-z_]+)` \| (.*?) \| (.*?) \|$", line)
        if match:
            name, guarantee, _ = match.groups()
            if name in result:
                raise ValueError(f"duplicate contract law: {name}")
            result[name] = " ".join(guarantee.split())
    if not result:
        raise ValueError("no named contract laws found")
    return result


def tests_list(output):
    return Counter(line[:-6] for line in output.splitlines() if line.endswith(": test"))


def classification(old_surface, new_surface, old_laws, new_laws,
                   checks_pass, observations_equal, consumers_pass, violated_laws=(), specification_unchanged=True):
    removed = sorted((old_surface.keys() - new_surface.keys()) | (old_laws.keys() - new_laws.keys()))
    changed = sorted(name for name in old_surface.keys() & new_surface.keys()
                     if old_surface[name] != new_surface[name])
    changed_laws = sorted(name for name in old_laws.keys() & new_laws.keys()
                         if old_laws[name] != new_laws[name])
    added = sorted((new_surface.keys() - old_surface.keys()) | (new_laws.keys() - old_laws.keys()))
    added_laws = sorted(new_laws.keys() - old_laws.keys())
    evidence = {"removed": removed, "changed_surface": changed,
                "changed_laws": changed_laws, "added_laws": added_laws, "added": added,
                "violated_laws": list(violated_laws), "specification_changed": not specification_unchanged}
    if removed or not observations_equal or not consumers_pass or violated_laws:
        return "breaks_checked_contract", evidence
    if not checks_pass or changed or changed_laws or added_laws or not specification_unchanged:
        return "not_established", evidence
    return ("extends_checked_contract" if added else "preserves_checked_contract"), evidence


def archive(root, revision, destination):
    data = subprocess.check_output(["git", "archive", revision], cwd=root)
    with tarfile.open(fileobj=io.BytesIO(data)) as contents:
        contents.extractall(destination, filter="data")


def replace_dev_dependencies(candidate, baseline):
    # Freeze the test dependencies too. This intentionally supports the plain
    # manifest sections used here; a new target layout must extend the evaluator.
    def without_dev(text):
        return re.sub(r"(?ms)^\[dev-dependencies\]\n.*?(?=^\[|\Z)", "", text)
    old = re.search(r"(?ms)^\[dev-dependencies\]\n.*?(?=^\[|\Z)", baseline)
    return without_dev(candidate).rstrip() + "\n" + (old.group() if old else "")


def prepare_candidate(root, baseline, destination, inspector):
    if destination.exists():
        shutil.rmtree(destination)
    shutil.copytree(baseline, destination)
    manifest = (root / "Cargo.toml").read_text()
    manifest = re.sub(r"(?ms)^members\s*=\s*\[.*?\]",
                      'members = ["crates/canon-core", "crates/canon-cli"]', manifest, count=1)
    (destination / "Cargo.toml").write_text(manifest)
    shutil.copy2(root / "Cargo.lock", destination / "Cargo.lock")
    for crate in ("canon-core", "canon-cli"):
        relative = Path("crates") / crate
        candidate_manifest = (root / relative / "Cargo.toml").read_text()
        parsed = tomllib.loads(candidate_manifest)
        package = parsed["package"]
        if package.get("autotests", True) is False or package.get("autobins", True) is False:
            raise ValueError("automatic baseline test/bin targets must stay enabled")
        if parsed.get("test") or parsed.get("lib", {}).get("path", "src/lib.rs") != "src/lib.rs":
            raise ValueError("custom test/library target layouts are not established")
        (destination / relative / "Cargo.toml").write_text(replace_dev_dependencies(
            candidate_manifest, (baseline / relative / "Cargo.toml").read_text()))
        old_src = baseline / relative / "src"
        new_src = root / relative / "src"
        output_src = destination / relative / "src"
        # Remove production files that disappeared, retaining external baseline
        # test modules. The candidate cannot substitute a no-op for a law.
        for path in output_src.rglob("*.rs"):
            rel = path.relative_to(output_src)
            if "tests" not in rel.parts and rel.name != "tests.rs":
                path.unlink()
        for path in new_src.rglob("*.rs"):
            rel = path.relative_to(new_src)
            if "tests" in rel.parts or rel.name == "tests.rs":
                continue
            output = output_src / rel
            output.parent.mkdir(parents=True, exist_ok=True)
            previous = old_src / rel
            if not previous.exists():
                previous = destination / "empty.rs"
                previous.write_text("")
            checked([str(inspector), "transplant", str(previous), str(path), str(output)], root)


def observations(workspace, binary, output, require_success=True):
    """Exact deterministic fixture results and minted record bytes, including IDs."""
    result = {}
    for name in ("fernwood-commons", "eleven-principles"):
        fixture = workspace / "fixtures" / name
        answer = command([str(binary), "replay", str(fixture), "--json"], workspace)
        if answer.returncode < 0:
            raise RuntimeError(f"{name} replay was terminated")
        if require_success and answer.returncode:
            raise ValueError(f"{name} replay failed:\n{answer.stdout}")
        try:
            value, end = json.JSONDecoder().raw_decode(answer.stdout.lstrip())
            footer = answer.stdout.lstrip()[end:].strip()
            if answer.returncode == 0 and footer and not re.fullmatch(r"\d+ step\(s\), all as expected, in .+", footer):
                raise ValueError(f"unrecognized replay footer: {footer}")
        except ValueError:
            if require_success:
                raise
            value = {"unreadable_result": answer.stdout}
        result[f"{name}/decisions"] = value if answer.returncode == 0 else {"exit": answer.returncode, "reading": value}
        record = output / name
        minted = command([str(binary), "replay", str(fixture), "--out", str(record)], workspace)
        if minted.returncode < 0 or (require_success and minted.returncode):
            raise RuntimeError(f"{name} record could not be materialized")
        result[f"{name}/record"] = (record / "acts.jsonl").read_text() if minted.returncode == 0 else {"exit": minted.returncode, "refused": minted.stdout}
    return result


def read_old_records(workspace, binary, records):
    result = {}
    for name in ("fernwood-commons", "eleven-principles"):
        record = records / name
        acts = [json.loads(line) for line in (record / "acts.jsonl").read_text().splitlines()]
        clock = datetime.fromtimestamp(max(a["ts_unix"] for a in acts) + 86_400, timezone.utc).date().isoformat()
        scenario = [{"step": "clock", "at": clock}, {"step": "state"},
                    {"step": "unattended"}, {"step": "overdue"}]
        for scope in sorted({a["scope"] for a in acts if isinstance(a.get("scope"), str)}):
            scenario.append({"step": "who", "scope": scope, "name": "who/" + scope})
        (record / "scenario.jsonl").write_text("".join(json.dumps(step) + "\n" for step in scenario))
        answer = command([str(binary), "replay", str(record), "--json"], workspace)
        if answer.returncode < 0:
            raise RuntimeError("old-record reader was terminated")
        result[f"{name}/old-record-reading"] = {
            "exit": answer.returncode,
            "reading": json.loads(answer.stdout) if answer.returncode == 0 else answer.stdout,
        }
    return result


def run(root):
    report = {"outcome": "not_established", "baseline": None,
              "evaluator": str(TOOL_ROOT), "evidence": {}}
    target = root / "target"
    target.mkdir(exist_ok=True)
    report_path = target / "contract-report.json"
    try:
        report["candidate_revision"] = checked(["git", "rev-parse", "HEAD"], root).strip()
        fingerprint = hashlib.sha256()
        for crate in ("canon-core", "canon-cli"):
            for path in sorted((root / "crates" / crate).rglob("*.rs")):
                fingerprint.update(str(path.relative_to(root)).encode())
                fingerprint.update(path.read_bytes())
            fingerprint.update((root / "crates" / crate / "Cargo.toml").read_bytes())
        fingerprint.update((root / "Cargo.toml").read_bytes())
        fingerprint.update((root / "Cargo.lock").read_bytes())
        for document in ("docs/CONTRACT.md", "docs/SPEC.md", ".github/contract-baseline"):
            fingerprint.update((root / document).read_bytes())
        report["candidate_fingerprint"] = fingerprint.hexdigest()
        policy = TOOL_ROOT / ".github/contract-baseline"
        revision = os.environ.get("CANON_CONTRACT_BASELINE", policy.read_text().strip())
        if not re.fullmatch(r"[0-9a-f]{40}", revision):
            raise ValueError("baseline must be a full commit ID")
        report["baseline"] = revision
        # The candidate cannot replace the evaluator's baseline policy.
        if (root / ".github/contract-baseline").read_text().strip() != revision:
            raise ValueError("candidate changed the independently selected baseline")
        checked(["git", "cat-file", "-e", f"{revision}^{{commit}}"], root)
        env = os.environ.copy()
        for key in ("CANON_DIR", "CANON_ACTOR", "CANON_ENDPOINT", "CANON_MODEL", "CANON_EXTRACT_MODEL", "CANON_API_KEY"):
            env.pop(key, None)
        tool_env = dict(env, CARGO_TARGET_DIR=str(target / "contract-tools"))
        # Build the inspector owned by the evaluator checkout, not the candidate.
        checked(["cargo", "run", "--quiet", "--locked", "--manifest-path",
                 str(TOOL_ROOT / "Cargo.toml"), "--package", "canon-contract", "--",
                 "inspect", str(root / "crates/canon-core/src")], TOOL_ROOT, tool_env)
        executable = "canon-contract.exe" if os.name == "nt" else "canon-contract"
        inspector = target / "contract-tools/debug" / executable
        reference_env = dict(env, CARGO_TARGET_DIR=str(target / "contract-build/reference"))
        candidate_env = dict(env, CARGO_TARGET_DIR=str(target / "contract-build/candidate"))
        cache = target / "contract-work"
        cache.mkdir(exist_ok=True)
        baseline, candidate = cache / "baseline", cache / "candidate"
        if baseline.exists():
            shutil.rmtree(baseline)
        archive(root, revision, baseline)
        with tempfile.TemporaryDirectory(prefix="contract-", dir=target) as temporary:
            work = Path(temporary)
            old_api = json.loads(checked([str(inspector), "inspect", str(baseline / "crates/canon-core/src")], root))
            new_api = json.loads(checked([str(inspector), "inspect", str(root / "crates/canon-core/src")], root))
            old_laws = laws((baseline / "docs/CONTRACT.md").read_text())
            new_laws = laws((root / "docs/CONTRACT.md").read_text())
            same_specification = " ".join((baseline / "docs/SPEC.md").read_text().split()) == " ".join((root / "docs/SPEC.md").read_text().split())
            prepare_candidate(root, baseline, candidate, inspector)
            (baseline / "crates/canon-core/tests/baseline_consumer.rs").write_text(old_api["consumer"])
            report["evidence"]["standard"] = hashlib.sha256(subprocess.check_output(
                ["git", "archive", revision], cwd=root)).hexdigest()
            # Establish that the reference standard itself is runnable first.
            reference = command(["cargo", "test", "--locked", "--workspace"], baseline, reference_env)
            (target / "contract-reference.log").write_text(reference.stdout)
            if reference.returncode:
                raise ValueError("baseline suite did not pass; see target/contract-reference.log")
            reference_list = checked(["cargo", "test", "--locked", "--workspace", "--", "--list"], baseline, reference_env)
            baseline_binary = target / "contract-build/reference/debug" / ("canon.exe" if os.name == "nt" else "canon")
            candidate_binary = target / "contract-build/candidate/debug" / ("canon.exe" if os.name == "nt" else "canon")
            old_observations = observations(baseline, baseline_binary, work / "old-records")
            old_observations.update(read_old_records(baseline, baseline_binary, work / "old-records"))

            # Baseline-derived exhaustive matches and typed field accesses are
            # old consumers compiled against the candidate's public Rust API.
            consumer = candidate / "crates/canon-core/tests/baseline_consumer.rs"
            consumer.write_text(old_api["consumer"])
            consumer_run = command(["cargo", "test", "--locked", "--package", "canon-core",
                                    "--test", "baseline_consumer"], candidate, candidate_env)
            (target / "contract-consumer.log").write_text(consumer_run.stdout)
            if consumer_run.returncode:
                # Only compiler diagnostics are an observed source-compatibility
                # break; transport/toolchain failures establish nothing.
                if re.search(r"error\[E\d+\]:.*?\n\s*--> [^\n]*baseline_consumer\.rs", consumer_run.stdout, re.DOTALL):
                    report["outcome"], delta = classification(old_api["surface"], new_api["surface"],
                        old_laws, new_laws, True, True, False,
                        specification_unchanged=same_specification)
                    report["evidence"].update(delta, consumer="failed: target/contract-consumer.log")
                else:
                    raise ValueError("baseline consumer could not run; see target/contract-consumer.log")
            else:
                # Run the unchanged baseline script from the candidate snapshot
                # so its working directory and dependency checks target the candidate.
                shutil.copy2(baseline / "scripts/core-boundary.sh", candidate / "scripts/core-boundary.sh")
                boundary = command(["bash", "scripts/core-boundary.sh"], candidate, candidate_env)
                (target / "contract-boundary.log").write_text(boundary.stdout)
                suite = command(["cargo", "test", "--locked", "--workspace"], candidate, candidate_env)
                (target / "contract-candidate.log").write_text(suite.stdout)
                current_list = command(["cargo", "test", "--locked", "--workspace", "--", "--list"], candidate, candidate_env)
                inventory = tests_list(reference_list)
                missing = inventory - tests_list(current_list.stdout)
                violated = sorted(name for name in old_laws if re.search(
                    r"^test (?:[a-zA-Z0-9_]+::)*" + re.escape(name) + r" \.\.\. FAILED$",
                    suite.stdout, re.MULTILINE)) if suite.returncode else []
                report["evidence"]["baseline_tests"] = sum(inventory.values())
                report["evidence"]["missing_tests"] = dict(missing)
                compiled = suite.returncode == 0 or re.search(r"^\s*Finished `test` profile", suite.stdout, re.MULTILINE)
                observed = observations(candidate, candidate_binary, work / "new-records", require_success=False) if compiled else None
                if observed is not None:
                    observed.update(read_old_records(candidate, candidate_binary, work / "old-records"))
                equal = observed == old_observations if observed is not None else True
                if observed is not None:
                    report["evidence"]["changed_observations"] = sorted(
                        key for key in old_observations if observed[key] != old_observations[key])
                    report["evidence"]["observation_differences"] = {
                        key: {"before": old_observations[key], "after": observed[key]}
                        for key in old_observations if observed[key] != old_observations[key]
                    }
                report["outcome"], delta = classification(old_api["surface"], new_api["surface"],
                    old_laws, new_laws, suite.returncode == 0 and boundary.returncode == 0
                    and current_list.returncode == 0 and not missing, equal, True, violated, same_specification)
                report["evidence"].update(delta, consumer="passed", baseline_suite="passed" if suite.returncode == 0 else "failed: target/contract-candidate.log",
                                          boundary="passed" if boundary.returncode == 0 else "failed: target/contract-boundary.log")
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError, KeyError) as error:
        report["error"] = str(error)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(f"contract: {report['outcome']} against {report['baseline']}\n  evidence: {report_path}")
    if "error" in report:
        print("  " + report["error"])
    return 0 if report["outcome"] in ("preserves_checked_contract", "extends_checked_contract") else 1


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate", type=Path, default=TOOL_ROOT)
    options = parser.parse_args()
    raise SystemExit(run(options.candidate.resolve()))
