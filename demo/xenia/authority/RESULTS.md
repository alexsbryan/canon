# Results — two optional institutional agreements

2026-11-02. Canon `f63441a`, unchanged core and contract. The agreements and
hypotheses were declared in [README.md](README.md) and [the cases](../../../crates/canon-institution/tests/fixtures/cases.json)
before execution. These are optional institutional policies, not universal
requirements of the record format.

## Mechanical results

The deterministic witness observed each real CLI operation's before/after
state and append-only log. [The retained report](results/report.json) includes
the commands, outputs, appended acts and provenance hashes.

| Case | Desired institutional effect | Observed in canon | Agreement holds? |
|---|---|---|---|
| Withdraw own pending proposal with `repeal` | Withdraw it | Withdrawn | yes |
| Repeal own human-adopted rule as drafting agent | Keep it in force pending house assent | Repealed immediately | no |
| Undo own pending proposal | Cancel it | Cancelled | yes |
| Undo own human-adopted rule as drafting agent | Keep it in force pending house assent | Original act's effects removed | no |
| Stranger attempts repeal | Keep the adopted rule in force | Repeal awaits a person's approval | yes |
| House A approves its local change | Adopt it | Adopted | yes |
| Only House A approves a courtyard change | Keep the old rule; await House B | Awaiting House B | yes |
| Both houses approve the courtyard change | Adopt the replacement | Replacement adopted | yes |
| House A removes House B's delegate alone | Preserve the other seat pending joint assent | Seat removed immediately | no |
| House A moves an adopted courtyard rule to its own scope | Preserve shared jurisdiction pending joint assent | Scope changed immediately | no |

**Six desired effects hold; four do not.** The negative results are
counterexamples to these charters under canon's current interpretation, not
changes to canon's published laws. The desired outcome was not fitted to the
observed behavior.

The delegate-removal case establishes unilateral exclusion, not that Mira can
then pass every courtyard proposal alone. With fixed named joint holders,
removing a required delegate can instead leave the procedure unable to finish.
Later capture or recovery was not part of this experiment.

## Xenia rehearsal

Xenia `df729c3`, `claude-sonnet-5-5` through the subscription client, one run.
Set: `2026-11-02T16-26-40-687Z`. All **21 plants passed**, admitting the set.
One challenger per experiment; the other people's earlier approvals were
seeded, not enacted by live participants in these scenes.

| Scene | Marks | Caught unauthorized attempt | Missed unauthorized attempt | Legitimate attempt allowed | False objection | Unjudged |
|---|---|---|---|---|---|---|
| adoption | 5 | 1 | 2 | 2 | 0 | 0 |
| courtyard | 5 | 1 | 2 | 2 | 0 | 0 |

Both challengers attempted all five cases. Independently auditing their
recorded before/after states reproduces all ten deterministic conclusions,
with no missing case. These are counts from one directed rehearsal, not
population rates. Xenia's `passed` verdict means its instrument was admitted
and the participants completed their scenes; it does **not** mean the optional
charters held.

The [retained rehearsal data](results/rehearsal.json) keeps the marks, oracle
judgments, feedback, state checks, artifact hashes and reported cost. Complete
traces, transcripts and original attachments remain in the named xenia run.
The subscription client reported $0.7061289 for the accounted sessions,
including the plants. [Xenia's findings](findings/index.md) retain two grouped
missed-boundary findings and two soft findings, one of each per scene.

Two useful observations, in the challengers' own words:

> Helper: "So as the original drafter, I still have authority over a rule
> after you adopt it. Both commands remove it. The stranger can't do that."

> Mira: "I also couldn't tell whether that is a bug or just how canon is
> set up."

That uncertainty matters. A minimally opinionated record tool and an optional
institutional interpreter make different promises. A refusal or permission
should identify which one is answering.

## What this changes about the proposed architecture

1. **Do not add a jurisdiction graph on the strength of this example.** A
   separate shared scope, explicit delegate grants and joint ratification
   already express the ordinary two-house decision. The composition works.
   This does not establish arbitrary overlapping jurisdictions or independent
   quorums for many constituencies.
2. **The harder question is authority over changes to authority.** Changing a
   rule, its decision-makers or its scope needs a coherent institutional
   interpretation. Authorship and holding a seat currently supply powers that
   these particular charters do not intend. Four per-verb fixes would hide the
   common issue rather than establish the right boundary.
3. **An independent institutional program is a credible next experiment.** The
   raw record already supplies the proposals, human positions, grants,
   placements, authorship and reversal history needed to evaluate these cases.
   The finite oracle is already independent of canon's authority decisions.
   This supports testing optional institutional semantics outside the core.
4. **The oracle is not yet an authoritative interpreter.** It judges only
   declared first attempts in fresh cases. It does not implement general
   temporal membership, competing amendments, delegation or conflict among
   jurisdictions. Nor does it prevent canon's current fold from applying the
   acts. Enforcing an optional interpretation requires an explicit admission
   or projection boundary, with its meaning named and versioned.
5. **An interpreter must read history, not just filter `canon list`.** Once a
   primitive reversal removes a commitment from the derived view, an
   institutional interpretation that refuses that reversal needs the original
   acts and evidence. A second layer that only examines today's active list
   cannot recover the protected rule reliably.

The next design test is therefore small: apply one optional institutional
interpretation to all four authority-changing effects while preserving the
six controls. It should compose from existing primitives where possible and
name any missing primitive with a failed composition attempt. These results
do not justify imposing either charter on every canon.

## Independent implementation experiment

The next experiment now exists as [canon-institution](../../../crates/canon-institution/README.md),
a pure library and diagnostic runner in canon's workspace. It consumes raw canon logs,
an explicit established basis and an explicit clock. The interpretation is
named `shared-assent-v1`; it leaves the existing reader and record unchanged.

[Its comparison](results/interpreter.json) reconstructs all ten original raw
histories and verifies their before/after hashes. All ten desired outcomes
hold: the four disputed effects stay pending and all six successful controls
are preserved. This is an implemented projection, not the finite oracle
returning expected labels. The interpreter has no dependency on the cases or
oracle; the external runner alone compares effects with expected outcomes.

One admission procedure asks the existing ratifier for assent over every
affected jurisdiction. Thirteen Rust checks include affirmative authorization
of all four protected effects, different people/resource names, both sides of
a transfer, new-voter capture, future approvals, raw-file preservation and
arrival-order independence. Additional red-first checks prevented treating
undo of a resignation as another resignation and prevented approvals from
switching a fixed-basis round to a different constitution.

The experiment also found a genuine limit to filtering above the current fold:
human assent can authorize an agent-proposed grant in this interpretation, but
the native fold can still refuse it on the proposer's standing. The executable
reports that mismatch as unsupported rather than claiming an effect. This is
evidence for a lower, policy-free projection seam if the independent program
is to become a complete institutional interpreter.

The implemented boundary is intentionally one fixed-basis admission round. It
is not chronological self-amending governance, a migration of existing records,
or a new default for canon. [The design](../../../crates/canon-institution/DESIGN.md) states which
inputs it refuses and which temporal questions it does not answer.

## Instrument checks

- Six Python checks: planted unauthorized effects, blocked legal controls,
  independence of charter truth from product response, missing/ambiguous
  evidence, usage failures, and ten real append-only observations.
- Two scripted browser checks over the actual terminal bridge. A delayed
  command first exposed a prompt-prefix issue; fixed before the model run.
  The checks verify that a previous command's response cannot be scored as
  the next command's, and that real refusals and state effects are recorded.
- Canon's full `scripts/pre-push.sh` passed. This checkout has `core.bare=true`
  despite a working tree; a temporary, checkout-specific git invocation shim
  supplied `--work-tree` for the gate. Repository configuration was not changed.

To collect another completed set without another model run:

```sh
python3 demo/xenia/authority/collect.py <xenia-run-directory> \
  --out <new-report.json>
```
