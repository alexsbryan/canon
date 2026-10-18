#!/usr/bin/env python3
"""contract-guard.py — keep an agent from moving canon's contract by surprise.

Wired from .claude/settings.json. One script, one subcommand per hook event:

  pre-tool     refuse moving the standard; ask before touching the machinery
               that judges it
  post-edit    the first contract-judged edit of a session says what that
               means, before the agent has built on it
  stop         an agent with contract-judged changes it has not checked
               cannot finish its turn without running the check
  session      install the pre-push gate in a clone that never ran
               install-git-hooks.sh

## Why hooks, and why these ones

0.5.0 was adopted backwards: the pin moved in a direct push, Contract CI
went red, and the next push laundered it green (docs/CONTRACT_ADOPTIONS.md
has the account). CI saw it, but only after the code had left. The agent
that wrote it never ran the check, because nothing asked it to until CI
did. These hooks move each signal to the moment the agent acts.

Shell matching is best-effort: a command that writes a file through python
gets past it. The enforcement is the evaluator, the pre-push gate, and the
ruleset on main. This is the part that speaks up early.
"""

import json
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(os.environ.get("CLAUDE_PROJECT_DIR") or Path(__file__).resolve().parents[2])
PIN = ".github/contract-baseline"

# The holder's to move. An agent drafts the adoption; it does not make it.
REFUSED = (PIN,)

# The machinery that judges a change, and this guard itself. An edit here is
# legitimate but never incidental, so the person is asked.
ASKED = (
    "scripts/contract-check.py",
    "scripts/pre-push.sh",
    ".github/workflows/contract.yml",
    "crates/canon-contract/",
    "docs/CONTRACT_ADOPTIONS.md",
    ".githooks/",
    ".claude/settings.json",
    ".claude/hooks/",
)

# What the evaluator judges: the two crates' sources and manifests, the
# lockfile, the laws and the format. Tests are left out because the
# evaluator replaces them with the standard's own.
JUDGED = ("crates/canon-core/", "crates/canon-cli/", "Cargo.toml", "Cargo.lock",
          "docs/CONTRACT.md", "docs/SPEC.md", PIN)

PASSING = ("preserves_checked_contract", "extends_checked_contract")
REPORT = ROOT / "target/contract-report.json"
STATE = ROOT / "target/contract-guard"


def relative(path):
    try:
        return Path(path).resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return None


def under(path, prefixes):
    return path is not None and any(path == p or path.startswith(p) for p in prefixes)


def judged(path):
    if not under(path, JUDGED):
        return False
    parts = path.split("/")
    return "tests" not in parts and not path.endswith("tests.rs")


def emit(payload):
    json.dump(payload, sys.stdout)
    sys.exit(0)


def decide(decision, reason):
    emit({"hookSpecificOutput": {"hookEventName": "PreToolUse",
                                 "permissionDecision": decision,
                                 "permissionDecisionReason": reason}})


REFUSAL = ("Moving the contract standard is the holder's decision, not an agent's. "
           "Stop and tell the user what you need: the evaluator verdict "
           "(python3 scripts/contract-check.py, target/contract-report.json) and, if "
           "they choose to adopt, docs/CONTRACT_ADOPTIONS.md says how. They move "
           f"{PIN} and the CANON_CONTRACT_BASELINE variable themselves.")


def pre_tool(event):
    tool, given = event.get("tool_name", ""), event.get("tool_input") or {}
    if tool in ("Edit", "Write", "MultiEdit", "NotebookEdit"):
        path = relative(given.get("file_path") or given.get("notebook_path") or "")
        if under(path, REFUSED):
            decide("deny", REFUSAL)
        if under(path, ASKED):
            decide("ask", f"{path} is part of what judges the contract, or of this guard. "
                          "Approve only if this change is what you asked for.")
        return
    if tool != "Bash":
        return
    command = given.get("command", "")
    writes_variable = (re.search(r"\bgh\s+variable\s+(set|delete)\b[^\n;&|]*CANON_CONTRACT_BASELINE", command)
                       or re.search(r"\bgh\s+api\b[^\n;&|]*actions/variables", command)
                       and re.search(r"(-X|--method)\s*(POST|PUT|PATCH|DELETE)|\s-[fF]\s|--field|--input", command))
    writes_rules = (re.search(r"\bgh\s+api\b[^\n;&|]*(/rulesets|/protection)", command)
                    and re.search(r"(-X|--method)\s*(POST|PUT|PATCH|DELETE)|\s-[fF]\s|--field|--input", command))
    if writes_variable:
        decide("deny", REFUSAL)
    if writes_rules:
        decide("deny", "Changing main's rulesets or branch protection loosens the contract gate. "
                       "Tell the user what you need; the change is theirs to make.")
    for paths, decision in ((REFUSED, "deny"), (ASKED, "ask")):
        for path in paths:
            name = re.escape(path.rstrip("/"))
            if not re.search(name, command):
                continue
            redirect = re.search(r">{1,2}\s*[\"']?[^\s;&|]*" + name, command)
            tee = re.search(r"\btee\b[^\n;&|]*" + name, command)
            verb = re.search(r"\b(sed\s+(-[a-zA-Z]*i|--in-place)|perl\s+-[a-zA-Z]*i|mv|cp|rm|truncate|ln|install|"
                             r"git\s+(checkout|restore|reset|apply|am|cherry-pick|revert|rm|mv))\b", command)
            if redirect or tee or verb:
                decide(decision, REFUSAL if decision == "deny" else
                       f"This command may change {path}, part of what judges the contract. "
                       "Approve only if that is what you asked for.")
    if re.search(r"\bgit\s+push\b[^\n;&|]*--no-verify", command):
        decide("ask", "--no-verify skips the pre-push gate, including the contract check. "
                      "pre-push.sh asks that a bypass be said out loud on the PR.")


def post_edit(event):
    path = relative((event.get("tool_input") or {}).get("file_path") or "")
    if not judged(path):
        return
    STATE.mkdir(parents=True, exist_ok=True)
    marker = STATE / f"warned-{event.get('session_id', 'session')}"
    if marker.exists():
        return
    marker.touch()
    emit({"hookSpecificOutput": {"hookEventName": "PostToolUse", "additionalContext": (
        f"{path} is judged by canon's contract (docs/CONTRACT.md). Before you commit, run "
        "python3 scripts/contract-check.py (about 40s warm). Only preserves_checked_contract and "
        "extends_checked_contract pass. A public enum variant, a changed law, a SPEC.md change or "
        "different record bytes do not: if the check says breaks_checked_contract or not_established, "
        "stop and tell the user, because adopting a new standard is their decision. Do not move "
        f"{PIN} to make the check pass.")}})


def git(*args):
    result = subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True)
    return result.stdout if result.returncode == 0 else None


def changed():
    """Contract-judged files that differ from origin/main, committed or not."""
    base = (git("merge-base", "HEAD", "origin/main") or "").strip() or "HEAD"
    names = set((git("diff", "--name-only", base) or "").split())
    names |= set((git("ls-files", "--others", "--exclude-standard") or "").split())
    return sorted(name for name in names if judged(name))


def stop(event):
    if event.get("stop_hook_active"):
        return
    files = changed()
    if not files:
        return
    newest = max(((ROOT / f).stat().st_mtime for f in files if (ROOT / f).exists()), default=0)
    shown = ", ".join(files[:5]) + (f" and {len(files) - 5} more" if len(files) > 5 else "")
    if not REPORT.exists() or REPORT.stat().st_mtime < newest:
        emit({"decision": "block", "reason": (
            f"You changed contract-judged files ({shown}) and have not run the contract check since. "
            "Run python3 scripts/contract-check.py and tell the user its verdict before you finish. "
            "If it is not preserves_checked_contract or extends_checked_contract, say what it found "
            "(target/contract-report.json) and that adopting it is their decision.")})
    report = json.loads(REPORT.read_text())
    if report.get("outcome") in PASSING:
        return
    STATE.mkdir(parents=True, exist_ok=True)
    seen = STATE / "reported"
    stamp = str(REPORT.stat().st_mtime)
    if seen.exists() and seen.read_text() == stamp:
        return
    seen.write_text(stamp)
    emit({"decision": "block", "reason": (
        f"The contract check says {report.get('outcome')} against {report.get('baseline')}"
        + (f" ({report['error']})" if report.get("error") else "") + ". "
        "Tell the user plainly before you finish: what changed (the evidence in "
        "target/contract-report.json), that this will fail Contract CI, and that adopting a new "
        "standard is their decision (docs/CONTRACT_ADOPTIONS.md). Do not move the pin or work "
        "around the check.")})


def session(event):
    if (git("config", "--local", "--get", "core.hooksPath") or "").strip():
        return
    result = subprocess.run([str(ROOT / "scripts/install-git-hooks.sh")], cwd=ROOT,
                            capture_output=True, text=True)
    if result.returncode == 0:
        emit({"hookSpecificOutput": {"hookEventName": "SessionStart", "additionalContext": (
            "This clone had no pre-push gate; core.hooksPath now points at .githooks, so "
            "git push runs scripts/pre-push.sh, the contract check included.")}})


if __name__ == "__main__":
    handlers = {"pre-tool": pre_tool, "post-edit": post_edit, "stop": stop, "session": session}
    try:
        event = json.load(sys.stdin)
    except ValueError:
        event = {}
    handlers[sys.argv[1]](event)
