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

Require a second person to approve a rule (otherwise proposing = deciding):

```sh
./house ratification set threshold:2/1 --scope house -m "two to pass, one to stop"
```

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

## 4. Import old notes (optional)

Needs a model: set `CANON_ENDPOINT` and `CANON_MODEL` in `.env`. Put a
handbook or unzipped Slack export in `data/import/`.

```sh
./house draft --from /data/import --dry-run --yes
./house post-draft /data/house/.canon/draft-runs/<number>.json
```

Each find gets posted with **Propose this** / **Not ours**. Nothing becomes
a rule until the house approves it.

Cloud model? It'll see your chat. Agree as a house, then set
`CANON_ALLOW_REMOTE=1` and add `--allow-remote`.

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

1. Stop the bot on the old machine (`podman stop canon-slack`, or
   `systemctl --user stop canon-slack`). Run one bot at a time: Slack
   hands each event to only one of them, so two would each hear half the
   house and keep two records.
2. On the new machine, do §2 from a canon checkout that pins the same
   release or a newer one. An older canon refuses the record.
3. Copy `.env` and all of `data/` across. With git sync on, you can clone
   the record's repo into `data/house` instead, and bring the deploy key.
4. Start the bot there (§3).

## 8. A page on the machine (optional)

`canon ui` is a page for the one person at the machine that holds the
record: what's waiting on you, proposing, changing a rule with its reason.
It listens on 127.0.0.1 only; housemates stay in Slack. Install canon at
the release the image pins and open it on the bot's record, as the name
the bot knows you by (see `data/house/people.json`):

```sh
cd ~/canon/integrations/slack
curl -fsSL https://raw.githubusercontent.com/alexsbryan/canon/main/install.sh \
  | CANON_VERSION="$(sed -n 's/^ARG CANON_VERSION=//p' Containerfile)" sh
CANON_DIR="$PWD/data/house/.canon" CANON_ACTOR="human:Your Name" canon ui
```

Leave its Identify alone (see the caveat below). With git sync on, what
you do there goes out with the bot's next write.

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
