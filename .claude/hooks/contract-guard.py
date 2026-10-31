#!/usr/bin/env python3
"""contract-guard.py — an agent changes canon's contract only when the person says so.

Wired from .claude/settings.json. One script, one subcommand per hook event:

  pre-tool     ask before an agent edits contract/ or blesses it
               (CANON_BLESS), or touches what holds the code to it
  session      install the pre-push gate in a clone that never ran
               install-git-hooks.sh

## Why the gate sits at the edit

The contract is the files in contract/, and `cargo test` fails when the code
and those files part. The one way to make them agree again on purpose is to
rewrite the files, so that rewrite is where the person decides: before it
happens, not in a verdict computed after the change has been built on. The
diff of contract/ is the record of what was decided.

Shell matching is best-effort: a command that writes a file through python
gets past it. The test, the diff and CODEOWNERS on contract/ are the backstop.
"""

import json
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(os.environ.get("CLAUDE_PROJECT_DIR") or Path(__file__).resolve().parents[2])

# The contract, what holds the code to it, and this guard. An edit here is
# legitimate but never incidental, so the person is asked.
ASKED = (
    "contract/",
    "crates/canon-contract/",
    "scripts/pre-push.sh",
    ".githooks/",
    ".claude/settings.json",
    ".claude/hooks/",
)

WRITES = re.compile(r"\b(sed\s+(-[a-zA-Z]*i|--in-place)|perl\s+-[a-zA-Z]*i|mv|cp|rm|truncate|ln|install"
                    r"|git\s+(rm|mv|checkout|restore))\b")


def relative(path):
    try:
        return Path(path).resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return None


def under(path, prefixes):
    return path is not None and any(path == p or path.startswith(p) for p in prefixes)


def emit(payload):
    json.dump(payload, sys.stdout)
    sys.exit(0)


def decide(decision, reason):
    emit({"hookSpecificOutput": {"hookEventName": "PreToolUse",
                                 "permissionDecision": decision,
                                 "permissionDecisionReason": reason}})


ASK = ("{what} changes canon's contract, or what holds the code to it. Approve only if "
       "this change is one you asked for; the diff of contract/ is the record of it.")


def pre_tool(event):
    tool, given = event.get("tool_name", ""), event.get("tool_input") or {}
    if tool in ("Edit", "Write", "MultiEdit", "NotebookEdit"):
        path = relative(given.get("file_path") or given.get("notebook_path") or "")
        if under(path, ASKED):
            decide("ask", ASK.format(what=path))
        return
    if tool != "Bash":
        return
    command = given.get("command", "")
    if "CANON_BLESS" in command:
        decide("ask", ASK.format(what="CANON_BLESS rewrites contract/ to what the code does now; it"))
    if (re.search(r"\bgh\s+api\b[^\n;&|]*(/rulesets|/protection)", command)
            and re.search(r"(-X|--method)\s*(POST|PUT|PATCH|DELETE)|\s-[fF]\s|--field|--input", command)):
        decide("deny", "Changing main's rulesets or branch protection loosens what holds the contract. "
                       "Tell the user what you need; the change is theirs to make.")
    for path in ASKED:
        name = re.escape(path.rstrip("/"))
        if not re.search(name, command):
            continue
        if (re.search(r">{1,2}\s*[\"']?[^\s;&|]*" + name, command)
                or re.search(r"\btee\b[^\n;&|]*" + name, command) or WRITES.search(command)):
            decide("ask", ASK.format(what=f"This command writes to {path}, which"))


def session(event):
    configured = subprocess.run(["git", "config", "--local", "--get", "core.hooksPath"], cwd=ROOT,
                                capture_output=True, text=True).stdout.strip()
    if configured:
        return
    result = subprocess.run([str(ROOT / "scripts/install-git-hooks.sh")], cwd=ROOT,
                            capture_output=True, text=True)
    if result.returncode == 0:
        emit({"hookSpecificOutput": {"hookEventName": "SessionStart", "additionalContext": (
            "This clone had no pre-push gate; core.hooksPath now points at .githooks, so "
            "git push runs scripts/pre-push.sh.")}})


if __name__ == "__main__":
    handlers = {"pre-tool": pre_tool, "session": session}
    try:
        event = json.load(sys.stdin)
    except ValueError:
        event = {}
    handlers[sys.argv[1]](event)
