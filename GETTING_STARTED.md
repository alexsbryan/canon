# Getting started

For a shared house, coliving community, or any group that already has
rules scattered across chat, documents and memory, and has never seen
them in one place.

About an hour. Most of it is the conversation, not the tool.

## What you'll need

- **A terminal**, and the `canon` binary (below).
- **The documents you already have**, gathered loosely into one folder.
  A handbook in whatever state it's in, meeting notes, a Slack export,
  the onboarding email you send new housemates. Don't tidy them.
- **A language model**, for the reading step only. One person in the
  house needs this. Everything from step 6 works without one.

## Install

```sh
git clone <this repo> && cd canon
cargo build --release
```

The binary lands at `target/release/canon`. Put it on your `PATH`. If you
don't have Rust, `rustup.rs` is a one-line installer.

### Pointing it at a model

The reference setup is
**[Commonwealth](https://github.com/alexsbryan/commonwealth-ai)**. Every
accuracy measurement in this repo was taken against it, and each one names
the model that served it rather than the alias it was called with: the
`primary` below answered as a 27B through 2026-08-30 and as a 35B
mixture-of-experts after, so the alias on its own would not tell you what
you were reading.

```sh
canon config set endpoint http://localhost:9741/v1
canon config set model primary
```

Commonwealth is worth a look for a house specifically. The model that's
good at this doesn't fit on one laptop, and Commonwealth pools machines —
yours and ones belonging to people you trust — splitting a model's layers
across them so you talk to it as if it were local. You join a mesh
because someone invited you; there's no token and no central registry.
A house that already pools a kitchen can pool GPUs. Start with
[Run a model bigger than your machine](https://github.com/alexsbryan/commonwealth-ai/blob/main/docs/RUN_A_BIGGER_MODEL.md).

Any OpenAI-compatible server will also run it — canon sends plain chat
completions and carries no vendor anything:

```sh
canon config set endpoint http://localhost:8080/v1   # llama.cpp, vllm, …
canon config set model <the name that server expects>
```

Two things before you assume that's equivalent. canon asks the server to
enforce a JSON schema; if yours can't, it retries once in plain JSON mode
with the schema in the prompt and tells you it did. It never parses
prose. And model size is what actually moves quality — a small model on a
laptop proposes worse rules and misses more contradictions than the
numbers here suggest. That's a different quality regime, not a fallback
with the same batteries. `./scripts/draft-bar.sh 3` tells you where yours
lands.

## Set it up

**1. Start a canon.**

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

`--profile house` makes the tool say **rule** instead of *commitment*,
and phrase its answers as which conversation this needs rather than as a
verdict.

Don't set `CANON_ACTOR`. Left alone, canon records acts as
`human:<your git name>`. Set it to a bare name and your decisions get
marked machine-written, with a warning to match — a guard that exists so
automation can't quietly write your house's rules.

**2. Point it at everything you already have.**

```sh
canon draft --from ~/house-stuff
```

There's no format list. Anything under there that's text gets read,
whatever it's called. Don't convert anything.

Chat is read as chat, not as prose. Messages get rendered with who said
them and cut into bursts on a time gap, so a rule found in a channel
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

`--as` names the source, so a citation reads `#house:3-4` rather than
`stdin:3-4`. Worth it on a chat feed, where the passage has scrolled away
by the time anyone reads the proposal.

**3. Review together.**

This is the governance conversation, and it's the point of the whole
step. Proposals come one at a time and you accept, skip or reject each.
There's no `--accept-all` — going through them together is usually the
first time a house has seen its own rules as a list.

Judge every one. The model misses real rules and proposes things that
aren't rules. Two things make that cheap to catch:

Every proposal quotes the passage it came from, cut out of your own file,
so a citation that isn't in your document can't happen. If a proposed
rule has no source you recognise, reject it.

And it finds three kinds, not one. A rule is a rule. *"Nobody's ever said
who looks after the allotment"* is a **question**. *"We decided not to
make a rotation — it'd turn a kindness into a duty"* is a **silence**,
something you decided not to have. Silences are the ones houses lose.

Skip means *not now* and records nothing. Only reject means no, and a
rejected proposal doesn't come back.

Long review? `canon draft --resume` picks it up later with no second
model run.

**4. Check what it didn't read.**

```text
3 file(s) were not read:
  2 x ignored by .gitignore
  1 x not text
  Naming a file directly reads it whatever it is; --include-ignored reads what .gitignore covers.
```

Three cases: your own `.gitignore` calls it generated, it's structured
data holding no conversation, or it's too big to be writing. Point at a
file directly and it gets read regardless.

**5. Look at what you've got.**

```sh
canon list
```

```text
can-0bc00855477c  Quiet hours are 10pm to 7am.
can-3664f149e633  Whoever cooks does not wash up.
can-ffc1e6e30686  Anyone can invite a guest for up to three nights.

3 rules live
```

Those `can-…` ids are how you refer to a rule later. You only need enough
of one to be unambiguous.

**6. Now the everyday half.**

Nothing from here calls a model.

```sh
canon add "<the rule you thought of afterwards>"
```

Changing a rule keeps the reason. Don't edit — supersede:

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

A year later, *why do we have this rule about guests?* has an answer and
nobody had to remember it. The `-m` reason isn't politeness. A rule with
no reason can't be revisited well, because you can't tell whether the
thing it protected against still exists.

Record the other two shapes by hand as they come up:

```sh
canon question "Who looks after the plants when everyone travels at once?"
canon silence "a chore rotation" -m "We decided in March not to have one — it would turn a kindness into a duty."
```

```text
can-66fcda050a09  unwritten on purpose: "a chore rotation"
  We decided in March not to have one — it would turn a kindness into a duty.
  `canon check --about "a chore rotation"` will say so rather than call it a gap
```

**7. Share it.**

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
`canon merge-driver` (run it bare for setup) means two people adding
rules on the same day isn't a conflict.

## What to expect

The first pass over your documents is the expensive one. After that,
`.canon/seen` remembers which passages it already pulled from and which
proposals you turned down, so pointing canon at the same channel next
month costs you the new material only. It's ingest hygiene, not
governance — nothing in it is a rule, and deleting it costs a
re-extraction and changes nothing you decided.

Once you have a body of rules, two more model-backed verbs earn their
keep:

```sh
canon check "convert the spare room into a studio"   # what does this run against?
canon tensions                                       # which of our rules conflict?
```

## Later: decide how you decide

Worth doing after a few weeks, not on day one. A rule about the hall is
one thing; who may write it is another; who may change *that* is a third.
Ostrom found those three tiers in every commons that lasted the centuries,
and canon keeps them in the same log as the rules themselves.

| tier | what it settles | the act | who may |
|---|---|---|---|
| **operational** | bikes against the left wall; quiet after eleven | `canon add "…" --scope house.hall` | anyone — it's a proposal until the scope ratifies it |
| **collective-choice** | who writes hall rules, and by what count | `canon grant`, `canon ratification set` | whoever holds that scope |
| **constitutional** | who may change *that* | the same two acts, aimed one scope up | whoever holds the scope above |

Three commands, in the order a house reaches for them:

```sh
canon grant mira house.hall                                # who has a say, and where
canon ratification set threshold:2/1 --scope house.hall    # what makes a proposal a rule
canon policy set consent -m "One reasoned objection stops a thing."
```

**Ratification** is how a written proposal becomes a rule: `standing`
(whoever holds the scope writes directly — the default, and what every
canon did before there was a choice), `joint:a,b` (everyone named),
`threshold:2/1` (two approving carries, one objecting stops), or
`consent:7d` (seven days' silence carries it). Set it per scope. The
deepest one wins, so the kitchen can decide differently from the house
and neither has to know about the other.

**Policy** is the other axis — what `canon check` decides under when a
proposal runs into a rule: `default`, `consent`, `threshold`,
`supermajority --of 2/3`, `subsidiarity`. Both are ordinary recorded acts,
so you can question them, supersede them and ask `canon why`.

Once a ratification rule is set, a write tells you where it stands, and
`canon approve <id>` / `canon object <id> -m "<why>"` move it:

```text
can-7e3dfab7d635  Bikes go against the left wall of the hall.
  PROPOSED, not yet a rule — needs 1 more approval(s) from people who hold house.hall
```

Four things to know before you lean on it:

- **The narrowest seat counts.** Holding `house` covers the kitchen, but
  a kitchen rule is ratified by whoever holds `house.kitchen`. Wider
  standing asks; it doesn't act.
- **Nothing is judged retroactively.** A proposal is judged under the
  ratification rule that was in force the day it was written, so
  tightening the bar today doesn't unmake last year's rules.
- **An agent may propose and may object; it never ratifies** — not its
  own proposal, not even holding a seat.
- **Out of seat is recorded, not applied.** A grant, a policy, a
  ratification rule, a ruling or an `undo` by somebody without standing
  over what it touches lands in the log marked `NOT APPLIED`, and
  `canon list` says how many are sitting there.

### Before you change how you decide, ask what the change would do

```sh
canon replay                                              # your own record, re-decided
canon replay --policy consent --brief                     # what consent would have done
canon replay --policy threshold --objections 2 --brief    # or two objections to stop a thing
```

No setup, no files. The questions come from what your canon already holds —
every subject somebody took a position on, and everything the group
decided — so the answer is about decisions your house really had rather
than ones somebody invented for the exercise:

```text
Under `threshold` instead of the rules this canon adopted, 1 of 3 decisions land somewhere else.
1 would be easier to do.

  EASIER
    Nothing in the fridge unlabelled after Sunday.
        not under this policy → act
```

If the record has nothing in it yet, it says so and names the one act that
fills it in. If you want to ask better questions than the record can know
about — a proposal in your own words, who would be doing it, what can't be
undone — `canon replay --write-scenario questions.jsonl` writes the derived
ones out to edit, and `--scenario questions.jsonl` uses yours instead.

`canon who <scope>` answers who may decide something without anyone
having to be asked. `canon overdue` lists what's passed a review date set
with `canon horizon` — which is also how you give a seat a term.

## If it gets something wrong

Nothing is destroyed and everything is revertible, including a revert.
Your own acts are always yours to undo; undoing somebody else's takes
standing over it, the same as writing it would have.

```sh
canon undo <act-id> -m "<why>"
canon retract <id> -m "<why>"      # withdraw a rule, no replacement
canon log                          # the raw acts, oldest first
```

The whole canon is `.canon/acts.jsonl`, one line per decision. It's
yours, it diffs, and it greps. If canon vanished tomorrow the file would
still say what your house decided.

## Common questions

[COOKBOOK.md](./COOKBOOK.md) works through the ones groups actually hit — who
decides this, how to stop one person rewriting the rules, what to do when
nothing you have decided covers a situation, what a different rule would have
done to you, and how an agent fits in. Real commands, real output.

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
| undo anything you wrote | `canon undo <act-id> -m "<why>"` |
| share | `canon share` |
| everything else | `canon help all` |
