# House canon on a home server, via Slack

Housemates use `/canon` in Slack. One rootless podman container holds the
record. No open ports.

Needs: podman, Slack admin, ~30 min.

## 1. Slack app

1. [api.slack.com/apps](https://api.slack.com/apps) → Create New App → From
   a manifest → paste
   [`manifest.yaml`](../integrations/slack/manifest.yaml).
2. Install to Workspace. Copy the bot token (`xoxb-…`).
3. Basic Information → App-Level Tokens → new token, scope
   `connections:write`. Copy it (`xapp-…`).
4. `/invite @canon` in the house channel. Copy the channel ID (click the
   channel name, it's at the bottom).

## 2. Server

```sh
git clone https://github.com/alexsbryan/canon.git ~/canon
cd ~/canon/integrations/slack
cp .env.example .env    # paste tokens, channel id, your Slack full name
podman build -t canon-slack .
```

Data lives in `./data`.

## 3. Set up the house

`./house` runs canon in the container, as you.

```sh
./house init --profile house
./house people
```

`people` prints a `grant` line per channel member. Delete anyone who
shouldn't get a say, run the rest:

```sh
./house grant "human:Jo" house -m "founding member"
```

Someone who moves in later: `/invite` them, run `./house people` again,
and grant them. Until then the bot records their approvals and objections
but doesn't count them, and tells them so.

Choose how a proposal passes, before anyone proposes and before an import
(§4). Until you do, proposing is deciding. Set it for the whole canon, not
`--scope house`: what an import keeps carries no scope, and a scoped rule
would let it in at once.

```sh
./house ratification set threshold:2/1 -m "two to pass, one to stop"
```

`threshold:2/1` takes two approvals and one objection stops it.
`consent:3d` passes after three days with no objection.

Start it as a systemd user service (Quadlet):

```sh
mkdir -p ~/.config/containers/systemd
cp canon-slack.container ~/.config/containers/systemd/
systemctl --user daemon-reload
systemctl --user start canon-slack
loginctl enable-linger "$USER"    # keep running after you log out
```

Logs: `journalctl --user -u canon-slack -f`

On a Mac, or a laptop to start, there is no systemd. Install podman
(`brew install podman`, then `podman machine init` and
`podman machine start` once) and run the container yourself:

```sh
podman run -d --name canon-slack --restart=always --env-file .env \
  -v ./data:/data:Z localhost/canon-slack
```

Logs: `podman logs -f canon-slack`. The bot answers only while the
machine is awake. After a restart: `podman machine start`, then
`podman start canon-slack`.

The container is one way to run it. [Run it your way](#run-it-your-way)
says what the bot needs, for nix, launchd, a tmux pane or anything else.

## 4. Import the house's old chat (optional)

You read the archive at the terminal and keep the rules that are real; the
house then approves each one in Slack. A workspace admin can export the
channel's history (Slack takes a while to send it). Unzip it.

Install canon at the release the image pins, and give it a model in your
shell (`CANON_ENDPOINT`, `CANON_MODEL`, and `CANON_API_KEY` if it needs
one):

```sh
cd ~/canon/integrations/slack
curl -fsSL https://raw.githubusercontent.com/alexsbryan/canon/main/install.sh \
  | CANON_VERSION="$(sed -n 's/^ARG CANON_VERSION=//p' Containerfile)" sh
```

Then draft on the bot's record, as the name the bot knows you by (see
`data/house/people.json`), and post what you kept:

```sh
CANON_DIR="$PWD/data/house/.canon" CANON_ACTOR="human:Your Name" \
  canon draft --from ~/slack-export --max-chunks 20
./house post-pending
```

`draft` shows each finding with the passage it came from: `a` keeps it,
`r` drops it, `q` stops. Try 20 passages first, then drop `--max-chunks`
for the rest. `post-pending` posts each kept finding to the channel, with
its quote and Approve / Object, and skips what it has posted before.
Nothing is a rule until it passes under the rule you chose in §3.

Cloud model? It reads your chat. Agree as a house first, then add
`--allow-remote`. A local model keeps the chat on the machine and finds
fewer rules.

## 5. Use

```
/canon rules [words]      what's in force
/canon propose <rule>     post it for approval
/canon question <text>    log something undecided
/canon pending            waiting on approvals
/canon why <id>           history
/canon ask <plan>         does this clash? (needs model)
```

Weekly digest (crontab):

```sh
0 18 * * 0  ~/canon/integrations/slack/house digest
```

On a Mac, cron can't find podman: put `PATH=/opt/homebrew/bin:/usr/bin:/bin`
on its own line above.

## 6. Backup and sync

Back up `data/`. It is the whole house: the record in `data/house/.canon/`
and who is who in `data/house/people.json`.

For history and an offsite copy, keep `data/house` in a private git repo.
Make a key, add its `.pub` to the repo as a deploy key with write access,
then:

```sh
ssh-keygen -t ed25519 -N "" -f ~/.ssh/canon_deploy
cd ~/canon/integrations/slack/data/house
git init -b main && git add -A && git commit -m "the house's record"
git remote add origin git@github.com:YOU/house-canon.git
GIT_SSH_COMMAND="ssh -i ~/.ssh/canon_deploy" git push -u origin main
```

Set `CANON_GIT_SYNC=1` in `.env` and uncomment the deploy key line in
`canon-slack.container`. Each write from Slack then pulls, commits and
pushes; acts from `./house` go with the next one. If someone pushes from
their own clone, `canon merge-driver` joins their acts with the bot's. A
sync that fails says so in the log (`canon-gateway: git push failed`); the
act stays here and goes out with the next push that works.

## 7. Move to another machine

Start on a laptop and move to a server later; the house sees nothing
change. The house is `data/` and `.env`.

1. Stop the bot on the old machine (`podman stop canon-slack`,
   `systemctl --user stop canon-slack`, or however you run it). Run one
   bot at a time: Slack
   hands each event to only one of them, so two would each hear half the
   house and keep two records.
2. On the new machine, do §2 from a canon checkout that pins the same
   release or a newer one. An older canon refuses the record.
3. Copy `.env` and all of `data/` across. With git sync on, you can clone
   the record's repo into `data/house` instead, and bring the deploy key
   and `data/house/posted.json`; without that file, `post-pending` posts
   again whatever is still waiting.
4. Start the bot there (§3).

## 8. A page on the machine (optional)

`canon ui` is a page for the one person at the machine that holds the
record: what's waiting on you, proposing, changing a rule with its reason.
It listens on 127.0.0.1 only; housemates stay in Slack. Install canon as
in §4 and open it on the bot's record, as the name the bot knows you by:

```sh
cd ~/canon/integrations/slack
CANON_DIR="$PWD/data/house/.canon" CANON_ACTOR="human:Your Name" canon ui
```

Leave its Identify alone (see the caveat below). With git sync on, what
you do there goes out with the bot's next write.

## Run it your way

The bot is one process, `python3 canon_slack.py`. The container under
systemd is one way to keep it running; nix, launchd or a tmux pane work
as well. It needs:

- `canon` at the release the `Containerfile` pins, on PATH or named by
  `CANON_BIN` (install it as in §4);
- Python 3 with `slack_bolt` (`pip install -r requirements.txt`);
- one directory it can write: the record, whose `.canon` is `CANON_DIR`,
  with `people.json` and `posted.json` beside it;
- outbound HTTPS to Slack, nothing listening;
- to be the only one. Run one bot per Slack app: Slack gives each event
  to one connection, so a second bot hears half the house and keeps a
  second record.

```sh
cd ~/canon/integrations/slack
SLACK_BOT_TOKEN=xoxb-… SLACK_APP_TOKEN=xapp-… CANON_CHANNEL=C0… \
  CANON_DIR="$HOME/house/.canon" python3 canon_slack.py
```

Those four are required; without `CANON_DIR` it refuses to start.
The rest are optional: `CANON_SCOPE` (default `house`), `CANON_PEOPLE`
(default `people.json` beside the record), `CANON_GIT_SYNC` and
`CANON_ALLOW_REMOTE` as in `.env.example`, and `CANON_ENDPOINT`,
`CANON_MODEL` and `CANON_API_KEY`, which it hands to canon for
`/canon ask`. With git sync on, it pushes with your own git and ssh.

`./house` is the container's door. Without it, the one-shots take the
same environment (`python3 canon_slack.py post-pending`), and a canon
command runs with `CANON_DIR` and `CANON_ACTOR`, as in §4.

## Caveat: not signed yet

Anyone with shell on the server or push to the repo can write as anyone.
Fine if you trust whoever runs the box.

Don't run `canon identify` under a name the bot uses. It'll block the bot.

Signing is coming. You'll rebuild the image; housemates do nothing.

## Upgrade

The image installs the canon release its `Containerfile` names, so every
machine that builds from the same checkout runs the same canon. Upgrade
every machine that reads the record together: an older canon refuses a
record a newer one has written to.

```sh
cd ~/canon && git pull
cd integrations/slack && podman build --no-cache -t canon-slack .
systemctl --user restart canon-slack
```
