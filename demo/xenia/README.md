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
node src/run.ts run <canon>/demo/xenia/scenes --runs 3 --model deepseek-flash \
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
names another, built from canon `0f15ca8`. The first set used
`claude-sonnet-5-5`; the next uses `deepseek-flash`, which the plants have not
admitted, so the set's plants are where that shows. The first set ran once;
the next runs three, so every plant's admission and every scene's rate are
N=3.

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
  much; they do not claim `wordy` for a terminal. This is named, not hidden.
- The browser's reach filter covers the page. The CLI's own network access is
  the product's, as an app server's is; `first-hour` uses it for the installer.
- The first set's findings drew their product source from the page's first read
  (`Command line`), not from canon. The next set's xenia takes a soft finding's
  source from the person's own quoted words instead; a hard fault's stays the
  screen or element it names.

One stage per host at a time, as the design says. The scenes bind to
`127.0.0.1` and no one outside can reach them.
