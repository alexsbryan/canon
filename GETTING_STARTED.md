# Getting started — a house's first hour

For a shared house, coliving community, or any group that already has
rules scattered across chat, documents and memory, and has never seen
them in one place.

Most of the hour is the conversation, not the tool.

---

## 1. Install

```sh
git clone <this repo> && cd canon
cargo build --release
```

Binary at `target/release/canon`. Put it on your `PATH`. If you don't
have Rust, `rustup.rs` is a one-line installer.

**For the ingest step you need a language model on your machine** —
llama.cpp, Ollama, LM Studio, anything OpenAI-compatible:

```sh
canon config set endpoint http://localhost:8080/v1
```

Only one person in the house needs this. Everything after step 4 works
without it.

## 2. Start a canon

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

`--profile house` makes the tool say **rule** rather than *commitment*,
and phrase answers as *which conversation this needs* rather than as a
verdict.

> Don't set `CANON_ACTOR`. Left alone, canon records acts as
> `human:<your git name>`. Setting it to a bare name marks your
> decisions as machine-written and prints
> `warning: adjudication(s) were not authored by a person` — a guard
> that exists so automation can't quietly write your house's rules.

## 3. Point it at everything you already have

This is the step that matters. Gather it loosely — a folder is fine, and
it does not need tidying first:

- the house handbook, whatever state it's in
- meeting notes, minutes, the shared doc
- a Slack or Discord export
- the onboarding email you send new housemates

```sh
canon draft --from ~/house-stuff
```

**There is no format list.** Anything under there that is text gets read,
whatever it's called — `.org`, `.eml`, a `NOTES` file with no extension,
a transcript pasted into a `.log`. Don't convert anything.

**Chat is read as chat**, not as prose. Messages are rendered with who
said them and cut into bursts on a time gap, so a rule found in a channel
cites the exchange it was decided in:

```text
RULE      Recycling goes out Sunday night.
          slack-export/general/2026-08-01.json:1-12

  > sam: reminder the recycling goes out sunday night not monday
  > mira: ^ this has bitten us three times now, can we make it a rule
  > sam: yes. recycling out sunday night.
```

Other ways in, same reviewer:

```sh
canon draft --from-git --since 1y                        # commit messages
cat transcript.txt | canon draft --from - --as '#house'  # anything, via stdin
canon draft --max-chunks 20                              # just try a bit first
```

`--as` names the source so the citation reads `#house:3-4` rather than
`stdin:3-4` — worth it on a chat feed, where the passage has scrolled
away by the time anyone reads the proposal.

## 4. Review together — this is the governance conversation

Proposals come one at a time and you accept, skip or reject each. There
is **no `--accept-all`**, on purpose: a canon adopted wholesale is one
nobody has read, and going through them together is usually the first
time a house has seen its own rules as a list.

**Judge every one.** The model misses real rules and proposes things
that aren't rules. Two things make that cheap to catch:

**Every proposal quotes the passage it came from.** Canon cuts the quote
out of your own file, so a citation that isn't in your document can't
happen. If a proposed rule has no source you recognise, reject it.

**It finds three kinds, not one.** A rule is a rule. *"Nobody has ever
said who looks after the allotment"* is a **question**. *"Decided not to
make a rota — it would turn a kindness into a duty"* is a **silence**: a
thing you decided *not* to have. Silences are the ones houses lose, and
losing them is why the same proposal comes back every spring.

**`[s]kip` records nothing** — skip means *not now*. Only `[r]eject`
means no, and a rejected proposal is not raised again.

Long review? `canon draft --resume` picks it back up later with no second
model run.

## 5. See what you didn't read

```text
3 file(s) were not read:
  2 x ignored by .gitignore
  1 x not text
  Naming a file directly reads it whatever it is; --include-ignored reads what .gitignore covers.
```

Canon reports every file it passed over and why, before you spend a model
run on the rest. Three cases, each with a
way round it:

- **your own `.gitignore` calls it generated** — `--include-ignored`
- **structured data holding no conversation** (a lockfile read as prose
  proposes rules cited to dependency names)
- **too big to be writing**

Naming a file directly reads it regardless — a walk is a guess about
intent, `--from thatfile` isn't.

## 6. Now the everyday half — no model needed

From here nothing calls a model.

```sh
canon list                       # what's live
canon add "<a rule>"             # the one you thought of afterwards
```

**Changing a rule keeps the reason.** Don't edit; supersede:

```sh
canon supersede can-ffc1e6e30686 \
  "Anyone can invite a guest for up to three nights; longer needs a house chat." \
  -m "Sam's cousin stayed two weeks in June and nobody knew how to raise it."
```

```text
can-e7ab38908043  Anyone can invite a guest for up to three nights; longer needs a house chat.
  replaces can-ffc1e6e30686
```

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

A year later, *"why do we have this rule about guests?"* has an answer
and nobody had to remember it. The `-m` reason isn't politeness — a rule
with no reason can't be revisited well, because you can't tell whether
the thing it protected against still exists.

Record the two other shapes by hand as they come up:

```sh
canon question "Who looks after the plants when everyone travels at once?"
canon silence "a chore rota" -m "We decided in March not to have one — it would turn a kindness into a duty."
```

```text
can-66fcda050a09  unwritten on purpose: "a chore rota"
  We decided in March not to have one — it would turn a kindness into a duty.
  `canon check --about "a chore rota"` will say so rather than call it a gap
```

## 7. Share it with the house

```sh
canon share            # a block you paste into your group chat
canon adopt --paste    # a housemate reads it back
canon diff --upstream  # how your copy diverged from what you adopted
```

```text
--- canon housedemo · house · snapshot 2026-08-29 · 20a846
Quiet hours are 10pm to 7am.  (can-0bc00855477c)
Whoever cooks does not wash up.  (can-3664f149e633)
Anyone can invite a guest for up to three nights; longer needs a house chat.  (can-e7ab38908043)
--- 3 live · adopt: canon adopt --paste
```

Pasting is a real answer, not a stopgap. A snapshot carries current rules
and drops the rationales — enough to adopt, not enough to audit, which is
the right trade for a chat thread. A block edited after it was sent is
refused rather than adopted under the sender's name.

If the canon lives in a git repo everyone just pulls it, and
`canon merge-driver` (run bare for setup) means two people adding rules
on the same day isn't a conflict.

## 8. Keep feeding it

The house keeps talking after the first hour. Point canon at the channel
again whenever you like:

```sh
canon draft --from ~/house-stuff
```

`.canon/seen` records which passages were already extracted and which
proposals you turned down, so a second pass costs you the **new**
material only, and a rule you rejected isn't proposed again. It's ingest
hygiene, not governance: nothing in it is a rule, and deleting it costs a
re-extraction and changes nothing you decided.

Once you have a body of rules, two more model-backed verbs earn their
keep:

```sh
canon check "convert the spare room into a studio"   # what does this run against?
canon tensions                                       # which of our rules conflict?
```

## 9. Later: decide how you decide

Worth doing after a few weeks, not on day one. Until you set a policy,
canon holds no opinion about how many objections stop a thing.

```sh
canon policy show
canon policy set consent -m "One reasoned objection stops a thing."
```

Rules available: `default`, `consent`, `threshold`,
`supermajority --of 2/3`, `subsidiarity`. The policy is itself a
recorded act, so it can be questioned, superseded and explained like any
other. `canon grant` gives someone standing over a scope, `canon who
<scope>` answers who may decide something, `canon overdue` lists what has
passed a review date.

---

## The short version

| you want to | run |
|---|---|
| read everything you already have | `canon draft --from <folder>` |
| feed it anything else | `… \| canon draft --from - --as '<name>'` |
| finish a review later | `canon draft --resume` |
| see what's live | `canon list` |
| add the one you thought of after | `canon add "<rule>"` |
| change one | `canon supersede <id> "<new>" -m "<why>"` |
| find out why | `canon why <id>` |
| note an open question | `canon question "<question>"` |
| note a deliberate gap | `canon silence "<subject>" -m "<why>"` |
| undo anything | `canon undo <act-id> -m "<why>"` |
| share | `canon share` |
| everything else | `canon help all` |

Nothing is destroyed, everything is revertible, and the file is yours.
