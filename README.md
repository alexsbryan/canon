# canon

**You already have house rules. They're just scattered across two years
of chat, a handbook nobody's opened since 2023, and someone's memory.**

`canon` reads what you already wrote and proposes the rules it finds —
each one quoting the passage it came from, so you can check it. You go
through them together and keep the ones that are real. That review is
the point: it's usually the first time a house has seen its own rules in
one list.

```sh
canon init --profile house
canon draft --from ~/house-stuff     # a folder. anything text in it.
```

Then you review, one at a time. There is deliberately **no
`--accept-all`** — a canon adopted wholesale is one nobody has read.

**New here? → [GETTING_STARTED.md](./GETTING_STARTED.md).**

## Point it at the mess

**There is no format list.** Anything under that folder which is text
gets read, whatever it's called — `.org`, `.eml`, a `NOTES` file with no
extension, a transcript someone pasted into a `.log`, a Slack export.
A house writes in whatever it writes in.

**Chat is read as chat.** Messages are rendered with who said them and
cut into bursts on a time gap, so a rule found in a channel cites the
exchange it was actually decided in:

```text
RULE      Recycling goes out Sunday night.
          slack-export/general/2026-08-01.json:1-12

  > sam: reminder the recycling goes out sunday night not monday
  > mira: ^ this has bitten us three times now, can we make it a rule
  > sam: yes. recycling out sunday night.
```

**It tells you what it didn't read**, and why — how many files and for
what reason. Three things a walk passes over, each reported and each with
a way round it: files your own `.gitignore` calls generated
(`--include-ignored` reads them anyway), structured data that holds no
conversation (a lockfile read as prose proposes rules cited to dependency
names), and anything too big to be writing. Naming a file directly reads
it regardless.

**A feed you read twice doesn't ask twice.** `.canon/seen` records which
passages were already extracted from and which proposals you turned down,
so re-pointing at a growing channel costs you the new material, and a
rule you rejected isn't proposed again tomorrow. `[s]kip` records
nothing — skip means *not now*; only `[r]eject` means no.

**`--from -` reads stdin**, which is the whole integration surface.
Anything that can emit text can feed a canon — a ticket export, a
transcript, a system nobody has heard of — and canon carries no
connector, no vendor schema and no endpoint of its own.

```sh
canon draft --from ~/house-docs          # a folder, recursively
canon draft --from-git --since 1y        # or your commit messages
cat transcript | canon draft --from - --as '#house' --dry-run
canon draft --resume                     # finish a long review later
```

## Three kinds, because a group's rules are three shapes

An extractor that could only find rules hands back a list, not a canon.
Meeting notes saying *"nobody has ever said who looks after the
allotment"* is a **question**. *"Decided not to make a rota — it would
turn a kindness into a duty"* is a **silence**: a thing you decided not
to have, which is why it keeps getting re-proposed every spring unless
it's written down.

**Every proposal carries the passage it came from, or it isn't shown.**
The model answers with the position of the sentence it read and canon
cuts the quote out of your file itself — so a citation that isn't in your
document is not a thing that can happen. A drafted rule with no citation
is a model inventing a value you never held.

## Then it keeps the reason

Once rules exist, the everyday half needs no AI at all. Changing a rule
records what it replaced and why:

```sh
canon supersede can-ffc1 "Guests up to three nights; longer needs a house chat." \
  -m "Sam's cousin stayed two weeks in June and nobody knew how to raise it."

canon why can-e7ab     # six months later: why is this rule like this?
```

Nothing is destroyed, everything is revertible including a revert, and a
contradiction you're carrying on purpose is something you can record
(`canon accept`) rather than a bug to clean up.

**One file you own.** Everything is `.canon/acts.jsonl`, one line per
decision, append-only. It diffs, so git gives you history free. It greps.
Leaving is deleting a directory. No account, no server, nothing leaves
your machine.

## What needs a model, and what doesn't

**Needs an endpoint:** `draft`, `check`, `tensions`, `rebase`. Point at
any OpenAI-compatible server, normally one on your own laptop. A call is
**refused unless the endpoint is on this machine** unless you pass
`--allow-remote`, and every call prints which endpoint it used.

**Needs nothing:** everything else — `add`, `list`, `why`, `supersede`,
`retract`, `accept`, `question`, `open`, `silence`, `undo`, `log`,
`share`, `adopt`, `diff`, and every governance verb (`who`, `grant`,
`scope`, `policy`, `position`, `decide`, `rank`, `horizon`, `overdue`,
`voice`, `draw`, `replay`). That's most of the tool and all of the daily
use. In a house, one person runs the model half; everyone else needs
nothing.

## Status — read this before you rely on it

**Early. Another group has not used this yet.** You would be among the
first, which is a real thing to know before a house puts its rules
somewhere. Every verb is implemented and tested (361 tests).

**The ingest is the good part and the imperfect part.** It calls a
language model, and a language model misses real rules and proposes
things that aren't rules. That's why review is one-at-a-time and why
every proposal has to cite its source — the design assumes the model is
wrong some of the time and makes that cheap to catch.

Accuracy is measured against two vendored documents — a house charter
(`fixtures/maple-house`) and municipal code
(`fixtures/des-moines-noise`) — and the scripts ship here so you can
measure it against your own:

```sh
./scripts/draft-bar.sh 3                        # runs against your endpoint
./scripts/score-bar.sh maple-house <runs-dir>   # score them
```

Numbers are deliberately not quoted in this README: the last published
ones predate a change to how contradictions are detected, and a stale
accuracy figure is worse than none.

## More

- **[GETTING_STARTED.md](./GETTING_STARTED.md)** — a house's first hour.
- `canon --help` — seven verbs · `canon help all` — all of them.
- [SPEC.md](./SPEC.md) — the file format, **CC0**. Adopting the format
  is not a lock-in decision.
- [PRIMITIVES.md](./PRIMITIVES.md) — the design argument. Ostrom's eight
  design principles are the acceptance test.

## Build

```sh
cargo build --release      # binary at target/release/canon
cargo test
```

AGPL-3.0-or-later. The format specification is CC0.
