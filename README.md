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
with an answer. [Skip to the
experiments](#can-software-hold-governance) if that's what you came for:
fourteen commons built from one spine, and the United States.

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

That's the real question. canon takes three swings at it: a bar, a study,
and one that's frankly cheeky.

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
canon replay fixtures/fernwood-commons --policy default --brief
```

```text
Under `default` instead of the rules this canon adopted, 9 of 42 decision(s) change.

  dig two more beds at the allotment
    was   not under this policy         nobody holds standing over `allotment`
    would ask one person with standing  nothing bears on it

  keep the bikes where they are
    was   not under this policy         1 reasoned objection(s); one is enough
    would ask one person with standing  at least one commitment pulls against

  replace the front door lock with a keypad only I know the code to
    was   not under this policy         irreversible (nothing bears on it and nobody
                                        objected, but nobody looked either)
    would ask one person with standing  nothing bears on it
```

Nine decisions in this house's real history land somewhere else under a
rule it didn't adopt — named, with the reason on both sides, in under a
tenth of a second and with no model.

It is decided twice to say that: once under the rules the house adopted,
once under the forced one. An earlier version of this README said twenty,
which counted changed *fields* rather than changed decisions and roughly
doubled the number. Nine is the number of decisions.

### Then we did it to fourteen commons at once

Two fixtures show the eight principles are reachable. They don't show they're
*general*, and "general" is the whole bet. So:
[**the CPR transfer study**](./STUDY.md).

Fourteen institutions — a makerspace's tools, a coliving building's boiler, a
monorepo, a mesh of pooled machines, shared CI capacity, a community garden's
standpipe, a forum's attention, and three of Ostrom's own cases as a control.
All fourteen are built from **one 104-line spine**. What a new commons costs
is a vocabulary of nouns and the shape of the thing — no rules; the bar
refuses a vocabulary that tries to name one.

```sh
./scripts/cpr-sweep.sh      # the whole study, no endpoint, ~3 seconds
```

The ten differ on purpose: 6 to 24 people, two levels of nesting or three,
monitored by a bot or by a person on a rotation, forked from an upstream or
founded outright. An earlier draft had ten vocabularies over three shapes,
which is one institution in ten coats of paint — the bar now refuses that too.

**Four of the fourteen are ablations**, each removing one use of one primitive
and naming in advance which principles it expects to lose. They go red exactly
there, which is what makes the other ten worth anything. One institution
declares two principles inapplicable to it — it was founded, not forked, and
both are about divergence from an upstream — and has to prove they genuinely
fail rather than quietly skipping them.

The study found two defects in its own instrument on the way through, one of
them a place where `canon who` couldn't tell two levels from one. Both are
written up, including the uncomfortable reading of how the first was fixed.

Then the harder half, which needs a model: point canon at a real house's real
charter, cold. On a 27B-class local model it reached **seven of the eight**
principles the document carries, in its own words — including *"Recorded
decisions amend or extend the Charter"*, which is Ostrom's seventh, unprompted.
The eighth is graduated sanctions, and the charter only half has it: one flat
late fee that never escalates. Pointed at a municipal noise ordinance instead,
it reached three of the four that document carries and dropped the one passage
holding an enforcement ladder.

Both numbers name their model and endpoint, and
[STUDY.md](./STUDY.md#what-the-two-legs-together-do-and-do-not-say) is
explicit about what neither leg establishes — starting with the fact that I
chose the ten shapes and the axes they vary along.

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

Where it stands, and this is two numbers rather than one.

Shown the two commitments alone, on a 35B local model, it finds 9 of the
11 supersessions and calls none of the four testable decoys a conflict.

**Then we checked whether it was reading or remembering.** Every model has
read the Constitution. So we took the nine it found, removed the fact each
contradiction turns on — matched the two dates, matched the two ages,
repealed a different amendment — and asked again. Five dropped, as a
reader should. **Four survived the removal of the thing they turn on.**
A control arm that changes an irrelevant word instead left all nine
standing, so it isn't that editing confuses it.

So the honest reading of 9 of 11 is: at least five of those are the model
reading the passage in front of it, and up to four are recall. That bound
is written down, the perturbation table was fixed before the run, and
neither number is quoted here without the other. The full-window run —
each pair among twenty-two others — has still never finished.

None of this touches the half of canon that needs no model. `replay`,
the ledger, standing, scopes and every governance verb never call one.
[DEMO_PLAN.md](./DEMO_PLAN.md) is the ledger of what's measured and what
isn't, including the bars written down before the data that tests them.

## Before you rely on it

It's early. Another group hasn't used this yet. You'd be among the first,
which is worth knowing before your house puts its rules somewhere. Every
verb is implemented and tested — 363 tests.

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
- [STUDY.md](./STUDY.md) — the CPR transfer study: fourteen commons in ten
  shapes, one spine, four ablations, and what a real charter gave up to a
  local model. Includes what it does not establish.
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
