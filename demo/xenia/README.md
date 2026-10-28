# Strangers through canon's CLI

xenia sends a model playing one person through canon's terminal, told only
what that person was told. This is canon's set of scenes for it. A rehearsal
points to where people will stumble; it is never a people result.

The stories come from the guide, not from the tests: the first hour
([GETTING_STARTED](../../docs/GETTING_STARTED.md)), a housemate reading a
shared canon ([GUIDE, "Someone else's canon"](../../docs/GUIDE.md)), the model
half of setup (GETTING_STARTED steps 2 and 3), and an exploration of the
everyday verbs ([GUIDE, "The verbs"](../../docs/GUIDE.md)). Each scene has one
person, one house, one terminal.

## Run the set

From xenia's checkout, with these paths in place of `<canon>` and `<xenia>`:

```sh
source ~/.nvm/nvm.sh && nvm use
cd <xenia>
node src/run.ts run <canon>/demo/xenia/scenes --runs 1 --model deepseek-flash \
  --findings <canon>/demo/xenia/findings --source <canon>
```

The draft scenes need the reference model server: a local OpenAI-compatible
endpoint at `http://localhost:9741/v1` that answers to `primary`. The other
scenes need nothing but the network for the installer, in `first-hour`.

xenia runs its own plants first, then each scene `--runs` times, and writes
the accounts under `runs/` where you run it, with a summary. `--findings`
keeps what went wrong, one file per finding, updated by each set that sees it
again. `--runs 1` is a rehearsal; one run is never a rate.

The canon binary audited is `<canon>/target/debug/canon` unless `CANON_BIN`
names another, built from canon `963e1cd`. The first set used
`claude-sonnet-5-5`; the second used `deepseek-flash`, whose plants came back
as they must in 15 of 17, so it is not admitted and the set is
could-not-judge: the plants are where an admission shows. The operator cut
each set to one run (2026-10-28): three runs of the plants is too many
sessions for the learning rate. What accumulates is sets, and one run is
still never a rate.

## The scenes

Each file in `scenes/` is one scene: it stands up a world in a fresh temp
directory, prints xenia's scene line, and keeps the terminal up until xenia
closes its stdin. A scene is one person and one story.

- **`first-hour`** — canon is not installed. The person's own note says they
  are setting it up; its README is in `~/canon-README.md`. They install it,
  start a canon, add the quiet-hours rule, and leave the list showing. The
  installer is canon's own, from the release.
- **`join`** — canon is installed and a housemate's share block is in
  `~/share.txt`. The person starts a canon and reads the block into it.
- **`draft`** — canon is installed and the house's chat export is in
  `~/house-notes`. Sam tells the person where the model is; the person points
  canon at it, reads the notes, reviews what it proposes, and lists what is
  live.
- **`explore`** — the house's canon is already written, with three rules, a
  supersession, a question and a silence. The person has a play and says what
  they think it is for. This is xenia's exploration: no goal, and the named
  screens are its milestones.

The people are `people/mira.md`, `people/sam.md` and `people/june.md`: a
steward who does not write code, a developer who does, and a new housemate who
is careful with computers. They are a first cut, and the operator may rewrite
them; a changed person is a new person, so the accounts on either side of a
rewrite do not compare.

## The first set

The set of 2026-10-28, one run by `claude-sonnet-5-5`, is in `findings/`.
Its 17 plants came back as they must. Of the 12 accounts, 4 passed: the three
explorations and Sam's draft. The setup scenes stopped on the install refusal
(a chat message asking for `curl | sh` is not acted on) and on the reading
budget; the drafts and explorations carry the product findings. One run is
not a rate.

## The second set

The set of 2026-10-28, one run by `deepseek-flash`, is in `findings/` with
the first set's still-open findings. Its plants came back as they must in 15
of 17: p4-relay's Ana gave up after `back` off the shed left her on a blank
page, and p10-explorer was never asked what it is for. Of the 12 accounts, 8
passed, 2 failed and 2 were could-not-judge. The two first-hours whose
stranger asked the operator for permission ("if you want me to…") are the
in-character check's first real catch: the instrument had no person there,
and the account says so instead of failing the person. The two failures are
the model's too: June's draft ended without acting, and Sam's draft spent its
patience. 27 findings are open, 14 of them from this set.

What the accounts say about canon, for the next round:

- Sam's exploration had `canon add "Guests may stay a month"` come back
  "in force" with nobody holding the canon: no ratification, no reason,
  beside the three-nights rule, where the help says a proposal waits for its
  scope. The log's later `ratify standing` act is there, so what's wrong is
  the headline, not a hidden write.
- June and Sam both found that `canon check --about "a chore rotation"` —
  the form the tool's own output suggests — is rejected; the usage is
  `canon check "<proposal>"`. And `check` cannot run at all with no model
  endpoint, while `config show` says "nothing configured" and exits 0.
- `canon help all`'s pager swallowed the commands typed next (the line sat
  unrun until Enter). The help is long: Sam read 1,038 words before acting on
  a 300-word budget.
- The hard `inert` faults — five taps on "Command line" and one on an empty
  "Enter" — are the terminal page's, not canon's: the box is already
  focused, and a tap on its label changes nothing the stage can see.

## The terminal is real; the keyboard is the page

canon is a terminal program and xenia's stage reads a browser. The bridge is a
real shell in a PTY, rendered as a terminal page:

- The shell is `/bin/bash` in a PTY (Python's `pty` module; macOS's `script`
  refuses a piped stdin). canon, the installer, the model client and the rest
  run exactly as they would in a terminal.
- The page shows the output as text. `look` reads it, `type` fills the command
  line, `tap` presses Enter. The `Ctrl-C` and `Ctrl-D` buttons are the two
  keys a story might need. ANSI colour is stripped, so a cue that is only
  colour is invisible.
- The screen a check sees is the last completed output line
  (`document.body.dataset.last`), not the whole scrollback: a terminal's
  current state is its latest output. The accounts' per-screen counts are that
  approximation.
- **Words before acting is a form measure.** The command line and Enter are the
  most prominent things on the page, so the hard `wordy` fault's target is read
  before the output a person actually read to choose the command. The scenes
  report words read and patience, and the soft findings carry what was too
  much; this is named, not hidden. The second set's one `wordy` fault is
  Sam's `canon help all` (1,038 words read against a 300-word budget): read
  it as the help-length signal, not as a rate.
- The browser's reach filter covers the page. The CLI's own network access is
  the product's, as an app server's is; `first-hour` uses it for the installer.
- A tap on the page's own chrome is not canon. "Command line" is a label over
  an already-focused box, and "Enter" with an empty box does nothing: the
  `inert` faults that name them are the page's. The stage's change counter
  does not see a focus change, so a tap that only focuses says nothing
  changed; a person who meant to type is not blocked by it.
- The first set's findings drew every product source from the page's first
  read (`Command line`), not from canon. Since the second set, a soft
  finding's source is the phrase the person put in quotes; a hard fault's
  stays the screen or element it names. A soft finding whose words quote no
  source verbatim says "None found" rather than falling back to the chrome,
  and a short quoted phrase can match a line that is not the product's (one
  cites the LICENSE): the phrase is the person's, and the match is a floor.

One stage per host at a time, as the design says. The scenes bind to
`127.0.0.1` and no one outside can reach them.
