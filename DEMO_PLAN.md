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
| Comparison | **never completed** | best attempt: 15% coverage, refused |
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
