# canon

**The house rules, written down once, with the reasons attached.**

Every shared house already has rules. They live in someone's memory, a
pinned message from 2023, and a document three people have read. So the
same conversation happens twice a year, a new housemate has no way to
find out what was already decided, and when a rule changes nobody can
remember why the old one existed.

`canon` is a small command-line tool that holds those rules in one file.
It records what was decided, when, by whom, and — the part that matters —
**why**. When a rule changes, the old one is not deleted; it is
superseded, and the reason is kept.

```sh
canon init --profile house
canon add "Quiet hours are 10pm to 7am."
canon add "Anyone can invite a guest for up to three nights."
canon list
```

```text
can-0bc00855477c  Quiet hours are 10pm to 7am.
can-ffc1e6e30686  Anyone can invite a guest for up to three nights.

2 rules live
```

**New here? → [GETTING_STARTED.md](./GETTING_STARTED.md)** walks a house
through its first hour, with real output at every step.

## What it does

**Keeps the reason, not just the rule.** Changing a rule records what it
replaced and why:

```sh
canon supersede can-ffc1e6e30686 \
  "Guests up to three nights; longer needs a house chat." \
  -m "Sam's cousin stayed two weeks in June and nobody knew how to raise it."
```

Six months later, `canon why <id>` answers "why is this rule like this?"
without anyone having to remember.

**Records what you decided *not* to have.** A house that decided against
a chore rota has made a decision, and it will be re-proposed every spring
unless it is written down as a decision:

```sh
canon silence "a chore rota" \
  -m "We decided in March not to have one — it would turn a kindness into a duty."
```

**Records open questions.** "Nobody has ever said who looks after the
plants" is real content. `canon question` holds it; answering one is
just superseding it with a rule.

**Nothing is destroyed and everything is revertible**, including a
revert. A contradiction you are carrying on purpose is a thing you can
record (`canon accept`), not a bug to clean up.

**One file you own.** Everything is in `.canon/acts.jsonl`, one line per
decision, append-only. It diffs, so git gives you history for free. It
greps. Leaving is deleting a directory. There is no account, no server,
and nothing leaves your machine.

## Two halves, and only one needs AI

**The record half needs nothing.** Writing rules down, changing them,
reading the history, recording questions and silences, deciding how you
decide, sharing with a housemate — all of it is plain local computation
on a text file. No model, no network, no endpoint. This is most of the
tool and it is the part a house will use daily.

**The reading half needs a model**, and it is optional:

| verb | what it does |
|---|---|
| `canon draft --from <folder>` | reads documents you already have and proposes rules, each quoting the passage it came from |
| `canon check "<proposal>"` | tells you which existing rule a proposal runs against |
| `canon tensions` | finds rules that contradict each other |

These call an OpenAI-compatible endpoint you point at — typically a model
running on your own laptop. A call is **refused unless the endpoint is on
this machine** unless you pass `--allow-remote`, and every call prints
which endpoint it used.

In a house, one person can run that half; everyone else uses the rest.

## Where to look next

- **[GETTING_STARTED.md](./GETTING_STARTED.md)** — a house's first hour.
- `canon --help` — seven verbs, the ones you need.
- `canon help all` — every verb, when you want it: standing, scopes,
  policies, horizons, drawing lots, sharing, forking.
- [SPEC.md](./SPEC.md) — the file format, released **CC0**. Adopting the
  format is not a lock-in decision; the record belongs to nobody.
- [PRIMITIVES.md](./PRIMITIVES.md) — the design argument, for anyone who
  wants to know why it is shaped this way. Ostrom's eight design
  principles are the acceptance test.

## Status — read this before you rely on it

**Early, and honest about it.** Every verb is implemented and tested
(361 tests). What has not happened yet is another group using it. You
would be among the first, and that is a real thing to know before a house
puts its rules somewhere.

Two specific cautions:

The **record half is solid** — it is plain data handling with no model in
the loop, and the file is yours in a format that outlives the tool.

The **reading half is genuinely imperfect.** `draft` and `tensions` call
a language model, and a language model misses things. Its accuracy has
been measured on two vendored documents (`fixtures/maple-house`, a house
charter, and `fixtures/des-moines-noise`, municipal code) and the scripts
to reproduce those measurements ship with the repo:

```sh
./scripts/draft-bar.sh 3                          # produce runs against your endpoint
./scripts/score-bar.sh maple-house <runs-dir>     # score them
```

Treat everything `draft` proposes as a suggestion a person reviews.
There is deliberately no `--accept-all`: you accept rules one at a time,
which is what makes setting it up the house's first governance
conversation rather than an import.

## Build

```sh
cargo build --release      # binary at target/release/canon
cargo test
```

## License

AGPL-3.0-or-later. The format specification is CC0.
