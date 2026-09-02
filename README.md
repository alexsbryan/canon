# canon

Your house already has rules. They're spread across two years of chat, a
handbook nobody's opened since 2023, and someone's memory. When a rule
changes, no one can remember why the old one existed.

canon reads what you already wrote and proposes the rules it finds. Each
one quotes the passage it came from, so you can check it. You go through
them together and keep the ones that are real.

```sh
canon init --profile house
canon draft --from ~/house-stuff     # a folder. anything text in it.
```

You review one at a time. There's no `--accept-all` — a set of rules
nobody read isn't worth having.

**New here? [Getting started](./GETTING_STARTED.md) walks a house through
its first hour.**

It's a house tool, and it's also a bet: that how a group decides things
can live in software as mechanism, not as a page in a wiki. Standing,
objections, scopes, deliberate silences and drawn lots are data
structures. *What would a different rule have done to us?* is a question
with an answer. [Skip to the experiments](#can-software-hold-governance)
if that's what you came for.

## Point it at the mess

Don't tidy anything first. There's no format list — anything under that
folder that's text gets read, whatever it's called: `.org`, `.eml`, a
`NOTES` file with no extension, a Slack export. Chat is read as chat, so a
rule found in a channel cites the exchange it was decided in. It counts
and names whatever it skipped. Read the same channel twice and it won't
ask twice.

```sh
canon draft --from ~/house-docs                      # a folder, recursively
canon draft --from-git --since 1y                    # or your commit messages
cat transcript | canon draft --from - --as '#house'  # stdin: the whole integration surface
canon draft --resume                                 # finish a long review later
```

And it finds three things, not one:

- A **rule** is a rule.
- *"Nobody's ever said who looks after the allotment"* is a **question**.
- *"We decided not to make a rotation — it'd turn a kindness into a duty"*
  is a **silence**: something you decided *not* to have.

Silences are the ones houses lose, and losing them is why the same
proposal comes back every spring. Every proposal carries the passage it
came from — cut out of your own file, so a citation that isn't in your
document can't happen. If a proposed rule has no source you recognise,
reject it.

## Then it keeps the reason

The everyday half needs no model at all. Don't edit a rule — supersede it:

```sh
canon supersede can-ffc1 "Guests up to three nights; longer needs a house chat." \
  -m "Sam's cousin stayed two weeks in June and nobody knew how to raise it."

canon why can-e7ab     # six months later: why is this rule like this?
```

Nothing is destroyed and everything is revertible, including a revert. A
contradiction you're carrying on purpose is something you can record
(`canon accept`), not a bug to clean up.

Everything lives in `.canon/acts.jsonl` — one line per decision,
append-only. It diffs, so git gives you history for free. It greps.
Leaving is deleting a directory. No account, no server, nothing leaves
your machine.

## A rule is a proposal until the people it governs say so

Give people standing over a scope, say how that scope makes rules, and a
write from anyone else stays visible and not in force until the rule is
met:

```sh
canon grant human:dana house.kitchen
canon grant human:sam house.kitchen
canon ratification set joint:human:dana,human:sam --scope house.kitchen \
  -m "Both cooks agree, or it is not a kitchen rule."

canon add "Wash your own pan before you sit down." --scope house.kitchen
#   PROPOSED, not yet a rule — needs approval from human:dana, human:sam
canon approve can-9b31       # as dana; then as sam, and it is in force
canon object can-9b31 -m "…" # one named holder's reason refuses it
```

That is Ostrom's three tiers in one log: the rules, the rules for making
rules, and who may change *those* — the same acts, aimed one scope up.
Four ratification rules ship: `standing` (holders write, others propose),
`joint` (named people, all of them), `threshold` (so many for, so many
against) and `consent` (a rule after N days unless a holder objects with
a reason).

An agent may propose and object under any of them. It cannot mint a rule,
even where it holds standing. Governing out of seat — a grant, a policy,
a ruling, a retraction or an undo by somebody with no say over what it
touches — is kept on the record and changes nothing. A canon that has
granted nobody standing is a notebook, and stays one until the first
grant. The tier table is in [Getting
started](./GETTING_STARTED.md#later-decide-how-you-decide).

## What needs a model, and what doesn't

**Needs one:** `draft`, `check`, `tensions`, `rebase`.

**Needs nothing:** everything else — the ledger, standing, scopes,
ratification, `replay`, and every other verb. That's most of the tool and
all of the daily use. In a house, one person runs the model half and
everyone else needs nothing.

A model call is refused unless the endpoint is on this machine, unless
you pass `--allow-remote`. Every call prints which endpoint it used.

### The endpoint, and a shout to Commonwealth

canon was built against
**[Commonwealth](https://github.com/alexsbryan/commonwealth-ai)**, and
every accuracy figure and bench script here was measured on it. It's a
sister project worth knowing about on its own, because it solves the
problem a house hits ten minutes after deciding to run its own model:
**the good model doesn't fit on anybody's laptop.** Commonwealth pools
machines — yours, and ones belonging to people you trust — splitting a
model's layers across them so three 64 GB machines hold a model none of
them could, with no master node. And **its trust model is social rather
than cryptographic**: you join a mesh because someone you know invited
you, with no token and no central registry. That's the same bet canon
makes about rules, made about hardware. A house that already pools a
kitchen can pool GPUs. Start with [Run a model bigger than your
machine](https://github.com/alexsbryan/commonwealth-ai/blob/main/docs/RUN_A_BIGGER_MODEL.md).

Any OpenAI-compatible server will also run canon, which speaks plain chat
completions and carries no vendor anything. Two caveats: if your server
can't enforce a JSON schema, canon retries once in plain JSON mode and
says so — it never parses prose; and model size is what actually moves
quality here, so a small local model proposes worse rules and misses more
conflicts than anything measured in this repo.
`./scripts/draft-bar.sh 3` tells you where yours lands.

## Can software hold governance?

That's the real question, and canon takes three swings at it.

### The bar is Ostrom's eight principles

Elinor Ostrom spent a career on what commons that *don't* collapse have in
common, and got it down to eight design principles. Those are the
acceptance test here, and the same eight marks have to clear in a
twelve-person house and in a codebase.

The decision layer is pure — `Log → Canon → policy → Decision`, no
filesystem, no network, no model — so a whole history of governance
replays instantly, and *what would a different rule have done to the last
six months?* stops being unanswerable and becomes a flag:

```sh
canon replay                                    # your own canon, no setup, no files
canon replay --policy consent --brief           # what consent would have done to it
canon replay fixtures/fernwood-commons --policy default --brief    # or a worked one
```

```text
Under `default` instead of the rules this canon adopted, 9 of 56 decisions land somewhere else.
6 would be easier to do; 3 harder.

  EASIER
    dig two more beds at the allotment
        not under this policy → ask one person with standing
    keep the bikes where they are
        not under this policy → ask one person with standing

  HARDER
    put a Wednesday cook on a rotation
        act, and say that you did → ask one person with standing
```

Nine decisions in this house's real history land somewhere else under a
rule it didn't adopt — six of them easier to do and three harder, each
named, with the reason on both sides, in under a tenth of a second and
with no model.

On your own canon it takes no arguments and no setup: the questions come
from what the record already holds, every subject somebody took a position
on and everything the group decided. The whole policy vocabulary can be
forced, so the rule you are actually weighing — `--policy threshold
--objections 2` — is one you can ask about.

### What about the USA?

`fixtures/founding/` is the Declaration, the Articles of Confederation and
the Constitution with all twenty-seven amendments — 91 sections, 12,672
words. We didn't write the answer key: the National Archives prints a note
under each amendment naming what it superseded, and `build.py` parses
those out of the same HTML the corpus is built from. Out falls eleven
supersessions nobody planted.

Shown the two commitments alone, on a 35B local model, canon finds **9 of
the 11** and calls none of the four testable decoys a conflict. **Then we
checked whether it was reading or remembering** — every model has read the
Constitution. We took the nine, removed the fact each contradiction turns
on, and asked again. Five dropped, as a reader should. **Four survived the
removal of the thing they turn on.** A control arm that changed an
irrelevant word left all nine standing, so it isn't that editing confuses
it.

So the honest reading of 9 of 11 is that at least five are the model
reading the passage in front of it and up to four are recall, and neither
number is quoted without the other. [DEMO_PLAN.md](./DEMO_PLAN.md) is the
ledger of what's measured and what isn't, including the bars written down
before the data that tests them.

## Before you rely on it

It's early. Another group hasn't used this yet. You'd be among the first,
which is worth knowing before your house puts its rules somewhere. Every
verb is implemented and tested — 398 tests.

The ingest is the good part and the imperfect part. It calls a language
model, and a language model misses real rules and proposes things that
aren't rules. That's why review is one at a time and why every proposal
has to cite its source: the design assumes the model is wrong sometimes
and makes that cheap to catch.

Accuracy is measured against two vendored documents — a house charter
(`fixtures/maple-house`) and municipal code
(`fixtures/des-moines-noise`) — always naming the model and endpoint that
produced a run. No ingest accuracy figures are quoted in this README on
purpose: the last published ones predate a change to how contradictions
get detected, and a stale number is worse than none. Measure your own:

```sh
./scripts/draft-bar.sh 3                        # runs against your endpoint
./scripts/score-bar.sh maple-house <runs-dir>   # score them
```

## More

- [One page](./ONE_PAGER.md) — why you'd use it, the ideas that carry it,
  and exactly where a model is called.
- [Getting started](./GETTING_STARTED.md) — a house's first hour.
- [Cookbook](./COOKBOOK.md) — the questions groups actually ask, and the acts
  that answer them, with real output.
- `canon --help` is seven verbs. `canon help all` is all of them.
- [SPEC.md](./SPEC.md) — the file format, CC0. Adopting the format isn't
  a lock-in decision.
- [STUDY.md](./STUDY.md) — the CPR transfer study, and what it does not
  establish.
- [PRIMITIVES.md](./PRIMITIVES.md) — the design argument: nine
  primitives, the line between mechanism and policy, and eighteen
  technologies of political economy tested against them.
- [DEMO_PLAN.md](./DEMO_PLAN.md) — the founding-documents ledger.
- [Contributing](./CONTRIBUTING.md) — every path is open; the fastest way
  in is a fixture. [Governance](./GOVERNANCE.md), [security](./SECURITY.md),
  and [where to get help](./SUPPORT.md).
- [Commonwealth](https://github.com/alexsbryan/commonwealth-ai) — pool
  your machines with people you trust and run a model none of them could
  hold alone. What canon was built and measured against.

## Build

```sh
cargo build --release      # binary at target/release/canon
cargo test                 # 398 tests, about six seconds
```

Two crates, no native dependencies. `rust-toolchain.toml` pins the
version, so `rustup` fetches the right one by itself.

## Contributing

Every path in this repository is open to pull requests. What gates a
change is the suite, not a list of permitted directories, and
`./scripts/pre-push.sh` runs the same set CI does in about ten seconds.

The most useful thing you can send isn't a patch — it's a **fixture**, or
what happened when you pointed `canon draft` at your own mess. Another
group hasn't used this yet. [CONTRIBUTING.md](./CONTRIBUTING.md) has the
rest.

canon governs itself, in canon. `.canon/acts.jsonl` is committed and holds
this project's own rules — three questions nobody has answered, three
things decided against on purpose with the reason attached, and a
fourteen-day consent rule on documentation that binds the steward rather
than you:

```sh
canon list        # what's in force here, and what's still proposed
canon open        # what nobody has decided
canon why <id>    # where any one of them came from
```

[GOVERNANCE.md](./GOVERNANCE.md) says who decides today and what would
change that.

AGPL-3.0-or-later. The format specification is CC0.
