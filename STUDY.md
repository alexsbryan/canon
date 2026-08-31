# The CPR transfer study

**Can you take an arbitrary common-pool resource and get Ostrom governance out
of it without designing anything?**

Elinor Ostrom's eight design principles are what long-enduring commons have in
common. They were derived from alpine pastures, irrigation canals and inshore
fisheries. The bet this repository makes is that they are not about pastures:
that they are a shape a group's decisions can have, that the shape is
mechanical, and that a codebase and a coliving house can have it for the same
reason a canal can.

That bet is testable, and this is the test. It has two legs, and they answer
different questions.

| | question | needs a model | runtime | state |
|---|---|---|---|---|
| **Leg 1** | do the eight compose out of the primitives across institutions of different shapes, with no mechanism written per resource? | no | ~3 s | ten institutions clear it |
| **Leg 2** | point canon at a real community's real documents, cold — is the material for the eight in there, and does extraction reach it? | yes | one sweep | first numbers in |

Neither leg settles the headline question. Leg 1 tests a family of shapes I
chose; leg 2 tests two documents. [What this does not
establish](#what-the-two-legs-together-do-and-do-not-say) is the section to
read before quoting anything here.

```sh
./scripts/cpr-sweep.sh                                   # leg 1, no endpoint
cargo test --test transfer_bar                           # leg 1, as a bar
python3 scripts/ostrom-reach.py maple-house <runs-dir>   # leg 2
```

---

# Leg 1 — the primitives

## The design

Fourteen institutions live in `fixtures/cpr/`. Ten are common-pool resources.
Four are **ablations** of those ten: one line of vocabulary removing one use of
one primitive.

Every one of the fourteen is built from **one spine** — 104 lines under
`fixtures/cpr/_spine/`, the same file for all of them. The spine holds every
grant, scope, rank, policy and step. What a fixture supplies is a
`vocab.json`: actors, boundaries, commitments, proposals, and the **shape** of
the commons — how many people, how many levels, whether a machine or a person
monitors, whether it was forked from anything or founded. Nouns and shape; no
rules.

**The shapes have to differ, and the bar checks that too.** An earlier version
of this study had ten vocabularies over three shapes, which is close to one
institution in ten coats of paint — and would have made every result below a
restatement that renaming strings changes nothing. The bar now refuses a set
whose institutions are not each structurally distinct:

| institution | people | holders | levels | monitor | ancestry |
|---|---|---|---|---|---|
| `torbel-alpine` | 6 | 2 | 2 | person | **founded** |
| `meridian-monorepo` | 9 | 2 | **3** | machine | forked |
| `harbourside-makerspace` | 10 | 2 | 2 | machine | forked |
| `commonwealth-mesh` | 10 | **4** | 2 | machine | forked |
| `alanya-fishery` | 11 | 2 | 2 | machine | forked |
| `crosswalk-coliving` | 11 | 3 | **3** | machine | forked |
| `parkside-allotments` | 11 | 3 | 2 | **person** | forked |
| `tidepool-forum` | 11 | 2 | **3** | **person** | forked |
| `valencia-huerta` | 11 | **5** | 2 | machine | forked |
| `northgate-buildfarm` | **24** | 2 | 2 | machine | forked |

**A vocabulary cannot choose mechanism, and the bar checks that it did not.**
`transfer_bar.rs` refuses any vocabulary that sets a `rule`, `authority`,
`outcome`, `horizon`, `principle` or `strength`. There is no step at which the
author of a new CPR picks a policy, because the spine already picked, once,
identically, for everybody.

**One institution declares two principles inapplicable, and has to prove it.**
`torbel-alpine` was founded rather than forked, so it has no upstream — and
principles 2 and 7 are both about divergence from an upstream. Forcing every
commons in the study to be a fork so the table stayed green would be fitting
the study to the instrument. The escape is deliberately narrow: the reason is
written in the vocabulary, at most two may be declared, a declared principle
must **actually fail** or the declaration is wrong and the bar says so, and
every principle still has to hold in at least eight of the ten institutions.

> **The ten resources.** A makerspace's tools · a coliving building's roof and
> boiler · a monorepo several teams live in · a mesh of pooled machines
> holding one model · shared CI capacity · a community garden's standpipe · a
> moderated forum's attention · a Swiss village's summer pasture · a gravity
> irrigation canal · named inshore fishing sites.
>
> Seven are modern and invented. Three are stylised from Ostrom's own accounts
> and are there as a control: the eight principles were derived from cases like
> those, so an instrument that works on a makerspace and fails on an alpine
> pasture is measuring the wrong thing.
> [PROVENANCE](./fixtures/cpr/PROVENANCE.md) says what they are and are not.

## The eight criteria

The instrument is eight predicates over the replay output, written from the
principles' definitions before any fixture was scored. **None of them mentions
a house, a repository or a canal** — that is what makes the study a study
rather than fourteen demonstrations.

| # | holds when |
|---|---|
| 1 | a holder of the inner boundary may `act`; someone with only wider standing gets `ask-one`; a boundary nobody holds **refuses** |
| 2 | what came from upstream and what this community wrote are both visible and both non-empty |
| 3 | the rule over the inner resource changed, and the change is what decided the next thing |
| 4 | the monitor's adjudication is surfaced by name, its standing lapses with nobody remembering, and its record is positions and no decisions |
| 5 | three occurrences escalate strictly, and a different subject starts at the bottom |
| 6 | the clash is `conflicts` citing both sides, and carrying it knowingly costs exactly **one act** |
| 7 | upstream shipped a new generation and every local commitment is still here |
| 8 | the two levels are held at different depths and decided by different rules |

## What came out

```
Ostrom's eight, over 14 institutions built from one spine
institution                  kind       1 2 3 4 5 6 7 8
alanya-fishery               resource   . . . . . . . .
commonwealth-mesh            resource   . . . . . . . .
crosswalk-coliving           resource   . . . . . . . .
crosswalk-upstream-capture   ablation   . . . . . . x .
harbourside-makerspace       resource   . . . . . . . .
harbourside-no-boundary      ablation   x . . . . . . x
meridian-imposed-rules       ablation   . . x . . . . .
meridian-monorepo            resource   . . . . . . . .
northgate-buildfarm          resource   . . . . . . . .
northgate-unwatched          ablation   . . . x . . . .
parkside-allotments          resource   . . . . . . . .
tidepool-forum               resource   . . . . . . . .
torbel-alpine                resource   . n . . . . n .
valencia-huerta              resource   . . . . . . . .

`.` holds   `x` does not   `n` declared inapplicable, with a reason
10 resources in 10 distinct shapes; 4 ablations, red where each predicted.
```

**The ablations are the part that makes this falsifiable.** Each one names,
in its own `vocab.json` and before the run, the principles it expects to lose.
A study whose instrument reports success on a broken institution has measured
nothing.

| ablation | the one line | predicted | observed |
|---|---|---|---|
| `harbourside-no-boundary` | no grants over the inner boundary | 1, 8 | 1, 8 |
| `meridian-imposed-rules` | the people governed by the rule never change it | 3 | 3 |
| `northgate-unwatched` | the monitor's grant carries no end date | 4 | 4 |
| `crosswalk-upstream-capture` | upstream holds standing and uses it | 7 | 7 |

## The counterfactual, and what it is worth

`canon replay --policy X` re-decides a whole history under a rule the
community did not adopt. The **set** of `(step, field)` pairs that come out
differently is identical across all ten institutions:

```
--policy default        10 of 45 decisions change, one signature, 10 institutions
--policy consent         8 of 45 decisions change, one signature, 10 institutions
--policy subsidiarity    9 of 45 decisions change, one signature, 10 institutions
```

*(An earlier draft reported 27 / 24 / 22 here. Those were changed FIELDS —
`outcome`, `authority`, `because` and `rule` each counted separately for one
decision — which roughly doubled every figure. The counts above are decisions,
which is the unit the sentence claims. The signature is still computed over
field pairs, because as a fingerprint it should be as sensitive as possible;
only the headline count changed.)*

**Read this narrowly.** It says *where* a policy change lands is a property of
the policy and the spine, not of the nouns or the shape — across 6 to 24
people, two levels or three, forked or founded. It does **not** say the
decisions themselves are identical, and it is not by itself evidence that the
eight principles generalise. The ablation table is that.

The obvious objection is that a metric returning the same answer for
everything measures nothing, so the test carries its own null control: two of
the four ablations change structure a policy reads, and their signatures
**must** differ from the baseline or the test fails. They do. The other two
ablations produce the baseline signature, which is the honest limit of the
metric's resolution.

## What an institution cost

| | |
|---|---|
| shared spine, written once | **104 lines** |
| mechanism written per new CPR | **0 lines** |
| naming and shape written per new CPR | 107–149 lines of `vocab.json` |
| an ablation | **8 lines** |
| all fourteen institutions | 1,400 lines, and no mechanism |

**The zero is weaker than it looks and should be read as a description, not a
guarantee.** No test fails when a commons needs new mechanism: if one did not
fit, the spine would be edited and this counter would still read zero. What
the bar does enforce is that no vocabulary names a rule, that every
institution runs the same steps in the same order apart from the ones a
declared shape turns on, and that the ten are ten distinct shapes. Those are
real constraints. "Zero mechanism per CPR" describes the fixtures that exist.

`expected.json` is **predicted** by `scripts/cpr-build.py`, in Python, from
the policy semantics — never recorded from a run — and `canon replay` compares
the two. Calling that a test between *independent* implementations would be
too strong: the predictor was written by reading `policy.rs`, so it is a
transcription into another language rather than an independent derivation, and
on the one occasion the two disagreed the predictor was the thing corrected.
It catches transcription-level drift and arithmetic, not a shared
misconception. Of 26 assertions per fixture, two are recorded rather than
predicted: a draw's `seats` and `seed`, which are hashes of the log.

## Findings

Two of these are defects the study found in its own instrument, and both were
found by widening the set rather than by thinking harder.

**The `who` verb did not name the level.** The nesting criterion failed on two
of ten resources — not because nesting was missing, but because `canon who`
printed deciders deepest-first without saying at what depth each held, so a
two-level boundary and a one-level one render identically whenever the narrow
holders happen to sort first alphabetically. `valencia-huerta` has holders
`amparo` and `bernat`; both sort ahead of every other member. `who` now also
emits `holders`, the deepest level on its own, which is the set subsidiarity
actually routes to.

Two honest readings of that episode exist and the second is not flattering: I
changed both the criterion and the tool until the table went green, and
nothing had pre-registered the criterion in a form I could not revise. The
depth gap is real either way, and `holders` is the right thing for `who` to
report — but a green table was the target throughout, and that is worth
knowing when reading the rest.

**The monitoring criterion assumed monitors are machines.** Ostrom's fourth
principle is specifically about monitors drawn from, or accountable to, the
appropriators — people, usually. The first version of the criterion required
the monitor's adjudication to appear in `unattended`, which reports
adjudications with **no person behind them**. The moment three institutions
started monitoring with one of their own people on a rotation, the criterion
failed them for doing the more Ostrom-conformant thing. The criterion now
tests attribution with both branches: a machine's adjudication must be
surfaced by `unattended`; a person's must **not** be, because it already
carries their name. Three of the ten now monitor with a person, and both
branches are exercised.

**Nesting is not separable from boundaries, and that is the correct result.**
`harbourside-no-boundary` loses principles 1 *and* 8, predicted in advance,
because in canon the eighth principle is not a feature — it is what having
boundaries at two depths *is*. `PRIMITIVES.md` claims dotted scopes give
Ostrom's eighth principle for free; the ablation is what that claim looks like
when you take the scopes away.

**Principle 7 is an affordance and the ablation shows exactly why.**
`crosswalk-upstream-capture` grants an upstream actor standing over the fork,
and it retracts three of the five commitments the community wrote. Nothing in
the format stops it. The README marks principles 2 and 7 `affordance` rather
than `mechanism`; this is the demonstration that the mark is honest. A CC0
format and an opt-in upgrade protect a fork from being *upgraded* out of its
own rules. They do not protect it from having granted somebody standing.

## What leg 1 still does not test

The spine fixes more than it varies, and the list is short enough to print.

- **One monitor, one unheld scope, eight commitments, one ladder, and the
  ladder always at depth two.** A commons with three monitors, or none, or
  with its escalation ladder on the inner resource, is not in the study.
- **Only 19 of the 31 named steps are read by the eight criteria.** The draw,
  the entrenchment pair, the reversibility pair and the silence check are
  asserted only by `expected.json`, which the generator writes — so for those
  the check is the predictor agreeing with the engine, not a principle being
  measured.
- **I wrote the spine and all ten vocabularies, knowing the spine.** A commons
  that resisted the shape would not have been written. Nothing internal to the
  study can correct for that; only somebody else adding an institution can,
  which is what `fixtures/cpr/README.md` is for.
- **Ten shapes is not "any CPR."** It is ten points in a space whose axes I
  also chose.

# Leg 2 — the documents

Leg 1 is about the primitives and settles a claim about them. It does not
touch the harder question, and nothing in it should be read as touching it:
**whether pointing canon at a real community's actual mess yields any of
this.** That needs a model, and it is measured the way the founding demo is
measured — against vendored corpora, naming the model and endpoint.

## The design

For each corpus, a hand-written `ostrom-anchors.json` says, per principle,
whether the **document** carries material for it, and the smallest phrase that
material turns on. Both manifests were written from the corpora before any run
was scored against them.

Then two numbers per principle, from the run artifacts `draft-bar.sh` already
produces — so a sweep paid for once answers both questions:

- **ceiling** — the passage survived into the candidate set. canon cuts the
  quote out of your file itself, so this is mostly a fact about chunking.
- **own words** — the proposed rule a person would actually review carries the
  principle. This is the number that means something.

**The two corpora are chosen to disagree.** `maple-house` is a house's own
charter and its own recorded decisions: the people bound by the rules wrote
them. `des-moines-noise` is a municipal ordinance: the people bound by it did
not write it and cannot change it by any procedure the text describes.
Ostrom's argument predicts those two documents carry different principles, and
the manifests are that prediction written down before scoring.

| corpus | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
|---|---|---|---|---|---|---|---|---|
| `maple-house` — self-governed | ● | ● | ● | ● | ◐ | ● | ● | ● |
| `des-moines-noise` — externally imposed | ● | ● | ○ | ○ | ● | ○ | ○ | ● |

● material in the document · ◐ partial · ○ none

## What came out

Three runs each, at `http://localhost:9741/v1`.

**The two corpora are not attributed equally, and the weaker one is a
warning.** The `maple-house` artifacts record the resolved model,
`Qwen3.8-27B-UD-Q6_K_XL`. The `des-moines-noise` artifacts record only the
alias they were called with, `primary` — and that alias has since moved: on
this machine today it resolves to `Qwen3.6-35B-A3B-UD-MTP-IQ4_NL`. So the
municipal-code numbers below cannot be attributed to a specific model after
the fact, and are reported as "whatever `primary` was on 2026-08-19". The
directory is named `qwen-27b` but a directory name is not provenance.

| | `maple-house` | `des-moines-noise` |
|---|---|---|
| principles the document fully carries | 7 of 8 | 4 of 8 |
| partly carried | 1 (graduated sanctions) | 0 |
| not carried at all | 0 | 4 |
| **reached, in the proposal's own words** | **7.00 of 7** | **3.00 of 4** |
| anchor recall, own words | 33/33 (1.00) | 9/18 (0.50) |
| chunks yielding a candidate | 24/24 | 32/34 |

**Two caveats before reading those numbers, both of which cost them
something.**

The scorer also reports a *ceiling* — matched against the source quote rather
than the model's own wording — and that number is worthless here. All twelve
anchor phrases appear verbatim in their own section, and chunk coverage is
24/24, so with canon cutting the quote out of the file the ceiling is
guaranteed before the model does anything. It measures chunking. Only the
own-words column is reported above, and the scorer now prints the coverage
caveat next to it.

The own-words column is softer than it looks too. Ten of the anchor
alternatives are single common words — `backyard`, `logbook`, `treasurer`,
`agenda`, `study` — which a close paraphrase of the section retains almost by
default. "Reached" here means the proposal a reviewer would see contains the
phrase the principle turns on. It does not mean the model understood the
principle.

**A house charter nobody wrote with Ostrom in mind carries material for seven
of the eight principles outright, and part of the eighth.** Extraction reached
all seven in its own words. Principle 7 came out as *"Recorded decisions amend
or extend the Charter starting from the date they are made"* — the
right-to-organise material, correctly, unprompted.

The eighth is graduated sanctions, and the manifest marks it `partial` before
any scoring: the corpus has exactly one sanction, a flat twenty-five dollar
late fee, and nothing in it escalates. A house adopting only what this
document says would have a ladder with one rung. The scorer reports that line
separately and never folds it into the headline, because a hit there is a hit
on the half that is present.

**The one outright miss is specific and useful.** On the municipal code,
principle 5 was not reached at all. The ordinance carries a real two-rung
ladder in one passage: an order to halt first, an injunction if the sound has
not abated. Extraction produced 85 candidates and **one** from the two chunks
that hold it, about decibel limits. No candidate in any of the three runs
mentions the halt, the injunction or the abatement. The single
enforcement-escalation passage in a 6,073-word document was dropped.

**And the difference between the two profiles is mine, not the model's.**
Which principles each document carries was decided by hand, before scoring,
when I wrote the manifests. What the runs test is whether extraction can reach
the anchored material in both — not whether a self-governed corpus differs
from an imposed one. That prediction is written down and pre-registered, but
it has not been tested by anything here.

## The bars, for the next sweep

Written now, before the next run, and deliberately not fitted to the two
numbers above — those were produced by re-scoring artifacts that already
existed, with the manifests fixed beforehand but these thresholds not. Treat
them as the first measurement, not as a cleared bar.

| bar | value | consequence |
|---|---|---|
| Kill — own-words reach on a self-governed corpus | < 0.50 | leg 2 is abandoned; extraction cannot see governance material |
| Publish — own-words reach on a self-governed corpus | ≥ 0.75 | the leg-2 claim ships |
| Discrimination | anchored-principle reach within 0.25 across the two corpus kinds | below this, the difference between the profiles is extraction's difficulty and not the document's nature |
| Invention floor | zero candidates asserting a principle the manifest marks `absent` | checked by hand; an extractor that finds collective choice in a noise ordinance is inventing |

## What leg 2 does not establish

That a community pointing canon at its own folder gets Ostrom governance.
Reachability is a ceiling: it says the passage survived into a candidate set a
person still has to review one at a time. Between a reached anchor and a
governed commons there is a review, a set of grants nobody extracted, and a
policy somebody had to choose. Leg 1 says those are cheap. It does not say
they are free.

---

# What the two legs together do and do not say

**Shown.** Ten institutions of ten distinct shapes — 6 to 24 people, two
levels or three, monitored by a machine or by a person, forked or founded —
compose Ostrom's eight design principles out of canon's nine primitives with
no mechanism written per institution; where a policy change lands is the same
in all ten; and the instrument goes red on four single-variable ablations
exactly where each predicted, including two that were predicted to lose two
principles or the wrong one.

**Measured once.** That the documents one real self-governing community
already had carry material for seven of the eight principles outright, and
that a 27B-class local model reached all seven in its own words. One corpus,
three runs, one model.

**Not shown, and worth stating plainly.**

- That this generalises to commons whose shape is not in the ten. I chose the
  ten shapes and I chose the axes they vary along.
- That the criteria are the right operationalisation of Ostrom's principles.
  Two of them were wrong on first contact with a wider set, and both were
  fixed by me, after seeing them fail.
- That extraction reaching an anchor means a community gets governance.
  Between a reached anchor and a governed commons there is a review, a set of
  grants nobody extracted, and a policy somebody has to choose. Leg 1 says
  those are cheap. It does not say they are free.
- That any of this survives contact with a group that has not used it. The
  README says another group hasn't used this yet, and this study does not
  change that sentence.

# Reproducing

```sh
cargo build
./scripts/cpr-sweep.sh                    # leg 1 end to end, no endpoint
cargo test --test transfer_bar            # leg 1 as a bar, in cargo test
python3 scripts/cpr-build.py --all --pin-draw   # regenerate every fixture

python3 scripts/ostrom-reach.py maple-house fixtures/maple-house/runs/qwen-27b
python3 scripts/ostrom-reach.py des-moines-noise fixtures/des-moines-noise/runs/qwen-27b
./scripts/draft-bar.sh 3                  # your own runs, your own endpoint
```

Leg 1 needs no endpoint and no network and takes about three seconds. Leg 2
scores whatever runs you have; making new ones is a sweep against your own
model, and the numbers above are ours and not yours.

- [fixtures/cpr/README.md](./fixtures/cpr/README.md) — the fourteen, and how to add one
- [fixtures/cpr/PROVENANCE.md](./fixtures/cpr/PROVENANCE.md) — what they are, and are not
- [PRIMITIVES.md](./PRIMITIVES.md) — the nine primitives and the line under them
- [DEMO_PLAN.md](./DEMO_PLAN.md) — the founding-documents ledger
