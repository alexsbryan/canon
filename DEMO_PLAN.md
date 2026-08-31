# Founding demo — plan of record

Canon @ `draft-reads-anything`, HEAD `994c1a0`. Written 2026-08-26.

**The demo has never produced a number.** Not a low one — none. The corpus and
the extractor work; the comparison stage has never once run to completion. This
is the ordered path from that to two published artifacts, with every bar written
down before the data that tests it.

Phase states live in this file and are updated as work lands, so there is one
source of truth rather than two.

- 17 planted tensions · 11 supersessions · 6 decoys
- 330 bearing commitments · 690 comparison passes
- one sweep = 6h15m, measured

## What was decided

Three calls, 2026-08-26. Everything below descends from them.

**Ship shape — an HF pair plus a checkable README.** The six-hour run happens
once, here, and becomes provenance. A stranger's cost is a download, not six
hours of 27B inference. This replaces the old done-when ("reproducible by a
stranger on their laptop"), which contradicted the reason the demo is worth
doing at all.

**Snapshot scope — governance travels.** Silences, questions, scopes and ranks
survive an adopt. Grants and rulings deliberately do not: standing is not
transferable.

**Publish bar — hold below 0.50.** An honest-but-unimpressive number gets
iterated on, not shipped. This makes run cost the binding constraint on the
whole plan.

## The bars, pre-registered

Written before the data exists, which is the only thing that makes a verdict
honest. The first three already live in `crates/canon-cli/tests/draft_bar.rs`;
the last three are set by this document.

| Bar | Value | Where | Consequence |
|---|---|---|---|
| Kill — recall on the 11 supersessions | < 0.30 | `KILL_RECALL_FLOOR` | abandon the demo |
| Decoy ceiling — compatible pairs flagged as tensions | >= 5 of 6 | `KILL_DECOY_CEILING` | abandon; cannot tell a decoy from a conflict |
| Comparison coverage before a run may be scored | >= 95% | `MIN_COVERAGE` | refuse to score; the run is not a measurement |
| Publish — mean recall on the 11 supersessions | >= 0.50 | set here | ship the artifacts |
| Iterate band | 0.30 – 0.50 | set here | diagnose in Phase 1, then re-run |
| Two-up ceiling — supersessions visible with the pair shown alone | >= 6 of 11 | set here | below this, publishing is impossible by construction |

The two-up bar is derived, not chosen: publishing needs 0.50 x 11 = 5.5, so at
least 6. If the comparison stage cannot see 6 tensions when handed both sides
*alone*, it certainly cannot see them inside a 24-wide window, and no sweep
should be paid for.

## Where we actually are

Everything that exists is upstream of the measurement.

| Stage | State | Evidence |
|---|---|---|
| Corpus | built | 91 sections, 12,084 words, deterministic build |
| Ground truth | built | 17 planted, 11 Archives-derived, 6 decoys |
| Extraction | works | 342 candidates from 104 chunks |
| Reachability | 17/17 ceiling | 9/17 in the candidate's own words |
| Two-up ceiling | 9/11, all stable | Phase 1b, on the MoE the endpoint now serves |
| Comparison, in a window | **never run** | the harness does not exist |
| Comparison, full sweep | **never completed** | best attempt: 15% coverage, refused |
| Recall / precision | **no number** | — |

Two sweeps have been attempted. The first was destroyed by a `/tmp` purge on
reboot. The second died to the daemon shed defect at pass 93 of 690 — all 588
refusals were `local_queue_full` at queue position 1, which is a prediction bug,
not contention.

---

# Phase 1 — A cheap loop, and proof the comparison stage works · NEXT

Roughly a session. Nothing here costs six hours, and everything here can kill
the plan before six hours are spent.

Both failed sweeps died to infrastructure, not to canon, and neither cause is
fixed on this machine. Separately, `schedule()` builds a dense triangle with
`looks = 2`, so every pair is already shown twice — which means recall can be
measured on just the 17 planted pairs rather than all 690 passes. A broken
comparison prompt currently costs six hours to discover; that is the thing this
phase makes fifteen minutes.

**Harness**

- [x] Rebuild and restart the daemon so it carries the `predict_wait_ms`
      elapsed-subtraction fix. Restarted 23:36 onto the 23:29 binary; the old
      process had been up 5h47m. **Verified in the log semantics**: it now parks
      (`slot busy, waiting for permit`, `predicted_wait_ms=20341` under the
      30000 bound) and only sheds afterwards — `SHED after parking, the permit
      did not free within the wait bound`. That is the corrected behaviour, not
      the old instant prediction shed.
- [x] Point `scripts/draft-bar.sh`'s `CANON_DIR` somewhere durable — now
      `$OUT/scratch/run-N`, overridable with `CANON_BAR_SCRATCH`. Two further
      defects found and fixed in the same file: a failed run reported `exit 0`
      because `$?` had already been overwritten by the arithmetic expansion on
      the line above, and the closing summary counted directory entries rather
      than run artifacts, so a sweep where every run failed still reported
      "2 artifact(s)". All three falsified against the pre-fix script.
- [x] Confirm >=50GB free. 104GB after the host's own cleanup, up from 156MB.
- [ ] Decide the sweep's launch window against `com.svrn.co-sweep`, which holds
      a turn daily 03:30–04:20 local.
- [ ] **Timing gate — blocked on host quietness.** Median turn time across ~10
      sequential 40-token completions, taken with no build, test run or peer
      sweep active.

*Gate.* Median turn under 30s. This replaces "two concurrent completions,
neither shed", which was the wrong gate: with the fix in, a second caller parks
and then correctly sheds whenever a turn genuinely outlasts the 30s bound, so
that test measures host quietness while appearing to measure the shed logic.

The number matters more than the ceremony. Turns were measured at **75s** on
2026-08-26 with a `cargo nextest --workspace` and a `cargo check --all-targets`
running and swap exhausted (10,688M of 11,264M). At 26s a pass the sweep is
6h15m; at 75s it is over fourteen hours. The gate is a 2x schedule risk, not a
formality.

**Two-up: can it see a tension at all?**

- [x] Built as `two_up_upper_bound` in `crates/canon-cli/tests/draft_bar.rs`,
      reusing the bar's own `section_key` / `anchors` / `truth` rather than
      minting a second resolver. No production code: a canon holding exactly
      two commitments is at or below `BATCH`, so `canon tensions` makes one
      pass over one pair. `CANON_BAR_TWO_UP_DRY=1` resolves and spends nothing.
- [x] Ran, twice — the first run measured the instrument rather than the stage.
      `side_to_candidate` first copied `extraction_coverage`'s text-OR-citation
      haystack, which is right for "did extraction lose this from the section"
      and wrong here: the stage reads `c.text` alone, so selecting on a
      citation selects on evidence it never sees. 14 candidates from
      `constitution:III.2` share one wide quote, so S1 was handed "the judicial
      Power shall extend to all Cases in Law and Equity" against Amendment XI —
      a pair that does not conflict, scored as a model miss. Same for S4 and
      S7. Corrected: prefer the candidate whose OWN sentence carries the
      anchor, fall back to the citation only when none states it, and REFUSE
      when the fallback cannot name one (152 of 334 candidates share a citation
      with a sibling). Cross-check: the corrected resolver independently
      reproduces the 9/17 own-words reachability figure.
- [x] Labelled. The runner prints `UPPER BOUND` and the sidecar carries
      `"bound"`; each side's row records whether it resolved by own words or
      by citation.

*Gate.* >= 6 of 11 supersessions visible. Below that, fix the comparison prompt
here and re-run this loop — do not proceed to a sweep.

**Result 2026-08-27: PASS — 7 of 11 supersessions, against a floor of 6.**
Principles 4 of 6. All 17 pairs resolved, 14 of them naming both sides from the
candidates' own sentences; zero refused. 17 calls, 3m49s, against the sweep's
6h15m. Evidence at `fixtures/founding/runs/qwen-27b/two-up/`.

It took three runs to get one measurement, and the first two were the
instrument:

1. **5/11.** `side_to_candidate` copied `extraction_coverage`'s
   text-OR-citation haystack — right for "did extraction lose this from the
   section", wrong here, because the stage reads `c.text` alone. 14 candidates
   from `constitution:III.2` share one wide quote, so S1 was handed "the
   judicial Power shall extend to all Cases in Law and Equity" against
   Amendment XI: a pair that does not conflict, scored as a model miss. Same
   for S4 and S7.
2. **5 of 7 shown, CANNOT JUDGE.** Resolver corrected to prefer the candidate
   whose OWN sentence carries the anchor, fall back to the citation only when
   none states it, and refuse when the fallback cannot name one (152 of 334
   candidates share a citation with a sibling). That exposed the real blocker:
   four supersessions and one principle had anchors no candidate stated.
3. **7/11.** Six `ANCHOR` entries corrected in `fixtures/founding/build.py`
   against the rule that table already documents — a semantic invariant, not a
   span a paraphrase restyles. Corpus unchanged: `founding.md` and `truth.json`
   rebuilt byte-identical.

*Gate PASSED.* Proceed to the window test. Note the shape of the pass: 7/11 is
an UPPER bound, and the window gate below is 4/11 — a batch penalty of three
tensions would still clear it, a penalty of four would not.

**The four remaining misses are the comparison stage's own**, on pairs
correctly resolved and both stated in the candidates' own words except where
noted:

| id | the pair the stage could not tell apart |
|---|---|
| S2 | "vote by Ballot for two Persons" vs "in distinct ballots ... as Vice-President" |
| S7 | "first Monday in December" vs "noon on the 3d day of January" |
| S8 | House has not chosen "before the fourth day of March" vs "before the time fixed for the beginning of his term" |
| S10 | XIV.2 male-21 apportionment penalty vs XXVI.1 age 18 (side a resolved by citation) |

Three of the four are a DATE or a COUNT changing while the surrounding sentence
stays similar — the shape a comparison prompt is most likely to read as
agreement. That is the evidence any prompt change should be measured against,
and this loop is where to measure it: one call per pair, four minutes.

---

# Phase 1a — the comparison prompt, on the two-up loop · PRE-REGISTERED 2026-08-27

Written BEFORE the arm exists and before the baseline-with-decoys was run.
Everything below is falsifiable and none of it may be edited after data lands.

**Hypothesis.** Three of the four remaining supersession misses are one shape:
a DATE or a COUNT changes while the surrounding sentence stays similar. S7
(first Monday in December / noon on the 3d day of January), S8 (before the
fourth day of March / before the time fixed for the beginning of his term), S2
(vote by Ballot for two Persons / in distinct ballots). S10 is a fourth of the
same family — twenty-one years of age against eighteen.

**Why the prompt is the suspect.** Two of its five rules push exactly this way:
"A general rule and a specific case of it are not a tension" and "Commitments
about different subjects are not a tension". A re-enactment with one number
changed reads as a restatement under both. The rule set was written to hold
precision down and has no clause telling it that a re-stated rule with a
changed quantity is the ONE case where near-identity is the signal.

**Arm.** One added rule naming that case. Nothing else changes — not the
schema, not the temperature, not the batch.

**The bars.** Promote the change only if ALL THREE hold:

| Bar | Value | Why |
|---|---|---|
| Recall | supersessions strictly greater than baseline | a change that fixes nothing is not worth the precision risk |
| Precision | decoys flagged <= baseline | §18.6 — a judge change reported only in the direction it was meant to fix |
| No trade | every pair SEEN at baseline is still seen | the failure a recall-only reading hides |

**Reject on any of:** decoys flagged increases at all, even by one, whatever
recall does. Any baseline-seen pair lost. Recall unchanged or lower.

**n=1 is only valid if the instrument is silent.** Temperature is 0.0 and runs
2 and 3 agreed on 12 of 12 pairs held at identical candidate indices. The
baseline-with-decoys run below is also the third replicate: if its 17 planted
verdicts do not reproduce run 3 EXACTLY, this whole design is void and each arm
needs repeats before any delta may be read (§18.5).

> **AMENDED 2026-08-27, after the baseline and BEFORE any arm existed. THE
> CHECK ABOVE FAILED and n=1 is void, exactly as written.** The baseline
> reproduced 16 of 17 and flipped P2 from not-seen to SEEN on identical
> candidate indices, same prompt, same binary. Temperature 0.0 is not
> determinism on a batching endpoint, and 12-of-12 agreement over two runs was
> not evidence that it was — it was the sample being too small to show the
> flip.
>
> The bars themselves are UNCHANGED. What changes is how a pair's verdict is
> read: `CANON_BAR_TWO_UP_RUNS=3`, majority of three decides each pair, and
> the count of pairs that did not answer the same way every time prints beside
> every figure as the instrument's own noise floor. No delta smaller than that
> floor is readable. Both arms run at n=3 on this rule; the n=1 baseline above
> is superseded and is kept only because it is the run that caught the flip.

**The decoy denominator is 4, not 6, and that is a finding.** N3 resolves to
nothing because Amendment XIV.5 and XXVI.2 produced ZERO candidates — the
enforcement sentence is word-for-word identical across four amendments and
dedupe folded two of them away. N6 fails because Article I, Section 8's
1,585-character enumerated-powers list yielded 7 candidates and none of them
mentions coining money. Neither is an anchoring problem and neither is repaired
here; both are reported against the sweep, which will meet the same gaps.

**ARM A — the shipped prompt, n=3, recorded before arm B was written.**

| | value | stability |
|---|---|---|
| supersessions | 7/11 | every pair 3/3 or 0/3 |
| principles | 4/6 | P2 unstable at 1/3; P5 stable 0/3 |
| decoys flagged | 0/4 | every pair 0/3 |
| pairs that flipped | 1 (P2) | the noise floor |

Seen: S1 S3 S4 S5 S6 S9 S11, P1 P3 P4 P6. Not seen and STABLE at 0/3: S2 S7
S8 S10, P5. The four supersession misses never once came back as tensions in
three asks, so a flip to 2/3 or better in arm B is a real signal and not the
wobble. Evidence `two-up/two-up-1787887934.json`.

The noise is one pair, and it is the same pair both times. That is worth more
than a smaller number would have been: it means the instrument is quiet
everywhere the hypothesis is being tested.

---

**ARM B — one added rule, n=3. PROMOTED: all three bars pass.**

The rule, in `crates/canon-cli/src/tensions.rs` `SYSTEM`, placed after the
three "not a tension" negatives it has to overcome: *"But two commitments
governing the same situation that name different dates, ages, counts or
thresholds ARE a tension: honouring one breaks the other, and near-identical
wording makes this more likely, not less."*

| Bar | A | B | |
|---|---|---|---|
| supersessions, strictly greater | 7/11 | **9/11** | PASS |
| decoys flagged, <= baseline | 0/4 | 0/4 | PASS |
| no baseline-seen pair lost | 11 seen | all 11 still seen | PASS |

Everything that moved, and nothing that moved is omitted:

| pair | A | B | |
|---|---|---|---|
| S8 | 0/3 stable | **3/3 stable** | the clean win — a date, exactly the hypothesis |
| S10 | 0/3 stable | 2/3 FLIPPED | counts by majority, but UNSTABLE |
| S2 | 0/3 stable | 1/3 FLIPPED | moved, did not cross |
| P5 | 0/3 stable | 1/3 FLIPPED | moved, did not cross |
| P2 | 1/3 FLIPPED | 0/3 stable | moved DOWN, inside the floor |

**The cost, which no bar covered: instrument noise went from 1 flipped pair to
3.** Making the stage more willing to call a tension made it less repeatable,
and only one of the two recall gains is stable. Honest reading of +2: one solid
(S8) and one that would not survive a stricter stability rule (S10). A
stability bar belongs in the next pre-registration; adding one now, after
seeing this, would not be a bar.

**The hypothesis is confirmed in part and refuted in part.** S8 (a date) fixed
cleanly and S10 (an age) crossed unstably, but S7 — *"first Monday in December"*
against *"noon on the 3d day of January"*, the purest date case of the four —
did not move at all, 0/3 in both arms. A plausible reading is that S7 is a weak
planted tension rather than a stage failure: both passages carry the same escape
hatch ("unless they shall by law appoint a different day"), so Congress could
lawfully meet on one day under either. THAT READING IS NOT ACTED ON HERE.
Re-labelling ground truth after seeing which pair resisted is the one move that
would invalidate every number above it.

**Measured on ONE corpus.** This prompt is shipped code and reaches `canon
draft` and `canon tensions` for every user. The maple-house bar is the
regression check and has NOT been run against this change — it needs three
`draft` runs against an endpoint. That is the outstanding risk on this promotion
and it is not discharged by anything above.

**In the window: does it survive twenty-four?**

- [ ] Same 17 pairs, embedded in a 24-wide window of real distractors from the
      run artifact.
- [ ] Compare against two-up to get the batch penalty as a number.

*Gate.* >= 4 of 11 visible. If two-up passed and this fails, `BATCH` is the
lever — measure 24 against 12 before choosing, since halving it roughly doubles
passes and cost.

---

# Phase 1b — the instrument moved · 2026-08-30

**Every number above this line was measured on `Qwen3.8-27B-UD-Q6_K_XL`. The
endpoint no longer serves it.** `primary` is an alias, and on 2026-08-30 it
resolved to `Qwen3.6-35B-A3B-UD-MTP-IQ4_NL` — a 35B mixture-of-experts with
about 3B active. Nothing announced the change and nothing would have.

**Decision: take the MoE.** Speed matters more than continuity at this phase.
The cost is stated plainly rather than absorbed: the Arm A / Arm B comparison
is a 27B result and stays one. The dates-and-counts rule promoted in Phase 1a
is still shipped code, but its `+2 supersessions` was measured on a model this
project no longer runs, and nothing below re-establishes that delta.

**The defect that let this happen, and its fix.** Three run artifacts in this
repository — both `des-moines-noise` runs and the founding run itself — record
`"model": "primary"`. An alias is not provenance, and after it moves those
numbers cannot be attributed to any model. Only `maple-house` recorded a
resolved id, and only because it was written down by hand. `canon` now
captures the `model` field the server returns on the first reply and records
it as `served_model` on every draft artifact and every two-up sidecar; when it
differs from what was asked for, it says so on stderr as it happens:

```
note: `primary` answered by Qwen3.6-35B-A3B-UD-MTP-IQ4_NL
```

## The re-baseline

**Design: hold extraction constant, change only the judge.** The loop reuses
the candidate set from the 27B founding run rather than re-extracting, so the
sides shown are byte-identical to what both 27B arms saw. The one variable is
which model reads them. A fresh extraction would have moved two things at once
and made the comparison unreadable.

n=3, majority decides each pair, per the Phase 1a amendment. Evidence at
`fixtures/founding/runs/moe-35b-a3b/two-up/`, and it is the first artifact here
that names the model that produced it.

| | 27B, arm A | 27B, arm B | **MoE, shipped prompt** |
|---|---|---|---|
| supersessions | 7/11 | 9/11 | **9/11** |
| — all stable at 3/3? | yes | no, S10 at 2/3 | **yes, all nine** |
| principles | 4/6 | 4/6 | **5/6** |
| decoys flagged | 0/4 | 0/4 | **0/4** |
| pairs that flipped | 1 | 3 | **2** |

*Gate PASSED* — 9 against a floor of 6.

Same count as arm B, **and not the same nine.** Everything that moved:

| pair | 27B arm B | MoE | |
|---|---|---|---|
| S7 | 0/3 stable | **3/3 stable** | gained, and see below |
| S10 | 2/3 unstable | **3/3 stable** | the arm-B win, now solid |
| P5 | 1/3 flipped | **3/3 stable** | gained |
| S9 | 3/3 stable | 1/3 flipped | **lost** |
| S2 | 1/3 flipped | 0/3 stable | still not seen |
| P2 | 0/3 stable | 1/3 flipped | still not seen |

## S7, and why the plan was right not to touch it

Phase 1a ended with S7 — *"the first Monday in December"* against *"noon on the
3d day of January"* — at 0/3 in both arms, the purest date case of the four it
predicted. It offered a reading: that S7 is a weak planted tension rather than
a stage failure, since both passages carry the same escape hatch. And it
refused to act on it, in capitals: *"Re-labelling ground truth after seeing
which pair resisted is the one move that would invalidate every number above
it."*

**The MoE sees S7 at 3/3, both sides in the candidates' own words.** The
pair was always a real tension; the 27B could not see it. Had the ground truth
been relabelled on a plausible reading of a resisting pair, this corpus would
now carry a permanent error introduced to explain a model limitation that
turned out to be one model's.

That is the whole argument for pre-registration, and it cost nothing to
observe because the discipline was already in place.

## What this does and does not license

- **Does:** the sweep is worth paying for on this instrument. The two-up
  ceiling is 9/11, the publish bar needs a mean of 5.5, and the window gate of
  4/11 has three tensions of headroom against a batch penalty.
- **Does not:** carry over any 27B figure. The reachability numbers (17/17
  ceiling, 9/17 own words) are properties of the candidate set and still hold,
  because the candidate set did not change. Everything downstream of a model
  call is now a 35B-A3B number or it is nothing.
- **Still open, unchanged by any of this:** the window test harness does not
  exist; the timing gate has not run; and the maple-house regression check for
  the shipped prompt change remains undischarged — now harder, because its
  baseline is three 27B runs and the 27B is gone.

---

# Phase 1c — is it reading, or is it remembering? · PRE-REGISTERED 2026-08-30

Written BEFORE the harness exists and before either arm has been run. Nothing
below may be edited after data lands, and that rule has already paid once in
this file — see S7 in Phase 1b.

**The objection this exists to answer.** Every model has read the Constitution
many times. "It found that the Seventeenth Amendment superseded senators chosen
by state legislatures" is not, on its own, evidence that this pipeline works —
it is compatible with a model that took civics and never read the two passages
it was handed. Until that is ruled out, the founding numbers cannot be
published as a capability claim, and **no part of this plan has previously
addressed it.** Not the bars, not the provenance, not the README.

At six hours a sweep, this is also the cheapest possible thing to learn first.
If the founding verdicts are recall, the sweep is measuring the wrong thing.

**Hypothesis.** The comparison stage's verdict tracks the two passages in front
of it. Remove the fact a supersession turns on and the verdict should follow.

**Arm P — neutralised.** `fixtures/founding/perturbations/neutralised.json`.
One side of each pair is edited so the pair no longer conflicts. Ground truth
for this arm: **none of the nine is a tension.**

**Arm S — sham.** `fixtures/founding/perturbations/sham.json`. A word of
comparable size is changed on one side that the conflict does not turn on.
Ground truth: **all nine are still tensions.** This is the control for the
control: if arm P stops flagging and arm S stops flagging too, then editing the
text is what moved the verdict, and arm P says nothing about reading.

**Nine, not eleven.** S2 and S9 were not flagged at baseline, and a pair that
is not seen cannot become less seen. Both exclusions are recorded in the table
with their reason.

**The bars.** n=3, majority decides each pair, as amended in Phase 1a. `P` and
`S` are the counts of the nine still flagged in each arm.

| verdict | condition | consequence |
|---|---|---|
| **READING** | P <= 2 **and** S >= 7 | the verdicts track the text; the 9/11 stands and the sweep is worth paying for |
| **CONTAMINATED** | P >= 6 | verdicts survive the removal of the thing they turn on; the founding numbers are recall and may not be published as a capability claim |
| **CONFOUNDED** | S <= 4, whatever P does | editing the text alone moves the verdict; arm P is uninterpretable and the design must change before anything is read from it |
| **AMBIGUOUS** | anything else | report as partial recall; the 9/11 may not be quoted without this result beside it |

**Integrity conditions, also pre-registered.**

- The harness REFUSES if any `find` string is absent from the resolved
  candidate text. A silent no-op edit would leave the pair intact and score as
  "reading", which is the one way this experiment could lie in the direction
  it wants.
- The perturbation tables and their stated rationales were written before the
  harness and are fixed. Rewriting an edit after seeing which pair flags is the
  S7 move and is forbidden.
- S10's neutralisation is declared PARTIAL in the table, in advance, and is
  counted normally anyway.
- Both arms reuse the 27B candidate set and the same resolver as Phase 1b, so
  the sides are the same sentences; only the edits and the judge differ.

**What a CONTAMINATED verdict would NOT mean.** That canon is broken. Extraction,
citation-cutting, the ledger and the whole decision layer are untouched by this
— they never call a model. It would mean the founding corpus cannot carry a
claim about the comparison stage, and that a corpus the model has not memorised
has to. `maple-house` and `des-moines-noise` are that corpus, and they are
already vendored.

---

## Phase 1c RESULT — AMBIGUOUS. P = 4 of 9, S = 9 of 9.

Run 2026-08-30 on `Qwen3.6-35B-A3B-UD-MTP-IQ4_NL`, n=3, majority per pair.
Evidence at `fixtures/founding/runs/moe-35b-a3b/perturb-neutralised/` and
`…/perturb-sham/`. The pre-registered table is above this line and is unchanged.

| pair | baseline | P — conflict removed | S — irrelevant edit |
|---|---|---|---|
| S1 | 3/3 seen | **0/3 dropped** | 3/3 seen |
| S3 | 3/3 seen | 3/3 STILL FLAGGED | 3/3 seen |
| S4 | 3/3 seen | 3/3 STILL FLAGGED | 3/3 seen |
| S5 | 3/3 seen | 3/3 STILL FLAGGED | 3/3 seen |
| S6 | 3/3 seen | **0/3 dropped** | 3/3 seen |
| S7 | 3/3 seen | **0/3 dropped** | 3/3 seen |
| S8 | 3/3 seen | 3/3 STILL FLAGGED | 3/3 seen |
| S10 | 3/3 seen | **1/3 dropped** | 3/3 seen |
| S11 | 3/3 seen | **1/3 dropped** | 3/3 seen |

**The verdict is AMBIGUOUS and its consequence binds: the 9/11 may not be
quoted without this result beside it.** Not in the README, not in a talk, not
in an artifact. Half the verdicts followed the text and half survived the
removal of the thing they turn on.

**The sham arm is clean, and that is what makes P readable.** 9 of 9 still
flagged after an edit of comparable size that the conflict does not turn on.
Editing the text does not by itself move the verdict, so the CONFOUNDED branch
is ruled out and the neutralised arm means what it says.

**P is an UPPER bound on contamination.** This is a property of the design and
not a reading of the data: an incomplete neutralisation leaves a real residual
difference, which can only ADD a flag, never remove one. So contamination is at
most 4 of 9 and possibly less. It is not at least 4.

**The four that resisted, and the honest uncertainty about them.** S3, S4 and
S8 each retain a textual difference the edit did not remove — S3 still has both
sides speaking about the same institution, S4 still keeps the free/other person
categories apart, S8 still says "Vice-President" against "Vice President elect
… until a President qualifies". S5 is the cleanest neutralisation of the four
and the strongest single signal of recall.

**That reading is NOT acted on here.** Adjusting the count after seeing which
pairs resisted is the S7 move, and the table above stands at 4. Testing it
needs its own pre-registration, written before the arm, and the obvious design
has a trap in it: neutralising the residual differences too makes the two sides
near-duplicates, and a stage's behaviour on duplicates is a different question
from its behaviour on compatible pairs.

**What this changes.**

- The sweep is NOT killed. The kill condition was P >= 6 and P is 4.
- The founding corpus moves from evidence to demonstration. It shows the
  pipeline carries a real, large, adversarial document end to end without
  inventing a citation. It cannot, on this result, carry a claim about the
  comparison stage's judgement.
- The evidential load moves to corpora no model has memorised —
  `maple-house` and `des-moines-noise`, both already vendored with anchors —
  and to the decision layer, which never calls a model at all.
- Every future founding figure carries this number with it.

**What it does not change.** Extraction, citation-cutting, the ledger, the fold
and the whole decision layer are untouched by this result. They do not call a
model. `canon replay` is unaffected, and so is every governance verb.

---

# Phase 1d — the window gate, on rented hardware · 2026-08-31

**GATE PASSED: 11 of 11 supersessions in a 24-wide window, against a floor of
4.** Evidence at `fixtures/founding/runs/pod-27b/window-24/`, served_model
`Qwen3.8-27B-UD-Q6_K_XL`, n=3, majority per pair.

Run on a rented Vast A6000 (`dev-pod.sh`, solo mode) carrying the ORIGINAL
27B loadout, which is why this is comparable to Phase 1a arm B at all: same
model id, same shipped prompt, same candidate set. The local endpoint's
`primary` alias had moved to a 35B MoE (Phase 1b), so the pod is what made a
matched comparison possible.

| | two-up, 27B (arm B) | **window 24, 27B** |
|---|---|---|
| supersessions | 9/11 | **11/11** |
| principles | 4/6 | 3/6 |
| decoys flagged | 0/4 | 0/4 |
| pairs that flipped | 1 | **6** |

## The "upper bound" was not an upper bound

Phase 1a called two-up an UPPER bound and set the window gate at 4/11 on the
reasoning that a batch penalty could only subtract. **On supersessions the
penalty is negative**: the window gained S2 and S7 and lost none. Principles
lost P1 and P6 and gained P5.

That assumption is now falsified and should not be relied on again. The
plausible reading — untested — is that twenty-two distractors give the stage
something to contrast against, and a pair shown alone has no context to be
distinguished FROM. It is also possible the window simply makes the stage more
willing to call a tension; the decoys argue against that, because a more
willing stage should have flagged some, and 0 of 4 still stand.

## The cost of the gain, which no bar covered

**Instrument noise went from 1 flipped pair to 6.** A third of the resolved
pairs did not answer the same way three times. The aggregate is higher and the
individual verdicts are less reliable — the same trade Phase 1a arm B made at
smaller scale, at a larger size. Any future comparison at this window size
needs n greater than 3, and a stability bar belongs in the next
pre-registration rather than being added now, after seeing this.

## The timing gate, finally measured

63 window calls in 818.7s = **13.0s per 24-commitment pass**, which is one
`BATCH` and therefore one sweep pass. The 6h15m estimate assumed 26s.

At 13.0s, 690 passes is **about 2.5 hours** — on rented hardware at $0.44/hr,
roughly **$1.10**. The host-quietness timing gate in Phase 1 was written for a
laptop that also runs builds and tests; renting removes the variable it was
guarding against.

---

# Phase 2 — THE SWEEP COMPLETED · 2026-08-31

**676 of 676 comparison passes. No refusals, no shed, no hole in the tape.**
`fixtures/founding/runs/pod-27b/sweep/run-1788143950.json`, served_model
`Qwen3.8-27B-UD-Q6_K_XL`, 5,849s (1h37m) on a rented Vast A6000.

This is the run that had never finished. Two earlier attempts died — one to a
`/tmp` purge on reboot, one to the daemon shed at pass 93 of 690. Renting
removed both causes: the scratch was durable, and a dedicated host has no
build, no test run and no peer sweep competing for the endpoint.

| stage | |
|---|---|
| chunks | 104, none unread |
| candidates | 336, 334 kept |
| dropped | 3 — 1 bad citation, 2 stating a number their citation does not |
| duplicates folded | 2 groups |
| commitments compared | 324, in blocks of 24, every pair weighed twice |
| passes | **676 / 676** |
| pairs proposed | 283 |
| failed / checkpoint | none / none |

## The number

**Recall 0.59 — 10 of the 17 planted tensions. Precision 0.26. 1 decoy of 6.**

Scored by the repository's own scorer against `truth.json` and
`extraction-anchors.json`, no reimplementation.

| bar, pre-registered | value | this run | |
|---|---|---|---|
| Kill — recall on the planted set | < 0.30 | **0.59** | clear |
| Publish — mean recall | >= 0.50 | **0.59** | clears it |
| Decoy ceiling — compatible pairs flagged | >= 5 of 6 | **1 of 6** | well clear |
| Comparison coverage before scoring | >= 95% | 676/676 | clear |

**This is ONE run.** `MIN_RUNS` is 3 and the scorer enforces it; the figure
above was obtained by scoring the single artifact three times, so the
"0.59–0.59 noise floor" it printed is an artifact of that and not a spread.
The publish bar is on the mean of three INDEPENDENT runs and is not yet met —
what is met is the bar's value on the one run that exists.

Phase 1d measured 6 of 21 pairs flipping across three asks at this window
size, so the real spread is unlikely to be small.

**Never found: S1, S9, S11, P2, P3, P4, P6.** S1, S9 and S11 were seen 3/3 in
BOTH the two-up and the 24-wide window arms, and are missed here. Same model,
same prompt, same block size — the difference is the company: a block of 24
drawn from 324 commitments is not a block of 24 drawn from a 21-pair fixture.
That is the batch penalty appearing where the earlier arms could not see it,
and it is the first evidence that the window gate's 11/11 does not transfer.

Precision 0.26 is over the labelled region only — 35 sections, every pair of
them labelled. 567 proposed pairs reached outside it and are not scored, and
102 intra-section pairs are excluded.

## What it costs to turn this into a number

Two more runs on the same instrument. At 1h37m each that is **~3.2 hours and
about $1.50** of rented A6000, and it produces the triple the scorer needs.

The pod is destroyed and nothing is billing. Total for this session, including
the window gate and the boot, was about **$0.90**.

## What is settled, and what is not

- **Settled:** the pipeline carries the founding corpus end to end at full
  width without a hole. That was genuinely in doubt.
- **Settled:** cost. 13.0s a pass, 676 passes, under two hours, about a
  dollar. The 6h15m figure that shaped this whole plan was a laptop artifact.
- **Measured once:** recall 0.59, precision 0.26, decoys 1 of 6. Clears every
  pre-registered bar including publish, on n=1. Two more runs make it a mean.
- **Carries forward regardless:** the Phase 1c contamination bound. Whatever
  the eventual number is, it travels with "up to four of nine verdicts on this
  corpus survive removing the thing they turn on".

---

# Picking this up · paused 2026-08-30

The daemon was released mid-run at the operator's request. Nothing here is
blocked on anything except the endpoint.

**The window gate was killed at S10 of 11 and produced nothing.** The sidecar
is written only after every pair completes, so ~35 minutes and 19 of 21 pairs
were lost. It starts over:

```sh
CANON_BAR_TWO_UP_RUNS=3 CANON_BAR_WINDOW=24 \
CANON_BAR_TWO_UP_OUT=$PWD/fixtures/founding/runs/moe-35b-a3b/window-24 \
CANON_BAR_RUNS=$PWD/fixtures/founding/runs/qwen-27b \
CANON_BAR_TRUTH=$PWD/fixtures/founding/truth.json \
CANON_BAR_ANCHORS=$PWD/fixtures/founding/extraction-anchors.json \
cargo test --test draft_bar -- --ignored two_up --nocapture
```

The gate is >= 4 of 11, pre-registered in Phase 1, with the two-up ceiling at
9/11 — three tensions of headroom against a batch penalty.

**A preliminary timing read, from the partial run.** A 24-commitment window is
one `BATCH`, which is the shape of one sweep pass. Those pairs ran at roughly
1.7 minutes for three calls — about 35s a pass, against the 26s the 6h15m
estimate assumed. On that pace the sweep is nearer seven hours. **This is a
partial-run impression, not the timing gate**, which still has to be run
properly on a quiet host.

## In order, when the endpoint is free again

1. **Re-run the window gate.** ~35 min. It is the last thing between here and
   a defensible reason to spend six or seven hours on the sweep.
2. **Cut the demo tape.** `./scripts/record-demo-tape.sh` — one maple-house
   draft run, so act 1 of the demo is real output with no live risk.
3. **Look at `check` and `tensions` on stage.** They work; nobody has checked
   whether they read well at projector size.
4. **The maple-house regression check** for the Phase 1a prompt change. Still
   the plan's own undischarged risk, and now harder: its baseline is three 27B
   runs and the 27B is gone. Re-baselining on the MoE is the honest option.
5. **The sweep**, only if 1 clears.

## What needs no endpoint at all

```sh
cargo test                        # 363 pass
./scripts/cpr-sweep.sh            # the whole transfer study, ~3 seconds
./scripts/demo.sh --offline       # acts 4-9 of the run of show
```

---

# Phase 2 — The number · later

6h15m for the first, ~19h for the last. Needs Phase 1 green throughout.

One run decides whether to iterate or proceed, and that decision does not need a
noise floor. Publication does. `MIN_RUNS = 3` exists for a reason the "one run
per sweep" decision was right to override for iteration and wrong to carry into
publication: run-to-run spread on this pipeline is about seven points, so a
single 0.52 is not distinguishable from 0.45 against a hard 0.50 gate.

**One clean sweep**

- [ ] launchd one-shot with a Monitor armed on the log in the same turn, so an
      hour cannot burn invisibly.
- [ ] Durable `CANON_DIR`; remove the `done` sentinel before firing.
- [ ] Score recall, precision, decoys flagged, coverage.

*Gate.* Coverage >= 95%, or the bar refuses to score it. Then: below 0.30 stop
and reconsider the project; 0.30–0.50 return to Phase 1 with what the misses
say; >= 0.50 continue.

**Three runs, for a number worth publishing**

- [ ] Three runs, scored together. Worst-run coverage governs.
- [ ] If the mean lands in the iterate band after three runs, the band decision
      stands — return to Phase 1.

*Gate.* Mean recall >= 0.50 across three runs, worst-run coverage >= 95%,
decoys under the ceiling.

---

# Phase 3 — What travels, and where it goes · later

~2 sessions. The governance work is offline and needs no daemon, so **it runs
during Phase 2's wall-clock** — the sweep is six hours of dead time that should
be spent on it. The repositories need Phase 2's number.

Both halves of the distribution already exist and use the same transport.
Canon's `lineage::fetch` is `git clone` plus a tag checkout; HuggingFace dataset
repos are git. Verified 2026-08-26: an anonymous clone of `svrnmesh/sep-index`
returns 144K with no token, LFS files as pointers.

**Governance travels**

A `Snapshot` carries commitments only, so an adopt drops the questions and
silences — which on this corpus is half of what makes the output interesting.

- [ ] Add `governance { policies, scopes, silences, ranks }` to `Snapshot`.
      Grants and rulings excluded: standing does not transfer on adopt.
- [ ] Round-trip it through `render()` and `parse()`, which is strict by design.
- [ ] Decide whether governance participates in the generation hash. Recommend
      yes — otherwise two canons with different silences share a generation, and
      generation is what answers "are we on the same version" without a registry.
- [ ] Emit inherited acts for governance items in `adopt`; teach
      `Divergence::compute` about them.

*Gate.* Adopt a canon carrying silences into a clean directory: `list`, `open`
and the silences are all present, `diff --upstream` is clean immediately after,
and a grant held by the publisher does *not* appear.

**The two repositories**

- [ ] `svrnmesh/founding-canon` — `acts.jsonl`, `name` and the profile at repo
      root, one git tag per generation. No new canon code.
- [ ] `svrnmesh/founding-index` — the documents as a prebuilt corpus:
      `[prebuilt]` block, matching `registry.toml` entry, `catalog_status`.
- [ ] Verify anonymous readability at the *download*, not the metadata. A gated
      repo still answers 200 on `/api/datasets/` and fails at the fetch with 401.

*Gate.* On a machine with no canon state: `canon adopt <url>@<gen>` succeeds and
`canon list` shows the rules; one-click Add installs the corpus; every citation
in the README dereferences to its source line.

**The README, and publish**

The ground truth is public, which is the whole reason this is viral and also the
reason a single wrong claim is the failure mode that matters. The first reader to
check will check in a browser, in thirty seconds.

- [ ] Numbers at the top: recall, precision, decoys, and *n*. Not below the fold.
- [ ] Three to five findings, each with canon's own sentence, its citation, and
      a link to the National Archives.
- [ ] What it missed, named — the misses are what make the hits credible.
- [ ] Framing discipline: "this is what the tool read, cold, with citations."
      Never "the canon of the United States."

*Gate.* Every claim in the README traced to its citation and checked against
archives.gov by hand before publish. No exceptions for the ones that look
obvious.

---

## Open decisions

**The decoy ceiling looks too loose to publish behind.**
`KILL_DECOY_CEILING = 5` means flagging four of six compatible pairs as genuine
conflicts still passes. That is a reasonable floor for "don't abandon the
project" and a weak one for "publish this against public ground truth". Recommend
tightening it for the publish gate specifically — but a bar moved after seeing
data is not a bar, so it moves now or not at all.

**Does governance enter the generation hash?** Recommend yes. Two canons that
differ only in what they are deliberately silent about should not report the same
generation. The cost is that every existing generation string changes.

**This plan is net-additive, and says so.** It adds one bar instrument, one
`Snapshot` field with its round-trip, two repositories and a README. It retires
the "reproducible by a stranger" done-when, the `mktemp` checkpoint trap, and the
ambiguity of two unlabelled reachability readings. That is not a net
simplification and should not be presented as one.

## What could still sink it

- **The two-up instrument is an upper bound.** Passing it does not guarantee the
  sweep succeeds; failing it guarantees the sweep fails. A cheap kill, not a
  cheap proof.
- **The host is shared.** Another workstream cycles this daemon and competes for
  one inference slot. Every long run needs a health gate, not an assumption.
- **Six of the seventeen tensions reach comparison only as paraphrase.** The
  comparison stage reads `c.text` and nothing else, so those six work from a
  restatement rather than the document's words. That gap is real and currently
  unmeasured.
- **Publishing invites checking.** That is the point, and it is also the risk.
  The README's claims are the product; the aggregate number is only its warranty.
