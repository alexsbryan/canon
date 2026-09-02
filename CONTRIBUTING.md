# Contributing to canon

Every path in this repository is open to pull requests. There's no list of
permitted directories and no "core is closed" tier — what decides a change
is the suite, which runs in about six seconds and says the same thing on
your machine as it does on mine.

That's the whole policy. The rest of this file is how to make it easy.

Two things are worth knowing before you start:

- **The project's own rules are in a canon**, in `.canon/acts.jsonl`, and
  this file links into them. `canon list` prints them; `canon why <id>`
  says where one came from. If a rule here and a rule in the canon
  disagree, the canon is the one under version control with a reason
  attached.
- **It's early.** Another group hasn't used this yet. The most valuable
  thing you can send is not a patch — it's what happened when you pointed
  it at your own mess.

For anything security-related — including a way something could leave a
machine unexpectedly — don't open a public issue. See [SECURITY.md](./SECURITY.md).

## Getting set up

One clone, one workspace, no native dependencies beyond a Rust toolchain.
`rust-toolchain.toml` pins the version, so `rustup` fetches the right one
by itself.

```sh
git clone https://github.com/alexsbryan/canon
cd canon
cargo build --release          # binary at target/release/canon
./scripts/install-git-hooks.sh # the pre-push gate, shared and version-controlled
```

Then point it at something and see it work:

```sh
./target/release/canon replay fixtures/fernwood-commons --brief
```

That's the governance layer over a worked twelve-person house, in
milliseconds, with **no model and no endpoint**. If it prints Ostrom's
eight principles as a table, your build is good.

A model is only needed for four verbs — `draft`, `check`, `tensions`,
`rebase` — and only those need an endpoint. Everything else, which is most
of the tool, is pure. [The README](./README.md#what-needs-a-model-and-what-doesnt)
draws the line.

## The fastest way in

**A fixture.** It's the contribution this project needs most and the one
that's most mechanical to review, because a fixture either replays or it
doesn't.

There are two kinds, and they answer different questions:

- **A worked canon** — `fixtures/fernwood-commons/` is the model. A
  `scenario.jsonl` of questions, an `acts.jsonl` of the record they're
  asked against, and an `expected.json`. It replays deterministically with
  no endpoint, because the positions a model would have produced are
  written into the fixture directly. If yours needs a model to replay, the
  split between extraction and decision has been broken.
- **A scored corpus** — `fixtures/maple-house/` is the model. A document,
  a `truth.json` labelling what's actually in tension, and an
  `extraction-anchors.json` naming the smallest phrase each tension turns
  on. This measures ingest, so it needs an endpoint to produce a run and
  none to score one.

Either way, a fixture carries a `PROVENANCE.md` naming where it came
from — the upstream commit and the sha256 of every file copied. Vendored,
never depended on across repositories: a standalone `git clone` has to
work, and that's the property the whole tool is built around. Read
[`fixtures/maple-house/PROVENANCE.md`](./fixtures/maple-house/PROVENANCE.md);
it's short and it explains why each half is there.

**Other things that are worth as much as code:**

- **Point `canon draft` at your own notes and say what it got wrong.**
  With `--dry-run` it writes nothing and still leaves the run artifact
  behind. The proposals it invents and the real rules it walks past are
  both findings.
- **Run the bars against your own endpoint and report the numbers.** No
  ingest accuracy figure is quoted in the README on purpose — the last
  published ones predate a change to how contradictions get detected.
  `./scripts/draft-bar.sh 3` produces runs and
  `./scripts/score-bar.sh maple-house <runs-dir>` scores them. Name the
  model and the endpoint; a number without those is a number about nothing.
- **Documentation.** Typos, wrong commands, stale paths, a walkthrough
  that doesn't work on your machine. Send the PR, no issue first.

## The development loop

The gate is local and it is fast enough that there's no reason to skip it.

```sh
./scripts/pre-push.sh     # everything CI runs, in about ten seconds
```

`install-git-hooks.sh` wires that to `git push` through `core.hooksPath`,
so the gate is a reviewed file in the tree rather than something each
person copies into their own `.git/hooks/`. CI then runs the identical set
on a clean checkout — it's a confirmation, not the authority.

What both of them check:

| Gate | What it means |
|---|---|
| `cargo fmt --all --check` | rustfmt, on the pinned toolchain |
| `cargo clippy --all-targets -- -D warnings` | no warnings, not a ratchet — the tree is small enough to keep at zero |
| `cargo test --workspace` | 398 tests, ~6 seconds |
| `./scripts/docs-gate.sh` | every repository path a narrative document links to still resolves |

Two of those tests are worth calling out, because they gate the *design*
rather than the code:

- **`adequacy_bar`** is the stopping rule for the format, enforced. Every
  op has to be named by a technology of political economy and listed in a
  census with the primitive it serves and the composition that did not
  reach it. **A new op fails the suite until somebody writes down why it
  exists.** If you're adding one, that third column is the work; the code
  is the easy part.
- **`governance_bar`** checks that the decision layer stays pure — no
  filesystem, no network, no model under `canon-core`. Its purity is the
  dependency list, not discipline, so the way to break it is to add a
  dependency.

**What CI does not check: ingest accuracy.** It needs an endpoint and a
model whose numbers move between runs, and a gate that goes red for the
weather teaches people to ignore gates. That's a
[recorded silence](#the-rules-this-project-runs-on), not an oversight. The
bars run by hand.

## What makes a change easy to merge

Not a checklist — just what tends to earn a quick yes.

- **A test that pins it.** One that fails without your change. The suite is
  fast and cheap to add to; most of it is in-module `#[cfg(test)]` next to
  the code it covers.
- **The reason, not just the rule.** This is a project about keeping the
  why. A commit message that says what the old behaviour cost is worth
  more here than in most repos.
- **Green.** Red means not-ready-yet, not that you did something wrong.
  If CI looks confused, say so on the PR.

Two habits carry weight:

- **Write for the next reader.** Match the surrounding naming and idiom.
  The comments in this tree explain why a thing is the way it is, not what
  the line does — keep that ratio.
- **Don't destroy anything.** The record is append-only, everything is
  revertible including a revert, and a contradiction carried on purpose is
  something to record rather than a bug to clean up. Code that quietly
  drops a fact is the one kind of change this project can't take.

## The rules this project runs on

canon governs itself. `.canon/acts.jsonl` is committed and holds this
repository's own rules — the same file format, the same verbs, no special
case.

```sh
canon list                      # what's in force, and what's proposed
canon why can-96d951ae3378      # the stopping rule for new ops, and where it came from
canon open                      # what nobody has decided yet
canon ratification show         # how a proposal becomes a rule here
```

Three things in there are worth reading before you open a PR:

**The open questions are real.** *Nobody has decided what earns a grant* —
a second person holding standing over a scope is the next actual
governance event in this project, and there's no rule for it. If you want
to steer that, the question is the place.

**Three silences are recorded**, so `canon check` says "decided against"
rather than "gap": a plugin system for ops, a stale-issue bot, and an
ingest-accuracy gate in CI. Each carries the reason. Proposing one isn't
off-limits — you'd be arguing with a written-down position instead of
guessing at one, which is the point.

**`canon.docs` is under a fourteen-day consent rule**, and it binds the
steward rather than you. A documentation rule nobody objects to within two
weeks is a rule. You can watch this work right now: two rules in this
canon are `PROPOSED, not yet a rule` — mine, waiting out my own window.

## Commits and pull requests

- Imperative mood is appreciated; `type(scope): summary` runs through the
  history where it fits. Don't overthink it.
- The PR template is short on purpose. CI does the mechanical checking, so
  it just asks what changed and how you looked at it.
- Rebasing on `main` keeps history readable but isn't a blocker.
- `.canon/acts.jsonl` is append-only, so it merges additively. If you do
  hit a conflict in it, `canon merge-driver` resolves it — run
  `canon merge-driver` with no arguments for the one-time setup.

## Licensing

canon is free software under [AGPL-3.0-or-later](./LICENSE) and stays that
way. **The record format is separate: [SPEC.md](./SPEC.md) is CC0.**
Adopting the format must not be a lock-in decision, so an independent
implementation owes this project nothing.

Contributions are covered by a **Contributor License Agreement**
([CLA.md](./CLA.md)), the standard Harmony agreement (v1.0). In plain
terms: you keep your copyright, you grant the maintainer a broad license
including the right to offer the project under another license alongside
the public AGPL, and in return the project is guaranteed to stay open
source. Everyone signs the identical document, so no contributor ends up
holding rights another doesn't.

You sign once, electronically, the first time you open a pull request — a
bot posts a link and records the signature against your GitHub account. If
you're contributing as part of your job, use the Entity agreement in the
same file, or have your employer sign it, since your employer may own the
work.

Please don't paste in code you don't have the right to license this way.

## Code of Conduct

Participation is covered by the [Code of Conduct](./CODE_OF_CONDUCT.md).
Be kind.
