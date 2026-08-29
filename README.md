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

It is a house tool, and it is also a bet: that how a group decides things
can live in software as *mechanism* rather than as a page in a wiki — that
standing, objections, scopes, deliberate silences and drawn lots are data
structures, and that "what would a different rule have done to us?" is a
question with an answer. [Skip to the two experiments](#can-software-actually-hold-governance)
if that is the part you came for. One of them is the United States.

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

**Needs an endpoint:** `draft`, `check`, `tensions`, `rebase`. A call is
**refused unless the endpoint is on this machine** unless you pass
`--allow-remote`, and every call prints which endpoint it used.

**The reference setup is the commonwealth-ai mesh daemon** at
`localhost:9741`, model alias `primary`, and it is worth saying plainly:
every accuracy figure and every bench script in this repo was measured
against that, on a 27B-class model. Canon itself speaks plain OpenAI chat
completions and carries no connector, no vendor schema and no endpoint of
its own, so llama.cpp, vllm or anything compatible will run it — but
"runs" and "has the batteries" are different claims.

Two honest caveats on a generic server. Canon asks it to **enforce a JSON
schema**; if it can't, canon retries once in plain JSON mode with the
schema in the prompt and *says so on stderr* rather than substituting
quietly, and it never parses prose. And **model size is what moves
quality** — reading documents and finding contradictions is the hard part
here, and a small model will propose worse rules and miss more conflicts
than anything measured in this repo. `./scripts/draft-bar.sh 3` tells you
where your own endpoint lands.

**Needs nothing:** everything else — `add`, `list`, `why`, `supersede`,
`retract`, `accept`, `question`, `open`, `silence`, `undo`, `log`,
`share`, `adopt`, `diff`, and every governance verb (`who`, `grant`,
`scope`, `policy`, `position`, `decide`, `rank`, `horizon`, `overdue`,
`voice`, `draw`, `replay`). That's most of the tool and all of the daily
use. In a house, one person runs the model half; everyone else needs
nothing.

## Can software actually hold governance?

That is the real question, and canon takes two swings at it — one
serious, one cheeky.

### The serious one: Ostrom's eight principles are the acceptance test

Elinor Ostrom spent a career documenting what commons that *don't*
collapse have in common, and got it down to eight design principles.
They are the bar here, and the same eight marks have to clear in a
twelve-person house and in a codebase.

The decision layer is pure — `Log -> Canon -> policy -> Decision`, no
filesystem, no network, no model — so a whole history of governance
replays in milliseconds:

```sh
canon replay fixtures/fernwood-commons
```

```text
42 step(s), all as expected
```

That is 42 governance decisions in **0.036 seconds** with no endpoint:
standing granted and withdrawn, an objection blocking a thing, a scope
handed down, a lot drawn from a sealed seed nobody could steer.

But the counterfactual is the better trick. **What would a different
rule have done to the last six months?** is the question a group
actually has before changing how it decides, and it is usually
unanswerable. Here it is a flag:

```sh
canon replay fixtures/fernwood-commons --policy default
```

```text
a-sabotage-proposal-dies-on-unaddressed.because: expected "cautious: irreversible", got "default: nothing bears on it"
the-same-thing-reversible-is-not-refused.authority: expected "act-and-notify", got "ask-one"

20 mismatch(es)
```

Twenty places the house's actual history would have gone differently
under another rule, named individually, in 36 milliseconds, against the
real record. That is what it means for governance to be a thing software
can hold rather than a thing software stores.

### The cheeky one: we handed it the United States

`fixtures/founding/` is the Declaration of Independence, the Articles of
Confederation, and the Constitution with all twenty-seven amendments —
91 sections, 12,672 words, built from vendored National Archives and
Avalon Project transcripts.

**The ground truth is the good part, because we didn't write it.** The
Archives prints a note under each amendment naming the article it
modified or superseded. `build.py` parses those notes out of the same
HTML the corpus is built from and refuses anything it cannot parse. Out
falls **eleven supersessions nobody planted** — the 11th Amendment
against Article III's judicial power, the 17th against senators chosen
by state legislatures, the 13th against the fugitive slave clause, the
16th against the ban on unapportioned direct taxes.

Then six tensions we *did* author, each quoting both passages so the
reading can be argued with — all men are created equal, against the
three-fifths clause. And six decoys: pairs that look like contradictions
and are not, like two amendments carrying the same enforcement sentence.

So: point a tool at the founding documents of a country, cold, and ask
it which rules contradict which.

**Where it stands, with the caveat load-bearing.** Shown the two
commitments alone, on a 27B-class model, it finds **9 of the 11
supersessions** and calls none of the four testable decoys a conflict.
That is an **upper bound, not a score** — the real run shows each pair among twenty-two others, and that
run has never completed. It is the most interesting unfinished thing in
this repo, and `DEMO_PLAN.md` is the honest ledger of what has and
hasn't been measured, including the bars written down before the data
that tests them.

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
(`fixtures/des-moines-noise`) — always naming the model and endpoint that
produced a run, because a number that cannot say which build it describes
cannot be compared with anything. The scripts ship here so you can measure
your own setup rather than inherit ours:

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
- [PRIMITIVES.md](./PRIMITIVES.md) — the design argument: nine
  primitives, the line between mechanism and policy, and eighteen
  technologies of political economy tested against them.
- [DEMO_PLAN.md](./DEMO_PLAN.md) — the founding-documents ledger: what
  is measured, what isn't, and the bars written before the data.

## Build

```sh
cargo build --release      # binary at target/release/canon
cargo test
```

AGPL-3.0-or-later. The format specification is CC0.
