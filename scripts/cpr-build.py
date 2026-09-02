#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Build a CPR fixture from the shared spine and one vocabulary of nouns.

The claim this script exists to make checkable: **adding a common-pool
resource to the study costs naming and nothing else.** The spine under
`fixtures/cpr/_spine/` is the only place mechanism lives, and it is byte
identical for a codebase, a coliving house, a compute mesh and an alpine
pasture. A vocabulary names actors, scopes, commitments and proposals; it
cannot name a rule, a policy, an authority or an outcome.

`expected.json` is PREDICTED here, in Python, from the vocabulary and the
policy semantics — never recorded from a run. It is a second implementation
of what the Rust decision layer should do, so `canon replay` is a
differential test between the two and not a tautology. The two values that
cannot be predicted, a draw's `seats` and `seed`, are recorded on request
with `--pin-draw` and labelled as recorded in this file's output.

    scripts/cpr-build.py fixtures/cpr/harbourside-makerspace
    scripts/cpr-build.py --all --pin-draw
"""

from __future__ import annotations

import argparse
import copy
import datetime as dt
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SPINE = ROOT / "fixtures" / "cpr" / "_spine"
CPR = ROOT / "fixtures" / "cpr"

# ── the timeline is mechanism, not naming ───────────────────
#
# A vocabulary supplies one date, `start`. Every other instant in the fixture
# is this table, so no institution can quietly move its own clock to make a
# horizon lapse at a convenient moment.
DAYS = {
    "adopt": 0, "inherit": 1, "grant": 2, "inner_grant": 3, "monitor_grant": 4,
    "policy": 5, "local": 13, "scope": 16, "rank": 17, "silence": 19,
    "p3_policy": 60, "monitor_pos": 69, "monitor_dismiss": 70,
    "clock_after_horizon": 126, "decided_1": 127, "decided_2": 128,
    "accept": 129, "revisit": 300, "upgrade": 130, "upstream_retract": 131,
    "panel_commit": 172, "seal": 173, "clock_past_boundary": 183, "reveal": 183,
}
MONITOR_HORIZON_DAYS = 120
PANEL_BOUNDARY_DAYS = 181

ABLATIONS = ("no_inner_grants", "no_local_policy", "no_monitor_horizon",
             "upstream_retracts_local")


# ── a template engine small enough to read in one sitting ───

def render(tmpl: str, ctx: dict) -> str:
    """`{{a.b}}`, `{{#each xs as x}}`, `{{#if k}}`, `{{#unless k}}`."""
    lines = tmpl.split("\n")
    out: list[str] = []
    i = 0
    while i < len(lines):
        line = lines[i]
        each = re.fullmatch(r"\{\{#each ([\w.]+) as (\w+)\}\}", line.strip())
        cond = re.fullmatch(r"\{\{#(if|unless) ([\w.]+)\}\}", line.strip())
        if each:
            body, i = _block(lines, i + 1, "{{/each}}")
            for item in _lookup(ctx, each.group(1)):
                out.append(render("\n".join(body), {**ctx, each.group(2): item}))
            continue
        if cond:
            body, i = _block(lines, i + 1, "{{/%s}}" % cond.group(1))
            truthy = bool(_lookup(ctx, cond.group(2), missing=False))
            if truthy == (cond.group(1) == "if"):
                out.append(render("\n".join(body), ctx))
            continue
        out.append(_subst(line, ctx))
        i += 1
    return "\n".join(out)


def _block(lines: list[str], i: int, closer: str) -> tuple[list[str], int]:
    body = []
    while lines[i].strip() != closer:
        body.append(lines[i])
        i += 1
        if i >= len(lines):
            raise SystemExit(f"unclosed block, expected {closer}")
    return body, i + 1


def _subst(line: str, ctx: dict) -> str:
    def one(m: re.Match) -> str:
        v = _lookup(ctx, m.group(1))
        if isinstance(v, bool) or v is None:
            raise SystemExit(f"`{m.group(1)}` is not a value: {v!r}")
        return str(v) if isinstance(v, int) else json.dumps(str(v))[1:-1]
    return re.sub(r"\{\{([\w.]+)\}\}", one, line)


def _lookup(ctx: dict, path: str, missing="__raise__"):
    cur = ctx
    for part in path.split("."):
        if isinstance(cur, list):
            cur = cur[int(part)]
            continue
        if not isinstance(cur, dict) or part not in cur:
            if missing != "__raise__":
                return missing
            raise SystemExit(f"vocabulary has no `{path}`")
        cur = cur[part]
    return cur


# ── vocabulary ──────────────────────────────────────────────

def load_vocab(d: Path) -> dict:
    v = json.loads((d / "vocab.json").read_text())
    parent = v.get("extends")
    if parent:
        base = load_vocab(CPR / parent)
        base.pop("extends", None)
        merged = copy.deepcopy(base)
        _merge(merged, v)
        v = merged
    v.setdefault("ablate", {})
    for k in v["ablate"]:
        if k not in ABLATIONS:
            raise SystemExit(f"unknown ablation `{k}`; known: {ABLATIONS}")
    for k in ABLATIONS:
        v["ablate"].setdefault(k, False)
    return v


def _merge(base: dict, over: dict) -> None:
    for k, val in over.items():
        if isinstance(val, dict) and isinstance(base.get(k), dict):
            _merge(base[k], val)
        else:
            base[k] = val


def context(v: dict) -> dict:
    start = dt.date.fromisoformat(v["start"])
    day = lambda n: (start + dt.timedelta(days=n)).isoformat()
    ts = lambda n: int(dt.datetime.combine(
        start + dt.timedelta(days=n), dt.time(), dt.timezone.utc).timestamp())

    d = {k: day(n) for k, n in DAYS.items()}
    d["monitor_horizon_ts"] = ts(MONITOR_HORIZON_DAYS)
    d["panel_after_ts"] = ts(PANEL_BOUNDARY_DAYS)

    slug = v["slug"]
    sealers = []
    for who in v["members"][:5]:
        secret = f"{slug}-{who.split(':')[-1]}-panel"
        sealers.append({"who": who, "secret": secret,
                        "digest": hashlib.sha256(secret.encode()).hexdigest()})
    # The shape of the commons: how many levels it has, and whether it was
    # forked from anything. Both are facts about the institution, not rules.
    middle = f"{v['outer']}.{v['middle_leaf']}" if v.get("middle_leaf") else ""
    inner = f"{middle or v['outer']}.{v['inner_leaf']}"
    ctx = dict(v)
    ctx.update({
        "d": d,
        "middle": middle,
        "inner": inner,
        "ladder": f"{v['outer']}.{v['ladder_leaf']}",
        "insider": v["insiders"],
        "forked": v.get("forked", True),
        "sealers": sealers,
        "revealers": sealers[:4],
    })
    # Upstream act ids are opaque and only have to be distinct and stable.
    ctx["up"] = {str(i): "can-" + hashlib.sha256(
        f"{slug}-upstream-{i}".encode()).hexdigest()[:10] for i in (1, 2, 3)}
    return ctx


# ── what the decision layer should do ───────────────────────
#
# Predicted from the policy semantics, not recorded from a run.

def predict(v: dict, ctx: dict, seed_acts: int, scenario: list[dict]) -> dict:
    ab = v["ablate"]
    monitor, members = v["monitor"], v["members"]
    ins = v["insiders"]
    outer, middle, inner = v["outer"], ctx["middle"], ctx["inner"]
    forked = ctx["forked"]
    inner_depth = inner.count(".") + 1

    n_inherited = 3 if forked else 0
    n_local = (5 if forked else 8) - (3 if ab["upstream_retracts_local"] else 0)

    def live_grants(monitor_live: bool, upstream: bool) -> list[tuple[int, str]]:
        """Every live grant, as (depth of its scope, actor)."""
        g = [(1, m) for m in members]
        if upstream:
            g.append((1, v["upstream_actor"]))
        if middle:
            g += [(2, h) for h in v["mid_holders"]]
        if not ab["no_inner_grants"]:
            g += [(inner_depth, a) for a in ins]
        if monitor_live:
            g.append((inner_depth, monitor))
        return g

    def covering(depth: int, monitor_live: bool, upstream: bool):
        # Every grant in this spine sits on the path from the outer boundary
        # down to the inner one, so a grant covers a scope exactly when it is
        # no deeper than that scope.
        return [(d, a) for d, a in live_grants(monitor_live, upstream) if d <= depth]

    def deciders(depth: int, monitor_live: bool, upstream: bool) -> list[str]:
        """Deepest grant first, then actor, one row per person."""
        rows = sorted(covering(depth, monitor_live, upstream), key=lambda r: (-r[0], r[1]))
        out, seen = [], set()
        for _, a in rows:
            if a not in seen:
                seen.add(a)
                out.append(a)
        return out

    def holders(depth: int, monitor_live: bool, upstream: bool) -> list[str]:
        """The deepest level on its own — what subsidiarity routes to."""
        rows = covering(depth, monitor_live, upstream)
        top = max(d for d, _ in rows)
        return sorted(a for d, a in rows if d == top)

    # Under `no_inner_grants` the only deep holder is the monitor, so an
    # insider is no longer "including you" — that IS the failure.
    insider_is_holder = not ab["no_inner_grants"]
    p3_rule = "subsidiarity" if ab["no_local_policy"] else "threshold"

    acts = {}
    n = seed_acts
    for s in scenario:
        if s["step"] == "act":
            n += 1
        acts[s.get("name", s["step"])] = n

    e: dict = {}
    e["boundary-who-holds-the-inner"] = {
        "policy": "subsidiarity", "deciders": deciders(inner_depth, True, False),
        "holders": holders(inner_depth, True, False)}
    e["boundary-an-insider-decides"] = {
        "outcome": "supported", "rule": "subsidiarity",
        "authority": "act" if insider_is_holder else "ask-one",
        "because": "including you" if insider_is_holder else "not you",
        "cites": [f"@{v['c']['inner_a']['label']}"]}
    e["boundary-an-outsider-does-not"] = {
        "outcome": "supported", "authority": "ask-one", "because": "not you"}
    e["boundary-nobody-holds-this"] = {
        "outcome": "unaddressed", "authority": "refuse",
        "because": f"nobody holds standing over `{v['unheld']}`"}

    e["congruence-forked-and-diverged"] = {
        "lineage": v["lineage"] if forked else None,
        "generation": v["generation"] if forked else None,
        "inherited": n_inherited, "local": 5 if forked else 8}

    if ab["no_local_policy"]:
        # The rule the insiders never got to change still routes to them.
        e["collective-choice-under-the-new-rule"] = {
            "outcome": "conflicts", "authority": "ask-one", "rule": "subsidiarity",
            "because": "not you", "voices": [ins[0]]}
    else:
        e["collective-choice-under-the-new-rule"] = {
            "outcome": "supported", "authority": "act", "rule": "threshold",
            "because": "1 against, 2 needed", "voices": [ins[0]]}

    # `unattended` reports adjudications with NO PERSON behind them. Where a
    # community monitors with one of its own people — which is Ostrom's
    # actual finding — the dismissal carries that person's name and there is
    # nothing unattended to report. Both branches are attribution; only one
    # of them needs the library's help.
    e["monitors-adjudication-is-surfaced"] = {
        "unattended": ["@monitor-dismiss"] if monitor.startswith("agent:") else []}
    e["monitors-standing-lapsed"] = (
        {"count": 0, "targets": []} if ab["no_monitor_horizon"]
        else {"count": 1, "targets": ["@monitor-grant"]})
    e["monitors-record-is-queryable"] = {"positions": 1, "decided": 0}

    lad = f"@{v['c']['ladder']['label']}"
    for name, rung, prior in [("graduated-first", "ask-one", 0),
                              ("graduated-second", "ask-panel", 1),
                              ("graduated-third", "refuse", 2)]:
        e[name] = {"outcome": "conflicts", "authority": rung, "rule": "graduated/default",
                   "because": f"{prior} prior decision(s)", "cites": [lad]}
    e["graduated-a-different-subject-restarts"] = {
        "outcome": "conflicts", "authority": "ask-one",
        "because": f"0 prior decision(s) about `{v['other_subject']['about']}`"}

    e["conflict-before"] = {"acts": acts["conflict-before"], "tolerated": 0, "live": 8}
    e["conflict-surfaced"] = {
        "outcome": "conflicts", "authority": "refuse", "because": "one is enough",
        "cites": [f"@{v['c']['clash_a']['label']}", f"@{v['c']['clash_b']['label']}"]}
    e["conflict-after"] = {"acts": acts["conflict-after"], "tolerated": 1, "live": 8}

    e["organize-the-fork-keeps-what-it-wrote"] = {
        "generation": v["generation_next"] if forked else None,
        "inherited": n_inherited, "local": n_local}

    up = ab["upstream_retracts_local"]
    mon = ab["no_monitor_horizon"]
    e["nesting-the-outer"] = {
        "policy": "cautious/entrenched/consent",
        "deciders": deciders(1, False, up), "holders": holders(1, False, up)}
    if middle:
        e["nesting-the-middle"] = {
            "policy": "subsidiarity",
            "deciders": deciders(2, False, up), "holders": holders(2, False, up)}
    e["nesting-the-inner"] = {
        "policy": p3_rule,
        "deciders": deciders(inner_depth, mon, up),
        "holders": holders(inner_depth, mon, up)}

    e["entrenchment-amending-a-convention"] = {
        "outcome": "supported", "authority": "act", "because": "no objection"}
    e["entrenchment-amending-a-principle"] = {
        "outcome": "supported", "authority": "ask-panel",
        "because": "entrenched: amends a `principle`"}

    e["reversible-irreversible-and-unsupported"] = {
        "outcome": "unaddressed", "authority": "refuse",
        "because": "cautious: irreversible", "cites": []}
    e["reversible-the-same-behind-a-flag"] = {
        "outcome": "unaddressed", "authority": "act-and-notify"}
    e["silence-is-not-a-gap"] = {"outcome": "unaddressed", "silence": True}

    e["draw-before-anyone-opened-refuses"] = {
        "error": "nobody opened a secret, so there is no seed "
                 "— and there is no default seed"}
    pool = len(members) + (1 if ab["upstream_retracts_local"] else 0) - 1
    e["draw-the-panel"] = {"pool": pool, "withheld": [members[4]]}
    return e


# ── build one fixture ───────────────────────────────────────

def build(d: Path, pin_draw: bool) -> None:
    v = load_vocab(d)
    v["slug"] = d.name
    ctx = context(v)

    seed = render((SPINE / "seed.jsonl.tmpl").read_text(), ctx)
    scen = render((SPINE / "scenario.jsonl.tmpl").read_text(), ctx)
    seed = "\n".join(l for l in seed.split("\n") if l.strip() != "") + "\n"
    scen = "\n".join(l for l in scen.split("\n") if l.strip() != "") + "\n"

    body = lambda t: [json.loads(l) for l in t.split("\n")
                      if l.strip() and not l.startswith("//")]
    seed_acts, scenario = body(seed), body(scen)

    (d / "acts.jsonl").write_text(seed)
    (d / "scenario.jsonl").write_text(scen)

    e = predict(v, ctx, len(seed_acts), scenario)
    if pin_draw:
        e["draw-the-panel"].update(recorded_draw(d))
    (d / "expected.json").write_text(json.dumps(e, indent=2) + "\n")
    print(f"{d.name}: {len(seed_acts)} seed acts, {len(scenario)} steps, "
          f"{len(e)} assertions"
          + (" (draw pinned)" if pin_draw else ""))


def recorded_draw(d: Path) -> dict:
    """The two values a predictor cannot know. Recorded, and labelled so."""
    binary = ROOT / "target" / "debug" / "canon"
    r = subprocess.run([str(binary), "replay", str(d), "--json"],
                       capture_output=True, text=True)
    # `--json` prints the run, then the pass/fail line; take the object only.
    got = json.JSONDecoder().raw_decode(r.stdout)[0]["draw-the-panel"]
    return {"seats": got["seats"], "seed": got["seed"]}


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("dirs", nargs="*")
    ap.add_argument("--all", action="store_true")
    ap.add_argument("--pin-draw", action="store_true",
                    help="record `seats` and `seed`, which cannot be predicted")
    a = ap.parse_args()
    dirs = ([p for p in sorted(CPR.iterdir())
             if p.is_dir() and (p / "vocab.json").exists()]
            if a.all else [Path(x) for x in a.dirs])
    if not dirs:
        raise SystemExit("nothing to build; pass a fixture dir or --all")
    for d in dirs:
        build(d, a.pin_draw)


if __name__ == "__main__":
    main()
