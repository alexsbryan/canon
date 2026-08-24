# canon

A decision log that knows what it currently says.

Hold a body of commitments. Record what was decided about them, and why.
Ask whether a proposal sits with or against them.

Works for one person, one codebase, or one household.

```sh
canon init
canon add "Survey what exists and prove it cannot serve before building."
canon add "Ship the smallest thing that closes the issue."
canon list
```

Later, when something changes:

```sh
canon supersede can-4f19 "Prefer extending an existing helper." -m "PR #612 discussion"
canon why can-9b02          # what this replaced, when, and the reason given
```

And when two commitments genuinely conflict and you are keeping both:

```sh
canon accept can-a81 can-3d2 -m "reliability is how I earn the autonomy, for now"
```

Nothing is ever force-resolved and nothing is destroyed. A contradiction
you are carrying on purpose is a first-class state, not a bug to be
cleaned up.

## Why it is shaped like this

**One file.** Everything lives in `.canon/acts.jsonl`, append-only. It
diffs, so git gives it history for free. It greps. You own it — leaving
is deleting a directory.

**Current state is derived, never stored.** What is live, what replaced
what, which contradictions you are carrying: all of it is a pure fold
over the log. `canon-core` has no filesystem or network dependency at
all, which is enforced by its dependency list rather than by discipline.

**Nothing is destroyed.** Every act is revertible, including a revert.

**Most of it needs no model.** `add`, `list`, `why`, `supersede`,
`retract`, `accept`, `dismiss`, `undo`, `question`, `open`, `log`,
`share`, `adopt`, `diff --upstream` and the merge driver are all the
fold — and so is every governance verb: `who`, `grant`, `withdraw`,
`scope`, `policy`, `position`, `decide`, `rank`, `horizon`, `overdue`,
`silence`, `voice`, `leave`, `draw` and `replay`. Only `check`,
`tensions`, `draft` and `rebase` call a model, and they take any
OpenAI-compatible endpoint:

```sh
canon config set endpoint http://localhost:8080/v1   # any llama.cpp server
```

A model call is refused unless the endpoint is on this machine, and
`--allow-remote` is how you say otherwise. Every call names the endpoint
it used.

## Asking whether something fits

```sh
canon check "add a second scorer for the adjudication cache"
```

```
CONFLICT
  can-0e50f4ee  "One implementation per threshold, scorer, schema and key."
                asserted 2026-08-21, in force, never superseded
                because: the proposal adds a second scorer
```

Exit 1. The answer always cites a rule you can read, because a bearing
that names no commitment or gives no reason is refused before it is
rendered — the difference between *the agent misread the rule* and *the
rule is wrong* is a correction versus an amendment, and you cannot tell
them apart without the citation.

A canon on the `personal` profile never renders a verdict and never
returns exit 1. It reports which commitments have a stake and which way
each pulls, including contradictions you already chose to carry. A tool
that ruled on someone's inner life would do harm the codebase profile
cannot.

## Two verbs is the whole tool

```sh
canon draft --from ~/house-docs     # read what you already have
canon check "<something you want to do>"   # does it clash with one?
```

Nobody writes one of these a rule at a time. The normative content already
exists — a handbook, two years of meeting notes, the channel where things
actually get decided — so onboarding is pointing at that folder, and
`canon add` is for the one you think of afterwards.

`canon --help` is seven verbs, not forty-seven, and `canon init` prints the
ones you need before you have read anything. Everything below this line is
optional and stays out of the way until a group asks for it. `canon help
all` is the one door.

## How you decide is in the canon, not in the tool

`canon` holds no opinion about how many objections make a conflict, who
may decide what, or what happens when nothing bears on a proposal. Those
have defensible ranges of answers, communities differ, and a library that
answered them would be a product.

Until a group adopts one, the tool does not mention any of it — `check`
names the rule you are up against and the act that would settle it, and
stops. The moment somebody runs `canon policy set`, it starts also saying
what you may then do, because by then that is something the group decided
rather than a restatement of the verdict above it:

```
THIS NEEDS AN AMENDMENT
  it runs against a rule the house already has:
  ...
  amend it:  canon supersede can-dbdc4161 "<the new rule>" -m "<why>"

not under this policy
  consent: 1 reasoned objection(s); one is enough
```

The rule itself is an act like any other:

```sh
canon policy set consent --cautious -m "One reasoned objection stops a thing. \
  Anything we cannot undo is not decided by silence."
canon who house.kitchen        # answerable without asking a person
canon overdue                  # what has gone past its date
```

Defaults are extraordinarily sticky and most adopters never change them,
so whatever ships as default *is* the governance for nearly everyone.
Calling that loosely held describes an intention rather than an outcome.
The mitigation costs nothing because the machinery already exists: a
policy in the ledger is subject to `check`, to tension detection, to
`supersede` with a rationale, and to a visible diff against the lineage
it was forked from. A default you can run `canon why` against is loosely
held; one in a TOML file is not.

**Ostrom's eight design principles are the acceptance test**, and they
clear on six mechanisms and two affordances — the same marks in a
twelve-person house and in a codebase. `fixtures/fernwood-commons` and
`fixtures/eleven-principles` replay one scenario per principle in under
half a second with no endpoint, because the decision layer is pure:

```sh
canon replay fixtures/fernwood-commons
canon replay fixtures/fernwood-commons --policy default   # the counterfactual
```

The second form is worth having on its own: *what would another rule have
done to the last six months?* is the question a group actually has before
changing how it decides.

`PRIMITIVES.md` is the argument — nine primitives, the line between
mechanism and policy, eighteen technologies of political economy tested
against them, and the threat model for the one that needed designing.

## The agent surface

```sh
canon mcp    # stdio MCP server: canon_list, canon_why, canon_open, canon_check
```

Everyone running agents has the same problem: the agent does not know the
house rules, and pasting them into the prompt saturates. This lets it ask.

**Every tool is a read.** There is no tool that writes an act — not
permission-gated, absent. Amending requires the CLI, run by a person. An
agent that thinks something should be recorded says so in chat, as a
command you can run. The canon is what your agents are measured against;
an agent that can edit it is grading its own work.

Any MCP client that speaks stdio can use it. In Claude Code:

```sh
claude mcp add canon -- canon mcp
```

Or by hand, in whatever config your client uses:

```json
{ "mcpServers": { "canon": { "command": "canon", "args": ["mcp"] } } }
```

It finds the canon the same way git finds a repository: nearest `.canon`
walking up from the working directory, falling back to `$HOME/.canon`.
Set `CANON_DIR` to point it somewhere specific.

## Starting from what you already wrote

```sh
canon draft --from ~/house-docs   # a folder, recursively
canon draft --from-git --since 1y # or the commit messages
cat whatever | canon draft --from - --as '#eng' --dry-run --json
canon draft --resume              # finish a long review later
```

Point it at a folder. **There is no format list.** Anything under it that
is text gets read, whatever it is called — `.org`, `.rst`, `.eml`, a
`NOTES` file with no extension, a transcript someone pasted into a `.log`.
A canon lives in whatever its group already writes in, and a reader that
knows four extensions works on the corpora its authors happened to test
against.

Three things a walk passes over, each reported and each with a way round
it: files the project itself calls generated (`git check-ignore`, so the
authority is your own `.gitignore` rather than a list of build directories
we guessed at — `--include-ignored` reads them anyway), structured data
that holds no conversation (a lockfile read as prose proposes commitments
cited to dependency names), and anything too big to be writing. Naming a
file directly reads it regardless — a walk is a guess about intent and
`--from thatfile` is not.

**And it tells you what it did not read** — how many files and why. A
directory with some readable files used to drop the rest in silence, so a
Slack export sitting beside three documents was never opened by anyone and
two rules that existed only in chat were never seen.

`--from -` reads stdin, which is the whole integration surface. Anything
that can emit text can feed a canon, and `canon` carries no connector, no
vendor schema and no endpoint of its own. `--as` names the source so the
citation reads `#eng-decisions:3-4` rather than `stdin:3-4`, which matters
most on a live feed where the passage has scrolled away by the time anyone
reads the candidate. Pipe it with `--dry-run --json`, then
`canon draft --resume` to review what it found with no second model run.

### The shape canon reads best

Every candidate cites a POSITION in its passage, so a passage has to have
positions. `canon` finds them two ways, in this order:

1. **Prose** — sentence ends, plus the units a document marks itself: `|`
   table rows, `#` headings, `>` quotes, `- * +` list items, `(a)`, `1.`
2. **Lines** — one unit per line, used when prose splitting found no
   structure at all.

The second exists because YAML `key: value`, a CSV row, a line of code and a
log line match none of the first. Before the fallback, a passage of any of
them was ONE unit: the model saw a single giant `[1]`, cited it, and the
"quote" it got back was all 1,819 characters of the passage. In range,
verbatim, and useless as evidence. `draft` now says which basis it used —
`3 of 24 passage(s) are line-oriented` — and the run artifact records it
per passage, because a citation into a table row means something different
from a citation into an argument.

**So the shape that always works is: one unit per line, a blank line between
passages.** That is what the built-in chat reader emits (`> who: said what`,
blank line on a conversation gap), and it is the whole contract. An agent
piping anything — a diff, a ticket export, a transcript, a system nobody
has heard of — gets first-class citations by rendering into that shape in
about five lines, and needs nothing from `canon` to do it. Pipe something
unshaped and it still works; you just get line units and a note saying so.

**A feed you read twice does not ask twice.** `.canon/seen` records the
passages already extracted from and the candidates you declined, so
re-pointing at a growing channel costs the new material rather than the
whole history, and a rule you said no to is not proposed again tomorrow.
It is ingest hygiene, not part of the canon: nothing in it is an act,
`check` never consults it, and deleting it costs a re-extraction and
changes no commitment. `[s]kip` records nothing — skip means not now, and
only `[r]eject` means no.

Chat is not prose and is not chunked as though it were. Messages are
rendered with who said them and cut into bursts on a time gap, so a
citation quotes the exchange a rule was actually decided in:

```text
RULE      Recycling goes out Sunday night.
          slack-export/general/2026-08-01.json:1-12

  > sam: reminder the recycling goes out sunday night not monday
  > mira: ^ this has bitten us three times now, can we make it a rule
  > sam: yes. recycling out sunday night.
```

**Three kinds, because a group's normative content is three shapes.** A
meeting note saying "nobody has ever said who looks after the allotment"
is recording a QUESTION; one saying "decided not to make a rota — it would
turn a kindness into a duty" is recording a SILENCE. An extractor that
could only mint commitments dropped both on the floor, and what it handed
back was a list of rules rather than a canon.

**Every candidate carries the passage it came from, or it is not shown**:
the extractor answers with the POSITION of the sentence it read, and
`canon` cuts the quote out of the source itself — so a citation that is not
in your document is not something that can happen. A drafted commitment
with no citation is a model inventing a value you never held. A silence
with no stated reason is refused for the same reason: it cannot be told
apart from having forgotten.

There is no `--accept-all`. A canon adopted wholesale is disengagement at
t=0, so accepting one at a time is what makes onboarding the first
governance session — and `--resume` is what makes that survivable, picking
a review back up from the stored run with no second model pass.

### What it actually finds

A vendored house charter with eleven planted contradictions and seven
pairs that look like contradictions and are not. Three runs of one build
against that document, scored by replay from the persisted run artifacts
rather than by asking a model a second time:

```text
recall     0.64      7 of 11 planted tensions
precision  0.47
decoys     1 of 7 flagged
reachable  11 of 11  survived extraction and dedupe
```

The spread across the three runs is zero, which proves the pipeline
reproduces and not that the number is stable under anything else.

**These are train-contaminated and are not a held-out estimate.**
`truth.json` splits the eleven into train, dev and test and calls test
sacred; the work that produced this build looked at test-split misses
while choosing what to fix. What survives that is mechanism — that dedupe
was folding two contradicting rules into one is a fact readable in the
artifact, not a score. The rates are not, and one document on one model is
not a general claim about yours.

```sh
# score the evidence in this repo
./scripts/score-bar.sh maple-house fixtures/maple-house/runs/qwen-27b

# or produce your own, against your own endpoint
sh scripts/draft-bar.sh 3
```

There is a second corpus. `fixtures/des-moines-noise/` is Article IV of the
Des Moines municipal code interleaved with two ordinances that amend it, and
its labels are the council's own "Section X is amended" pointers rather than
anything written for a bench. It is scored the same way and is deliberately
kept separate — a mean over two corpora is a number about neither.

`runs/qwen-27b-before-fold-guard/` is the same document through the build
that preceded it, kept so the difference is checkable and not just
claimed: dedupe folded the 10 PM rule into the 11 PM one and two planted
tensions became unreachable.

## Sharing

```sh
canon share                 # a block you can paste into a chat thread
canon adopt --paste         # read one back
canon diff --upstream       # how you have diverged from what you adopted
```

For most communities, pasting is not a phase before something better —
it is how they will always share. A snapshot carries current state and
drops rationales and supersession history: enough to adopt, not enough to
audit, which is the right trade for a chat thread. A block edited after
it was shared is refused rather than adopted under the sender's name.

`adopt <url>` and `upgrade <gen>` clone a lineage so nobody has to type
git. `rebase --onto <url>@<gen>` maps your changes onto a newer base and
tells you how much of your law survives before you commit to the move.

## Status

Early, and honest about it. Every verb in the spec is implemented and
tested. What has not happened yet is anyone else using it.

## Format

[SPEC.md](./SPEC.md) — released **CC0**, public domain, so adopting the
format is not a lock-in decision. The tooling here is AGPL-3.0-or-later;
the record format belongs to nobody.

Larger tools read the same file.

## Exit codes

`0` supported · `1` conflicts · `2` unaddressed, or a usage error ·
`3` cannot judge

`--json` puts data on stdout and logs on stderr, so this drops into CI
and agent tooling without a wrapper.

## Build

```sh
cargo build
cargo test
```

## License

AGPL-3.0-or-later. The format specification is CC0.
