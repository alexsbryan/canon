# SPDX-License-Identifier: AGPL-3.0-or-later
"""canon-slack — a house canon in a Slack channel.

Housemates never install canon. They type `/canon ...` and press buttons;
this process runs the `canon` binary on their behalf, as them, against one
checkout on the house server.

Who someone is comes from Slack: the first time a person uses the bot, they
are enrolled in `people.json` under their Slack name (see gateway.py). Acts are **not signed** in this version: anyone with a shell
on the server can write as anyone. docs/SELF_HOST.md says so, and says what
signing will change.

Socket Mode, so nothing listens on the network: the bot dials out to Slack.

    python canon_slack.py              serve
    python canon_slack.py people       map the channel's members, print grants
    python canon_slack.py post-draft <run.json>
                                       post a draft run's candidates for review
    python canon_slack.py post-pending post each proposal the channel has not seen
    python canon_slack.py digest       post what is waiting on people
"""

import json
import os
import sys
import time
from pathlib import Path

from gateway import (
    SCOPE, PEOPLE, CANON_DIR, canon, canon_json, check, enroll, first_id,
    load_people, person_for, status_of, write,
)
from gateway import actor as human

CHANNEL = os.environ.get("CANON_CHANNEL", "")
# Every proposal the bot has posted, so `post-pending` posts each one once.
# It says only what the channel was shown; canon says what is pending.
POSTED = PEOPLE.with_name("posted.json")
# The Object button carries a post's text, as JSON, in a modal's
# private_metadata, which holds 3000 characters; JSON spells a "é" in six.
QUOTE_MAX = 600


def posted():
    try:
        return set(json.loads(POSTED.read_text()))
    except FileNotFoundError:
        return set()


def mark_posted(cid):
    tmp = POSTED.with_suffix(".tmp")
    tmp.write_text(json.dumps(sorted(posted() | {cid}), indent=1) + "\n")
    tmp.replace(POSTED)


# ── people ──────────────────────────────────────────────────


def slack_name(client, user_id):
    u = client.users_info(user=user_id)["user"]
    p = u.get("profile", {})
    return p.get("real_name") or p.get("display_name") or u.get("name") or user_id


def actor_for(client, user_id):
    """`human:<name>` for a Slack user, enrolling them the first time."""
    account = f"slack:{user_id}"
    name = person_for(account) or enroll(account, slack_name(client, user_id))
    return human(name)


def unheard(actor):
    """What to tell someone canon recorded but does not count: only people
    who hold the scope are counted, so their click changes nothing in the
    channel. None for a holder, and for a scope nobody holds yet."""
    try:
        holders = {h["actor"] for h in canon_json("who", SCOPE)}
    except (RuntimeError, ValueError):
        return None
    if not holders or actor in holders:
        return None
    return (f"Recorded, but it doesn't count yet: you don't have a say in {SCOPE}. "
            f"Whoever runs canon for the house can give you one with "
            f"`./house grant \"{actor}\" {SCOPE}`.")


# ── messages ────────────────────────────────────────────────


def proposal_blocks(cid, text, by, status):
    return [
        {"type": "section", "text": {"type": "mrkdwn", "text": f"*{by}* proposed:\n>{text}"}},
        {"type": "context", "elements": [{"type": "mrkdwn", "text": f"`{cid}` · {status}"}]},
        {"type": "actions", "elements": [
            {"type": "button", "action_id": "approve", "value": cid, "style": "primary",
             "text": {"type": "plain_text", "text": "Approve"}},
            {"type": "button", "action_id": "object", "value": cid,
             "text": {"type": "plain_text", "text": "Object"}},
        ]},
    ]


def settled_blocks(cid, text, by, status):
    return [
        {"type": "section", "text": {"type": "mrkdwn", "text": f"*{by}* proposed:\n>{text}"}},
        {"type": "context", "elements": [{"type": "mrkdwn", "text": f"`{cid}` · {status}"}]},
    ]


def candidate_blocks(i, c, run):
    label = "Rule" if c["kind"] == "rule" else "Open question"
    quote = c.get("quote", "").replace("\n", "\n>")
    return [
        {"type": "section", "text": {"type": "mrkdwn",
         "text": f"*{label} found in your notes:*\n{c['text']}\n>{quote}\n_{c['source']}_"}},
        {"type": "actions", "elements": [
            {"type": "button", "action_id": "propose_candidate",
             "value": json.dumps({"run": run, "i": i}), "style": "primary",
             "text": {"type": "plain_text", "text": "Propose this"}},
            {"type": "button", "action_id": "skip_candidate", "value": str(i),
             "text": {"type": "plain_text", "text": "Not ours"}},
        ]},
    ]


def parse_proposal(msg):
    """(who, text) back out of a message `proposal_blocks` wrote."""
    section = msg["blocks"][0]["text"]["text"]
    by, _, text = section.partition("* proposed:\n>")
    return by.lstrip("*"), text


def code_block(s):
    return f"```{s}```" if s else "_nothing_"


HELP = """\
`/canon rules [words]`     what's in force, optionally matching words
`/canon propose <rule>`    put a rule to the house
`/canon question <text>`   record something nobody has decided yet
`/canon pending`           what's waiting on approvals
`/canon why <id>`          a rule's history and where it stands
`/canon ask <plan>`        does this clash with a rule? (uses the model)"""


# ── the app ─────────────────────────────────────────────────


def build_app():
    from slack_bolt import App

    app = App(token=os.environ["SLACK_BOT_TOKEN"])

    def post_proposal(client, cid, text, by):
        client.chat_postMessage(
            channel=CHANNEL, text=f"{by} proposed: {text}",
            blocks=proposal_blocks(cid, text, by, status_of(cid)),
        )
        mark_posted(cid)

    @app.command("/canon")
    def slash(ack, command, client, respond):
        ack()
        verb, _, rest = command.get("text", "").strip().partition(" ")
        rest = rest.strip()
        user = command["user_id"]

        if verb in ("propose", "rule") and rest:
            actor = actor_for(client, user)
            code, out, err = write("add", "--scope", SCOPE, rest, actor=actor,
                                   note=f"propose by {actor}")
            cid = first_id(out)
            if code != 0 or not cid:
                return respond(code_block(err or out))
            post_proposal(client, cid, rest, actor.removeprefix("human:"))
            return respond(f"Posted to <#{CHANNEL}>.")

        if verb == "question" and rest:
            actor = actor_for(client, user)
            code, out, err = write("question", rest, actor=actor,
                                   note=f"question by {actor}")
            return respond(code_block(out if code == 0 else err))

        if verb == "rules":
            data = canon_json("list")
            live = [c for c in data.get("commitments", [])
                    if c.get("status", {}).get("status") == "active"
                    and rest.lower() in c["text"].lower()]
            return respond("\n".join(f"• {c['text']}  `{c['id']}`" for c in live)
                           or "_no rules match_")

        if verb == "pending":
            items = canon_json("review").get("items", [])
            return respond("\n".join(
                f"• {i['text']}  `{i['id']}` — {i.get('needs', '')}" for i in items)
                or "_nothing waiting_")

        if verb == "why" and rest:
            code, out, err = canon("why", rest)
            return respond(code_block(out if code == 0 else err))

        if verb == "ask" and rest:
            respond("_reading the rules…_")
            code, out, err = check(rest)
            return respond(code_block(out or err))

        respond(HELP)

    @app.action("approve")
    def approve(ack, body, client, respond):
        ack()
        cid = body["actions"][0]["value"]
        actor = actor_for(client, body["user"]["id"])
        code, out, err = write("approve", cid, actor=actor, note=f"approve {cid} by {actor}")
        if code != 0:
            return respond(replace_original=False, text=err or out)
        by, text = parse_proposal(body["message"])
        refresh(client, body["channel"]["id"], body["message"]["ts"], cid, by, text)
        if note := unheard(actor):
            respond(replace_original=False, text=note)

    @app.action("object")
    def object_(ack, body, client):
        ack()
        cid = body["actions"][0]["value"]
        by, text = parse_proposal(body["message"])
        client.views_open(trigger_id=body["trigger_id"], view={
            "type": "modal", "callback_id": "object_reason",
            "private_metadata": json.dumps({
                "cid": cid, "channel": body["channel"]["id"], "ts": body["message"]["ts"],
                "by": by, "text": text}),
            "title": {"type": "plain_text", "text": "Object"},
            "submit": {"type": "plain_text", "text": "Object"},
            "blocks": [{"type": "input", "block_id": "why",
                        "label": {"type": "plain_text", "text": "Why? (an objection needs a reason)"},
                        "element": {"type": "plain_text_input", "action_id": "v", "multiline": True}}],
        })

    @app.view("object_reason")
    def object_reason(ack, body, view, client):
        why = view["state"]["values"]["why"]["v"]["value"]
        meta = json.loads(view["private_metadata"])
        actor = actor_for(client, body["user"]["id"])
        code, out, err = write("object", meta["cid"], "-m", why, actor=actor,
                               note=f"object {meta['cid']} by {actor}")
        if code != 0:
            return ack(response_action="errors", errors={"why": err or out})
        ack()
        refresh(client, meta["channel"], meta["ts"], meta["cid"], meta["by"], meta["text"])
        note = unheard(actor)
        standing = f" (no say in {SCOPE} yet)" if note else ""
        client.chat_postMessage(channel=meta["channel"], thread_ts=meta["ts"],
                                text=f"{actor.removeprefix('human:')} objected{standing}: {why}")
        if note:
            client.chat_postEphemeral(channel=meta["channel"], user=body["user"]["id"], text=note)

    def refresh(client, channel, ts, cid, by, text):
        """Rewrite a proposal message with where it stands now. The buttons
        stay while it is still a proposal."""
        status = status_of(cid)
        blocks = (proposal_blocks if status.lower().startswith("proposed")
                  else settled_blocks)(cid, text, by, status)
        client.chat_update(channel=channel, ts=ts, text=f"{by} proposed: {text}", blocks=blocks)

    @app.action("propose_candidate")
    def propose_candidate(ack, body, client, respond):
        ack()
        v = json.loads(body["actions"][0]["value"])
        c = json.loads(Path(v["run"]).read_text())["candidates"][v["i"]]
        actor = actor_for(client, body["user"]["id"])
        verb = ["add", "--scope", SCOPE] if c["kind"] == "rule" else ["question"]
        code, out, err = write(*verb, c["text"], actor=actor,
                               note=f"propose from notes by {actor}")
        cid = first_id(out)
        if code != 0 or not cid:
            return respond(replace_original=False, text=err or out)
        by = actor.removeprefix("human:")
        if c["kind"] == "rule":
            client.chat_update(channel=body["channel"]["id"], ts=body["message"]["ts"],
                               text=f"{by} proposed: {c['text']}",
                               blocks=proposal_blocks(cid, c["text"], by, status_of(cid)))
            mark_posted(cid)
        else:
            respond(replace_original=True, text=f"{by} recorded an open question: {c['text']}")

    @app.action("skip_candidate")
    def skip_candidate(ack, body, respond):
        ack()
        respond(replace_original=True,
                text=f"~{body['message']['blocks'][0]['text']['text'].splitlines()[1]}~ "
                     f"— <@{body['user']['id']}> says not ours")

    return app


# ── one-shot commands ───────────────────────────────────────


def client():
    from slack_sdk import WebClient
    return WebClient(token=os.environ["SLACK_BOT_TOKEN"])


def people():
    """Map every member of the house channel, then print the grants that
    give each of them a say. Printed, not run: granting is the founder's act."""
    c = client()
    for uid in c.conversations_members(channel=CHANNEL)["members"]:
        u = c.users_info(user=uid)["user"]
        if u.get("is_bot") or uid == "USLACKBOT":
            continue
        if not person_for(f"slack:{uid}"):
            enroll(f"slack:{uid}", slack_name(c, uid))
    found = load_people()
    print(f"wrote {PEOPLE} ({len(found)} people). To give each of them a say in {SCOPE}:\n")
    for name in sorted(found):
        print(f'  canon grant "human:{name}" {SCOPE} -m "founding member"')


def post_draft(run):
    run = str(Path(run).resolve())
    a = json.loads(Path(run).read_text())
    c = client()
    n = 0
    for i in a.get("kept", []):
        cand = a["candidates"][i]
        if cand["kind"] not in ("rule", "question"):
            continue
        c.chat_postMessage(channel=CHANNEL, text=cand["text"],
                           blocks=candidate_blocks(i, cand, run))
        n += 1
    print(f"posted {n} candidate(s) to {CHANNEL}")


def quotes():
    """The passage each finding came from, by its text and source, out of
    the draft runs canon keeps; the act records only the source. A finding
    edited at the review matches none and shows its source alone."""
    found = {}
    for run in sorted((CANON_DIR / "draft-runs").glob("*.json")):
        try:
            cands = json.loads(run.read_text()).get("candidates", [])
        except (OSError, ValueError) as e:
            print(f"left out the quotes in {run}: {e}", file=sys.stderr)
            continue
        found.update({(c.get("text"), c.get("source")): c["quote"]
                      for c in cands if c.get("quote")})
    return found


def post_pending():
    """Post each proposal waiting on the house that the channel has not
    seen, with Approve and Object: what was kept from an import at the
    terminal, and anything else written outside Slack."""
    seen = posted()
    todo = [p for p in canon_json("list").get("commitments", [])
            if p.get("status", {}).get("status") == "proposed" and p["id"] not in seen]
    if not todo:
        return print("nothing new is waiting on the house")
    c, cited = client(), quotes()
    c.chat_postMessage(channel=CHANNEL, text=f"{len(todo)} proposal(s) waiting on the "
                       "house. Approve or object to each one below.")
    for p in todo:
        time.sleep(1)  # Slack takes about one message a second in a channel
        by, text = p["actor"].removeprefix("human:"), p["text"]
        if src := p.get("source"):
            # In the text, not a block of its own: refresh rebuilds a post
            # from its text, so the passage stays through every click.
            q = cited.get((p["text"], src), "")
            q = q if len(q) <= QUOTE_MAX else q[:QUOTE_MAX] + "…"
            text += f"\nfound in {src}" + (f":\n```{q}```" if q else "")
        c.chat_postMessage(channel=CHANNEL, text=f"{by} proposed: {text}",
                           blocks=proposal_blocks(p["id"], text, by, status_of(p["id"])))
        mark_posted(p["id"])
    print(f"posted {len(todo)} proposal(s) to {CHANNEL}")


def digest():
    items = canon_json("review").get("items", [])
    if not items:
        return
    lines = "\n".join(f"• {i['text']} — {i.get('needs', '')}" for i in items)
    client().chat_postMessage(channel=CHANNEL, text=f"*Waiting on the house:*\n{lines}")


def main():
    if not CHANNEL:
        sys.exit("set CANON_CHANNEL to the house channel's id (C0…)")
    cmd = sys.argv[1] if len(sys.argv) > 1 else "serve"
    if cmd == "people":
        return people()
    if cmd == "post-draft" and len(sys.argv) > 2:
        return post_draft(sys.argv[2])
    if cmd == "post-pending":
        return post_pending()
    if cmd == "digest":
        return digest()
    if cmd == "serve":
        from slack_bolt.adapter.socket_mode import SocketModeHandler
        SocketModeHandler(build_app(), os.environ["SLACK_APP_TOKEN"]).start()
        return
    sys.exit(__doc__)


if __name__ == "__main__":
    main()
