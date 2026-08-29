# Getting started — a house's first hour

For a shared house, coliving community, or any group that has rules and
has never written them down in one place.

You need a terminal and about an hour. Most of that hour is the
conversation, not the tool. Every block of output below is real.

---

## 1. Install

```sh
git clone <this repo> && cd canon
cargo build --release
```

The binary is `target/release/canon`. Copy it somewhere on your `PATH`,
or run it by full path. If you don't have Rust, `rustup.rs` is the
one-line installer.

## 2. Start a canon

Pick where it lives — a folder in your house's shared drive, a git repo,
or just a directory on someone's laptop to begin with.

```sh
canon init --profile house
```

```text
canon initialised at /home/mira/house/.canon (house)

  canon add "<a rule you already have>"      write one down
  canon check "<something you want to do>"   does it clash with one?
  canon why <id>                             what replaced what, and why

that is the whole thing. `canon help all` when you want the rest —
who decides what, how you decide, what has gone stale, drawing lots.
```

`--profile house` matters: it makes the tool say **rule** instead of
*commitment*, and phrases its answers as *which conversation this needs*
rather than as a verdict. (`personal` and `code` are the other two.)

> **One setup note.** Don't set the `CANON_ACTOR` environment variable.
> Left alone, canon records acts as `human:<your git name>`. Setting it
> to a bare name marks your decisions as *machine-written* and you'll see
> `warning: 1 adjudication(s) were not authored by a person`. That
> warning exists so automation can't quietly write your house's rules.

## 3. Write down what you already have

Do this together, out loud, in one sitting. Ten or fifteen rules is a
realistic first pass.

```sh
canon add "Quiet hours are 10pm to 7am."
canon add "Anyone can invite a guest for up to three nights."
canon add "Whoever cooks does not wash up."
```

```text
can-0bc00855477c  Quiet hours are 10pm to 7am.
can-ffc1e6e30686  Anyone can invite a guest for up to three nights.
can-3664f149e633  Whoever cooks does not wash up.
```

```sh
canon list
```

```text
can-0bc00855477c  Quiet hours are 10pm to 7am.
can-3664f149e633  Whoever cooks does not wash up.
can-ffc1e6e30686  Anyone can invite a guest for up to three nights.

3 rules live
```

Those `can-…` ids are how you refer to a rule later. You only ever need
enough of one to be unambiguous.

## 4. Change a rule — with the reason

This is the point of the whole tool. Don't edit; **supersede**.

```sh
canon supersede can-ffc1e6e30686 \
  "Anyone can invite a guest for up to three nights; longer needs a house chat." \
  -m "Sam's cousin stayed two weeks in June and nobody knew how to raise it."
```

```text
can-e7ab38908043  Anyone can invite a guest for up to three nights; longer needs a house chat.
  replaces can-ffc1e6e30686
```

Now the history is answerable:

```sh
canon why can-e7ab38908043
```

```text
can-e7ab38908043  Anyone can invite a guest for up to three nights; longer needs a house chat.
  asserted 2026-08-29 by mira
  reason for the change: Sam's cousin stayed two weeks in June and nobody knew how to raise it.
  replaced can-ffc1e6e30686: "Anyone can invite a guest for up to three nights."
  status: in force
```

A year later, when someone asks "why do we have this rule about guests?",
that is the answer, and nobody had to remember it.

The `-m` reason is not optional politeness. A rule with no reason cannot
be revisited well — you can't tell whether the thing it was protecting
against still exists.

## 5. Record the questions and the deliberate gaps

Two kinds of thing that usually get lost.

**An open question** — something nobody has decided:

```sh
canon question "Who looks after the plants when everyone travels at once?"
```

```text
can-244a9a259afd  ? Who looks after the plants when everyone travels at once?
  answer it:  canon supersede can-244a9a259afd "<the rule>" -m "<reason>"
```

```sh
canon open
```

```text
can-244a9a259afd  ? Who looks after the plants when everyone travels at once?

1 open
```

**A silence** — something you decided *not* to have a rule about. This
is the one most houses lose, and it is why the same proposal comes back
every year:

```sh
canon silence "a chore rota" \
  -m "We decided in March not to have one — it would turn a kindness into a duty."
```

```text
can-66fcda050a09  unwritten on purpose: "a chore rota"
  We decided in March not to have one — it would turn a kindness into a duty.
  `canon check --about "a chore rota"` will say so rather than call it a gap
```

## 6. Decide how you decide

Optional, and worth doing once the house has used the tool for a few
weeks. Until you set a policy, canon holds no opinion about how many
objections stop a thing — it names the rule you are up against and stops.

```sh
canon policy set consent -m "One reasoned objection stops a thing. Anything we cannot undo is not decided by silence."
```

The available rules are `default`, `consent`, `threshold`,
`supermajority --of 2/3`, and `subsidiarity`. The policy is itself a
recorded act, so it can be questioned, superseded and explained like any
rule. `canon policy show` says what you're on.

Related, when you want it: `canon grant` gives someone standing over a
scope, `canon who <scope>` answers who may decide something without
asking a person, and `canon overdue` lists what has passed a review date
you set with `canon horizon`.

## 7. Share it with the house

```sh
canon share            # prints a block you can paste into your group chat
canon adopt --paste    # a housemate reads it back on their machine
canon diff --upstream  # how your copy has diverged from what you adopted
```

```text
--- canon housedemo · house · snapshot 2026-08-29 · 20a846
Quiet hours are 10pm to 7am.  (can-0bc00855477c)
Whoever cooks does not wash up.  (can-3664f149e633)
Anyone can invite a guest for up to three nights; longer needs a house chat.  (can-e7ab38908043)
--- 3 live · adopt: canon adopt --paste
```

Pasting is a real answer, not a stopgap. A shared snapshot carries the
current rules and drops the rationales — enough to adopt, not enough to
audit, which is the right trade for a chat thread. A block edited after
it was sent is refused rather than adopted under the sender's name.

If your canon lives in a git repo, everyone just pulls it, and canon
ships a merge driver so two people adding rules on the same day is not a
conflict:

```sh
canon merge-driver     # run with no arguments for setup instructions
```

---

## Optional: let it read your existing documents

If you already have a handbook, a year of meeting notes, or a Slack
export, `canon draft` will read them and propose rules — each one quoting
the passage it came from.

**This part needs a language model.** Point canon at any
OpenAI-compatible endpoint, normally one running on your own machine
(llama.cpp, Ollama, LM Studio, or similar):

```sh
canon config set endpoint http://localhost:8080/v1
canon draft --from ~/house-docs
```

Then review what it found, one at a time. There is no "accept all" — on
purpose. Reviewing the proposals together *is* your first governance
conversation, and a canon adopted wholesale is one nobody has read.
`canon draft --resume` picks a long review back up later without a second
model run.

Point it at a folder and it reads anything under it that is text,
whatever the extension. It tells you what it skipped and why. It also
remembers what you already reviewed, so re-pointing it at a growing
channel costs you the new material, and a rule you rejected is not
proposed again tomorrow.

**Judge every proposal yourself.** The model misses real rules and
proposes things that aren't rules. Its accuracy is measured in this repo
against two vendored documents, and the scripts are here if you want to
check it against your own:

```sh
./scripts/draft-bar.sh 3
./scripts/score-bar.sh maple-house <runs-dir>
```

Once you have rules, two more model-backed verbs become useful:

```sh
canon check "convert the spare room into a studio"   # what does this run against?
canon tensions                                       # which of our rules conflict?
```

---

## The short version

| you want to | run |
|---|---|
| start | `canon init --profile house` |
| write a rule down | `canon add "<rule>"` |
| see what's live | `canon list` |
| change one | `canon supersede <id> "<new>" -m "<why>"` |
| find out why | `canon why <id>` |
| drop one | `canon retract <id> -m "<why>"` |
| note an open question | `canon question "<question>"` |
| note a deliberate gap | `canon silence "<subject>" -m "<why>"` |
| undo anything | `canon undo <act-id> -m "<why>"` |
| share | `canon share` |
| everything else | `canon help all` |

Nothing is destroyed, everything is revertible, and the file is yours.
