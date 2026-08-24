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

`canon add "<a rule>"` and `canon check "<an idea>"`. That is what you
hand the next person. `canon --help` is six verbs, not forty-seven, and
`canon init` prints the three you need before you have read anything.

Everything below this line is optional, and stays out of the way until a
group asks for it. `canon help all` is the one door.

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
canon draft --from ./notes        # or --from-git --since 1y
```

`draft` extracts candidate commitments from text you already have and
offers them one at a time. **Every candidate carries the passage it came
from, or it is not shown**: the extractor answers with the POSITION of the
sentence it read, and `canon` cuts the quote out of the source itself — so
a citation that is not in your document is not something that can happen.
A drafted commitment with no citation is a model inventing a value you
never held.

There is no `--accept-all`. A canon adopted wholesale is disengagement at
t=0, so accepting one at a time is what makes onboarding the first
governance session.

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
