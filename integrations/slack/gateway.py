# SPDX-License-Identifier: AGPL-3.0-or-later
"""The part of the house bot that isn't Slack.

Running canon, keeping the record in sync, and knowing who is who. A second
door into the same canon (Signal, email, a web page) imports this and adds
only its own messages and buttons.

**People, not accounts.** `people.json` maps each person to the accounts
they use, as `<channel>:<id>`:

    {"people": {"Jo": {"accounts": ["slack:U03ABC"]}}}

The name is what the record carries, fixed when the person is first seen,
so a later display-name change doesn't split one person into two. A second
account joins a person only by `link`, never because two names match: two
housemates both called Sam stay two people.
"""

import json
import os
import subprocess
import sys
import threading
from pathlib import Path

CANON = os.environ.get("CANON_BIN", "canon")
# The record. The image sets /data/house/.canon; a bot run any other way
# names its own. No default here: one would point a bot run outside the
# container at a /data that is not there.
if not os.environ.get("CANON_DIR"):
    sys.exit("set CANON_DIR to the house record's .canon directory")
CANON_DIR = Path(os.environ["CANON_DIR"])
SCOPE = os.environ.get("CANON_SCOPE", "house")
PEOPLE = Path(os.environ.get("CANON_PEOPLE", CANON_DIR.parent / "people.json"))
# Commit and push the record after every write. Off unless the checkout has
# a remote to push to.
GIT_SYNC = os.environ.get("CANON_GIT_SYNC", "") not in ("", "0", "false")
# `check` sends the plan and the rules it might touch to the model. A model
# that is not on this machine has to be agreed to, once, here.
ALLOW_REMOTE = os.environ.get("CANON_ALLOW_REMOTE", "") not in ("", "0", "false")

# One writer at a time: two clicks in the same second must not interleave a
# pull, an append and a push.
LOCK = threading.Lock()
PEOPLE_LOCK = threading.Lock()


# ── canon ───────────────────────────────────────────────────


def canon(*args, actor=None):
    """Run canon. Returns (exit code, stdout, stderr)."""
    env = dict(os.environ, CANON_DIR=str(CANON_DIR))
    if actor:
        env["CANON_ACTOR"] = actor
    p = subprocess.run(
        [CANON, *args], env=env, stdin=subprocess.DEVNULL,
        capture_output=True, text=True, timeout=600,
    )
    return p.returncode, p.stdout.strip(), p.stderr.strip()


def canon_json(*args):
    code, out, err = canon(*args, "--json")
    if code != 0:
        raise RuntimeError(err or out)
    return json.loads(out)


def git(*args):
    """git in the record's checkout, under the gateway's name: the container
    has none of its own, and a rebase that replays a local commit needs one."""
    return subprocess.run(
        ["git", "-C", str(CANON_DIR.parent), "-c", "user.name=canon-gateway",
         "-c", "user.email=canon-gateway@localhost", *args],
        capture_output=True, text=True,
    )


def sync(*args):
    """A step of the sync, said in the log when it fails. The act stays
    written here and goes out with the next push that works."""
    r = git(*args)
    if r.returncode != 0:
        print(f"canon-gateway: git {args[0]} failed, the record is not synced: "
              f"{(r.stderr or r.stdout).strip()}", file=sys.stderr, flush=True)
    return r


def union_acts():
    """canon's merge driver for the act log, so a housemate's appends and
    the bot's union instead of leaving conflict markers that canon cannot
    read (`canon merge-driver` says why it is exact). Kept in the
    checkout's own attributes: nothing to commit, set on every sync."""
    git("config", "merge.canon.driver", f"{CANON} merge-driver %O %A %B")
    where = sync("rev-parse", "--git-path", "info/attributes").stdout.strip()
    if not where:
        return
    attrs = CANON_DIR.parent / where
    rule = "acts.jsonl merge=canon\n"
    if rule not in (attrs.read_text() if attrs.exists() else ""):
        attrs.parent.mkdir(parents=True, exist_ok=True)
        with attrs.open("a") as f:
            f.write(rule)


def write(*args, actor, note):
    """A canon verb that appends an act, with the git sync around it."""
    with LOCK:
        if GIT_SYNC:
            union_acts()
            if sync("pull", "--rebase", "--quiet").returncode != 0:
                # Never leave the record mid-rebase: canon reads the file as
                # it stands, conflict markers and all.
                git("rebase", "--abort")
        code, out, err = canon(*args, actor=actor)
        if code == 0 and GIT_SYNC:
            git("add", str(CANON_DIR))
            # Who is who goes with the record, so a machine it moves to
            # names people as this one did. Outside the checkout, it stays.
            git("add", str(PEOPLE))
            sync("commit", "--quiet", "-m", f"canon: {note}")
            sync("push", "--quiet")
        return code, out, err


def status_of(cid):
    """The one line that says where a commitment stands."""
    try:
        lines = canon_json("why", cid).get("lines", [])
    except (RuntimeError, ValueError):
        return ""
    return lines[-1] if lines else ""


def first_id(out):
    word = out.split(maxsplit=1)[0] if out else ""
    return word if word.startswith("can-") else None


def check(plan):
    remote = ["--allow-remote"] if ALLOW_REMOTE else []
    return canon("check", "--area", SCOPE, *remote, plan)


# ── people ──────────────────────────────────────────────────


def load_people():
    """{name: {"accounts": [...]}}. Reads the first version's flat
    `{slack_id: name}` file too."""
    try:
        raw = json.loads(PEOPLE.read_text())
    except (OSError, ValueError):
        return {}
    if "people" in raw:
        return raw["people"]
    people = {}
    for uid, name in raw.items():
        people.setdefault(name, {"accounts": []})["accounts"].append(f"slack:{uid}")
    return people


def save_people(people):
    tmp = PEOPLE.with_suffix(".tmp")
    tmp.write_text(json.dumps({"people": people}, indent=2, sort_keys=True) + "\n")
    tmp.replace(PEOPLE)


def person_for(account):
    """The name an account belongs to, or None."""
    for name, p in load_people().items():
        if account in p["accounts"]:
            return name
    return None


def enroll(account, name):
    """The person this account belongs to, made from `name` if it is new.
    A name already taken by somebody else gets a number, not a merge."""
    with PEOPLE_LOCK:
        people = load_people()
        for known, p in people.items():
            if account in p["accounts"]:
                return known
        base, n = name, 2
        while name in people:
            name, n = f"{base} ({n})", n + 1
        people[name] = {"accounts": [account]}
        save_people(people)
        return name


def link(account, name):
    """Add an account to an existing person. The caller has confirmed it
    through an account that person already has."""
    with PEOPLE_LOCK:
        people = load_people()
        if name not in people:
            raise KeyError(name)
        for p in people.values():
            if account in p["accounts"]:
                p["accounts"].remove(account)
        people[name]["accounts"].append(account)
        save_people({k: v for k, v in people.items() if v["accounts"]})


def actor(name):
    return f"human:{name}"
