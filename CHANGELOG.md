# Changelog

What changed in each release of canon, newest first. A change you would notice
lands here in the commit that makes it, under Unreleased. Cutting a release
renames that heading to the version, and its section becomes the release notes
([Releasing](./docs/CONTRIBUTING.md#releasing)).

## Unreleased

### Before you upgrade

- A canon that adopts `share` at any bar other than more than half or all
  of them needs this release to read it. 0.4.0 refuses the `ratification`
  line and names the rule (`unknown variant \`share\``) rather than fold
  under the wrong one. `share:>1/2` is written as `majority` and
  `share:100%` as `unanimous`, so those two still read in 0.4.0.
- Under `majority`, a refusal no longer names one person. 0.4.0 credited it
  to whoever's no came last ("refused by Kai: …"); it now says how many of
  the house said no or left, and gives each reason with its author. The
  verdict and its moment are unchanged.
- For code built on `canon-core`: `Ratify` has a `Share` variant, so an
  exhaustive `match` on it needs one more arm.

### Changes

- `canon ratification set share:60%` (or `share:2/3`, or `'share:>2/3'`
  for more than) carries a proposal at that share of the people who held the
  scope when it was written, counted as `majority` is: somebody who joins
  later never moves the count, and once the share can no longer be reached
  it is refused. Two-thirds of three is two. An impossible bar (`share:0%`,
  `'share:>100%'`, `share:3/0`) is refused and says why.
- A refusal that nobody made alone reads as one: `refused 2026-10-17: 13 of
  the 30 people who held house when it was proposed said no or no longer
  hold it, so 60% can no longer approve it — Jo: …; Sam: …`, in `why`,
  `list`, `review` and `canon ui`, where it used to read `refused by :`.

## 0.4.0 — 2026-10-17

### Before you upgrade

- A canon that adopts `majority` or `unanimous` needs this release to read
  it. Older canon refuses the `ratification` line and names the rule it
  does not know, rather than fold every later decision under the wrong
  rule. Every canon that doesn't adopt one reads exactly as before. Upgrade
  everyone who reads a canon before anyone in it sets one.
- For code built on `canon-core`: `Ratify` has `Majority` and `Unanimous`
  variants, so an exhaustive `match` on it needs two more arms.

### Changes

- Two ratification rules count a share of the house as it stands when each
  proposal is written: `canon ratification set majority` (more than half of
  whoever holds the scope then) and `canon ratification set unanimous`
  (all of them). Somebody who joins later never moves the count. Once the
  share can no longer be reached, because enough of them said no or no
  longer hold the scope, the proposal is refused and says why, so a house
  that has shrunk can still decide, its rule of rules included. Under
  `threshold:4/1`, a house of four that one person leaves can never carry
  anything again.

- [Self-hosting](./docs/SELF_HOST.md#run-it-your-way) says what the Slack
  bot needs to run without the container (nix, launchd, a tmux pane): the
  pinned canon, Python with `slack_bolt`, the environment it reads, one
  writable directory, outbound HTTPS, and only one running per Slack app.

- `./house post-pending` posts each proposal the channel hasn't seen, with
  Approve / Object, and with the passage a finding was drawn from. The
  import in [Self-hosting](./docs/SELF_HOST.md) is now: review the archive
  with `canon draft` at your terminal, then post what you kept for the
  house to approve. The guide sets the house's rule for the whole canon,
  because a rule set on `house` alone let whatever the import kept in at
  once.

- The Slack bot refuses to start without `CANON_DIR`, and says so. It used
  to fall back to the container's `/data/house/.canon`, so a bot run
  outside the container started with no record and failed on every
  command.

- [Self-hosting](./docs/SELF_HOST.md) covers starting on a Mac or a laptop,
  moving the house to another machine, and `canon ui` on the bot's record
  for whoever runs it.

- The Slack bot tells someone without a say that their approval or
  objection was recorded but doesn't count, and how to get one. It used to
  change nothing and say nothing, and an objection showed in the thread as
  if it counted.

- The Slack bot's git sync (`CANON_GIT_SYNC=1`) works in its container,
  and says when it does not. The image had no ssh and no host key, so
  every pull and push failed; it had no git identity, so a rebase stopped
  halfway; and a housemate's push left conflict markers in the act log,
  after which every `/canon` command failed. The bot now joins the two
  logs with `canon merge-driver`, takes the remote's host key on the
  first push, logs a failed step, and commits `people.json` with the record.

- The Slack image installs the release its `Containerfile` pins (0.3.0)
  instead of "latest". A cached build layer could keep an old canon that
  refuses a record a newer one founded, so a house moved to a second
  machine stopped reading.

## 0.3.0 — 2026-10-16

### Before you upgrade

- Nothing changes for a canon in which nobody runs `canon identify`: it
  keeps writing format version 2 and reads exactly as before. Once someone
  identifies, the file holds version 3 lines, which older canon refuses.
  Upgrade everyone who reads a canon before anyone in it identifies.
- For code built on `canon-core`: `ActKind` has a `Key` variant, so an
  exhaustive `match` on it needs an arm. `Act` carries `refs` and `sig`,
  and `Canon` carries `unverified`: acts under a name that has bound a key
  which its key did not sign, kept and not applied.
- **A canon founded by this release needs this release to read it.**
  `canon init` now writes a first act that turns on two governance knobs,
  and that line is format version 4, which older canon refuses with "upgrade
  canon rather than reading it partially". So does any `ratification set`,
  any `policy set --graduated`, and any `decide --citing`. Every canon
  written before reads exactly as it did. `canon init --plain` founds one an
  older build can still open. Upgrade everyone who reads a canon before
  anyone founds or changes one with this release.
- For code built on `canon-core`: `ActKind::Ratification` gains `governs`
  and `personal`, `ActKind::Decided` gains `citing`, and
  `Rule::Graduated` gains `key` and `repair`, so an exhaustive pattern on
  any of them needs `..` or the new fields. `Canon` carries
  `pending_policies`.

### Changes

- A change of policy can wait for the group. Under a ratification rule with
  `governs: all` — what `ratification set` and `canon init` now write — one
  holder can no longer switch a house that decides by consent to something
  else alone: `policy set` says PROPOSED, and `approve` and `object` take
  the policy's id. `--governs commitments` keeps the old shape.
- A person's own scope is theirs. With `personal: on` on the canon-wide
  rule, `human:phylla.glasses` is Phylla's alone: her rule there is in force
  on her word, anyone else's waits for her, and nobody else can take
  standing over it. A personal corner never counts as a seat in the house.
- A graduated ladder forgets on repair. `decide --citing <rule>` records the
  rule a decision turned on; a ladder keyed on it is not restarted by
  rewording, and a decision that went for closes the episode, so the next
  person does not inherit a resolved one. `policy set --graduated` writes
  both; `--by-about --no-repair` keeps the old ladder.
- `canon second <question>` adds your weight to a question without your
  name. `canon open` shows the count. Seconds decide nothing; only a person
  can second.
- A house can run canon through Slack without anyone installing it:
  `integrations/slack/` is one rootless podman container, and
  [docs/SELF_HOST.md](./docs/SELF_HOST.md) walks through it. Housemates
  propose, approve and object with `/canon` and buttons; acts from Slack
  are not signed yet.

- `canon review` walks you through what's waiting on you, one proposal at
  a time and one room at a time: the words, who proposed them and when, the
  source passage they came from, what they'd replace and why, and who has
  to decide. A long proposal shows its first 20 lines and how many there
  are, and `v` reads all of it in your `$PAGER` (`less` if unset).
  Approve, object with a reason, or skip. It shows what your
  approval or objection would actually change. When nobody holds a room
  yet, it asks once whether you'll take it on: yes is the same grant
  `canon grant <you> <room>` writes, and no skips that room's proposals.
  `--scope <room>` reviews one room and the rooms inside it. It says first
  who you're acting as and where that name came from (`CANON_ACTOR`, or
  your git name). It ends with how many you approved, objected to and
  skipped, and exits 1 if a decision couldn't be written. It's a person's
  verb: without a terminal, or with an actor that isn't `human:`, it
  refuses in one line and writes nothing. `canon review --json` lists
  what's waiting, for anyone.
- `canon ui` is canon as a page in your browser. Decide what's waiting on
  you, propose a rule, change a rule's wording or take it away, leave
  something unwritten on purpose or lift it, and identify yourself, all
  without the command line. A proposed change is shown marked up against
  the wording it replaces. It runs on this machine only, for as long as you
  leave it open, and loads nothing from the internet.
- You can make it so only your key writes as you. `canon identify
  <key.pub>` binds an SSH key to your name; after that, an act under your
  name counts only if your key signed it, and anything else under it, such
  as a line written with `CANON_ACTOR` set to you, is kept and marked as not
  counting in `list` and `log`. Run it again to see who you are. canon finds
  your key in `~/.ssh` or your ssh-agent, so there is nothing to configure,
  and it never holds a private key: `ssh-keygen` signs. A name binds once.
  Nothing changes for anyone who doesn't identify.
- Signed lines and key bindings are format version 3. Older canon refuses a
  file that has any, so upgrade everyone who reads a canon before anyone in
  it binds a key. A canon where nobody binds keeps writing version 2.
- `CANON_API_KEY` is sent as a bearer token, so canon can use a hosted
  endpoint that requires a key. It's read from the environment only; `config
  show` says whether it's set without printing it.
- Claude works: an endpoint at `api.anthropic.com` is spoken to in
  Anthropic's own API, which enforces the schema. Anthropic's "overloaded"
  (529) is waited out like a 429.
- The core compiles with `no_std` + `alloc`. Local and CI gates check its
  isolated runtime dependencies and features, alongside the workspace checks.
- [The contract](./docs/CONTRACT.md) names the guarantees, executable checks,
  and assumptions. Generated replay, merge, and ratification checks shrink
  failures and preserve regression seeds.
- Compatibility is computed against a pinned git baseline using its tests,
  fixtures, expectations, generated old consumers, and old-record comparisons.
  CI runs the evaluator from the PR base branch. Reports distinguish preservation,
  extension, observed breaks, and evidence that is not established; contributors
  link the report rather than choose a classification.

## 0.2.0 — 2026-10-03

### Before you upgrade

Two changes alter what an existing canon says. A canon where only people
wrote rules, and nobody retracted a rule they had no seat over, reads the same
as before.

- An agent's write into a scope nobody holds is a proposal now, not a rule.
  Any person's `canon approve <id>` makes it one. Rules an agent wrote before
  anyone was granted anything will show in `canon list` as proposed.
- A retraction by someone without a seat over the rule is a proposal now,
  where it used to be refused. It waits for approval from someone who could
  have retracted the rule themselves, so retractions refused before will show
  in `canon list` as waiting.

### Founding and governing a canon

- `canon <verb> --help` and `canon help <verb>` say what a verb does and what
  it takes. `canon init --help` says what it writes.
- `canon who` with no scope says who holds the canon as a whole, and a
  proposal waiting on them names the scopes that count: "holds this canon by
  holding arch or procedure". There is no scope named for the canon itself.
- `canon who <scope>` for a scope nothing uses says so and lists the scopes in
  use, rather than only suggesting you grant it.
- `canon adopt --paste --except <id>,… -m "<why>"` leaves rules behind as you
  fork. They are recorded as retracted, with your reason.
- `canon retract`, `canon why` and `canon approve` take the id a rule had
  upstream, as the pasted block printed it.
- `canon approve` works on a proposed change to how a scope decides. It used
  to answer its own hint with "no commitment matching".
- A refused act leads with NOT APPLIED and names the act. An act that took
  only because nobody held the scope yet says `applied while open`.
- `canon grant` names who granted, and says when you seat someone else while
  holding nothing yourself.
- `canon who <scope>` names both rules: how rules are made there, and how
  proposals are judged.
- `canon list` counts acts with no person behind them by kind, instead of
  printing every id on one line, and no longer counts an agent placing its own
  proposal in a scope.
- `canon init` adds `config` to `.canon/.gitignore`, so a new canon does not
  commit one machine's endpoint.

### Reading a canon without canon

- `canon list --markdown` renders what is in force as a document, grouped by
  scope with ranked rules first. `--out <file>` writes it, and `--check` exits
  1 when that file is stale. It carries no date, so a committed copy goes
  stale only when the canon changes.
- `canon list --json` carries `scope` and `rank` on each commitment.
- Over MCP, `canon_list` shows counts by scope first once a canon passes fifty
  rules, and takes `scope`, `query`, `limit` and `offset`.

### Drafting

- `canon draft --continue <run.json>` finishes a run that failed after
  extraction, from its recorded candidates, without reading a passage again.
- The number check can no longer run until the endpoint times out. Schemas go
  to the model in the order they are written; sent sorted, the model could
  fill a field with a copy of the prompt. Each reading is also capped, a block
  that fails is retried one pair at a time, and pairs it could not read are
  listed as `support_unchecked` for review.
- `canon draft --resume` offers every finished run with candidates left,
  oldest first.
- Markdown emphasis comes out of a candidate's text, and code symbols stay
  whole.
- Progress overwrites one line in a terminal and prints plain lines to a log.
- A chat export with timestamps, or a git commit, records in the run when each
  passage was said. Accepting a rule still writes its act at review time.

## 0.1.0 — 2026-09-08

The first release: one `canon` binary for macOS, Linux and Windows, and
`install.sh` to put it on your PATH.
