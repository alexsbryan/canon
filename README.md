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

Then you review, one at a time. There's no `--accept-all` — a set of
rules nobody read isn't worth having.

**New here? [Getting started](./GETTING_STARTED.md) walks a house through
its first hour.**

It's a house tool, and it's also a bet: that how a group decides things
can live in software as mechanism, not as a page in a wiki. Standing,
objections, scopes, deliberate silences and drawn lots are data
structures. "What would a different rule have done to us?" is a question
with an answer. [Skip to the two
experiments](#can-software-hold-governance) if that's what you came for.
One of them is the United States.

## Point it at the mess

You don't have to tidy anything first.

**There's no format list.** Anything under that folder that's text gets
read, whatever it's called — `.org`, `.eml`, a `NOTES` file with no
extension, a transcript someone pasted into a `.log`, a Slack export.

**Chat is read as chat.** Messages get rendered with who said them and
cut into bursts on a time gap, so a rule found in a channel cites the
exchange it was decided in:

```text
RULE      Recycling goes out Sunday night.
          slack-export/general/2026-08-01.json:1-12

  > sam: reminder the recycling goes out sunday night not monday
  > mira: ^ this has bitten us three times now, can we make it a rule
  > sam: yes. recycling out sunday night.
```

**It tells you what it didn't read.** Three things a walk passes over:
files your own `.gitignore` calls generated, structured data that holds
no conversation, and anything too big to be writing. Each one is counted
and named, with the way round it. Point at a file directly and it gets
read whatever it is.

**Read the same channel twice and it won't ask twice.** `.canon/seen`
remembers which passages it already pulled from and which proposals you
turned down. A second pass costs you the new material. A rule you
rejected doesn't come back tomorrow. Skip means *not now*; only reject
means no.

**`--from -` reads stdin**, which is the whole integration surface.
Anything that can emit text can feed a canon.

```sh
canon draft --from ~/house-docs          # a folder, recursively
canon draft --from-git --since 1y        # or your commit messages
cat transcript | canon draft --from - --as '#house'
canon draft --resume                     # finish a long review later
```

## It finds three things, not one

A tool that only found rules would hand you a list and miss half of what
a group actually decided.

- A **rule** is a rule.
- *"Nobody's ever said who looks after the allotment"* is a **question**.
- *"We decided not to make a rota — it'd turn a kindness into a duty"* is
  a **silence**: something you decided *not* to have.

Silences are the ones houses lose, and losing them is why the same
proposal comes back every spring.

Every proposal carries the passage it came from or it isn't shown. canon
cuts the quote out of your file itself, so a citation that isn't in your
document can't happen. If a proposed rule has no source you recognise,
reject it.

## Then it keeps the reason

Once you have rules, the everyday half needs no model at all. Don't edit
a rule — supersede it:

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

## What needs a model, and what doesn't

**Needs one:** `draft`, `check`, `tensions`, `rebase`.

**Needs nothing:** everything else. `add`, `list`, `why`, `supersede`,
`retract`, `accept`, `question`, `open`, `silence`, `undo`, `log`,
`share`, `adopt`, `diff`, and every governance verb — `who`, `grant`,
`scope`, `policy`, `position`, `decide`, `rank`, `horizon`, `overdue`,
`voice`, `draw`, `replay`. That's most of the tool and all of the daily
use. In a house, one person runs the model half and everyone else needs
nothing.

A model call is refused unless the endpoint is on this machine, unless
you pass `--allow-remote`. Every call prints which endpoint it used.

### The endpoint, and a shout to Commonwealth

canon was built against **[Commonwealth](https://github.com/alexsbryan/commonwealth-ai)**
and every accuracy figure and bench script here was measured on it —
`localhost:9741`, model alias `primary`, a 27B-class model. It's a sister
project and it's worth knowing about on its own, because it solves the
problem a house hits about ten minutes after deciding to run its own
model: **the good model doesn't fit on anybody's laptop.**

Commonwealth pools machines. Yours, and ones belonging to people you
trust. It splits a model's layers across them, and you talk to the result
as if it were running locally — three 64 GB machines hold a model no one
of them could. The mesh is symmetric: no master node, every machine runs
the same code, and when someone shuts their laptop the rest reform around
it.

The part that should sound familiar: **its trust model is social rather
than cryptographic.** You join a mesh because someone you know invited
you. No token, no blockchain, no central registry — each node holds all
the state there is. That's the same bet canon makes about rules, made
about hardware. A house that already pools a kitchen can pool GPUs.

It serves an OpenAI-compatible API, so canon just sees one local model.
There's no separate binary to babysit — the mesh lives inside the
Sovereign daemon, and you create or join one with `sovereign mesh create`
and `sovereign mesh join`. Start with
[Run a model bigger than your machine](https://github.com/alexsbryan/commonwealth-ai/blob/main/docs/RUN_A_BIGGER_MODEL.md).

**Other servers work too.** canon speaks plain OpenAI chat completions
and carries no connector, no vendor schema and no endpoint of its own, so
llama.cpp, vllm or anything compatible will run it. But "runs" and "has
the batteries" are different claims, and two things separate them.

canon asks the server to enforce a JSON schema. If yours can't, it
retries once in plain JSON mode with the schema in the prompt and says so
on stderr. It never parses prose.

And model size is what actually moves quality here. Reading documents and
spotting contradictions is the hard part, and a small local model
proposes worse rules and misses more conflicts than anything measured in
this repo. That's the gap Commonwealth exists to close. Run
`./scripts/draft-bar.sh 3` against your own endpoint to find out where
you land.

## Can software hold governance?

That's the real question. canon takes two swings at it, one serious and
one cheeky.

### Ostrom's eight principles are the acceptance test

Elinor Ostrom spent a career on what commons that *don't* collapse have
in common, and got it down to eight design principles. Those are the bar
here, and the same eight marks have to clear in a twelve-person house and
in a codebase.

The decision layer is pure — `Log → Canon → policy → Decision`, no
filesystem, no network, no model — so a whole history of governance
replays instantly:

```sh
canon replay fixtures/fernwood-commons
```

```text
42 step(s), all as expected
```

42 governance decisions in 0.036 seconds with no endpoint: standing
granted and withdrawn, an objection blocking a thing, a scope handed
down, a lot drawn from a sealed seed nobody could steer.

The counterfactual is the better trick. *What would a different rule have
done to the last six months?* is the question a group has before changing
how it decides, and it's normally unanswerable. Here it's a flag:

```sh
canon replay fixtures/fernwood-commons --policy default
```

```text
a-sabotage-proposal-dies-on-unaddressed.because: expected "cautious: irreversible", got "default: nothing bears on it"
the-same-thing-reversible-is-not-refused.authority: expected "act-and-notify", got "ask-one"

20 mismatch(es)
```

Twenty places your real history would have gone differently under another
rule, named one by one, in 36 milliseconds.

### We handed it the United States

`fixtures/founding/` is the Declaration of Independence, the Articles of
Confederation, and the Constitution with all twenty-seven amendments — 91
sections, 12,672 words, built from vendored National Archives and Avalon
Project transcripts.

The good part is that we didn't write the answer key. The Archives prints
a note under each amendment naming the article it modified or superseded.
`build.py` parses those notes out of the same HTML the corpus is built
from, and refuses anything it can't parse. Out falls eleven supersessions
nobody planted — the 11th Amendment against Article III's judicial power,
the 17th against senators chosen by state legislatures, the 13th against
the fugitive slave clause, the 16th against the ban on unapportioned
direct taxes.

Then six tensions we did write, each quoting both passages so you can
argue with the reading — all men are created equal, against the
three-fifths clause. And six decoys: pairs that look like contradictions
and aren't, like two amendments carrying the same enforcement sentence.

So: point a tool at the founding documents of a country, cold, and ask it
which rules contradict which.

Where it stands. Shown the two commitments alone, on a 27B-class model,
it finds 9 of the 11 supersessions and calls none of the four testable
decoys a conflict. That's an upper bound, not a score — the real run
shows each pair among twenty-two others, and that run has never finished.
It's the most interesting unfinished thing here.
[DEMO_PLAN.md](./DEMO_PLAN.md) is the ledger of what's measured and what
isn't, including the bars written down before the data that tests them.

## Before you rely on it

It's early. Another group hasn't used this yet. You'd be among the first,
which is worth knowing before your house puts its rules somewhere. Every
verb is implemented and tested — 361 tests.

The ingest is the good part and the imperfect part. It calls a language
model, and a language model misses real rules and proposes things that
aren't rules. That's why review is one at a time and why every proposal
has to cite its source: the design assumes the model is wrong sometimes
and makes that cheap to catch.

Accuracy is measured against two vendored documents — a house charter
(`fixtures/maple-house`) and municipal code
(`fixtures/des-moines-noise`) — always naming the model and endpoint that
produced a run. The scripts are here so you can measure your own setup
instead of inheriting ours:

```sh
./scripts/draft-bar.sh 3                        # runs against your endpoint
./scripts/score-bar.sh maple-house <runs-dir>   # score them
```

No numbers are quoted in this README on purpose. The last published ones
predate a change to how contradictions get detected, and a stale accuracy
figure is worse than none.

## More

- [Getting started](./GETTING_STARTED.md) — a house's first hour.
- `canon --help` is seven verbs. `canon help all` is all of them.
- [SPEC.md](./SPEC.md) — the file format, CC0. Adopting the format isn't
  a lock-in decision.
- [PRIMITIVES.md](./PRIMITIVES.md) — the design argument: nine
  primitives, the line between mechanism and policy, and eighteen
  technologies of political economy tested against them.
- [DEMO_PLAN.md](./DEMO_PLAN.md) — the founding-documents ledger.
- [Commonwealth](https://github.com/alexsbryan/commonwealth-ai) — pool
  your machines with people you trust and run a model none of them could
  hold alone. What canon was built and measured against.

## Build

```sh
cargo build --release      # binary at target/release/canon
cargo test
```

AGPL-3.0-or-later. The format specification is CC0.
