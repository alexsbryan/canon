# Design: fixing what the Commonwealth load test found

*Internal. The design behind each entry in [LOAD_TEST_COMMONWEALTH.md](./LOAD_TEST_COMMONWEALTH.md), written before any of it is implemented. One section per entry, numbered to match. Each says what changes, what was considered and turned down, and what pins it. The dispositions there stand unless a section argues otherwise, and where it does the argument is marked so it can be overruled.*

## The shape of it

Sixteen entries, four mechanisms. Most of the fixes are one of these applied at a different verb:

1. **The fold says how a governance act took, not only whether.** Today `may_govern` returns a bool and the fold records refusals in `ungoverned`. An act that took because the canon was still open leaves no trace, so nothing downstream can say "this applied only because no grant predated it." Entries 1, 2 and 16 are all this.
2. **A write reports its fold before it reports itself.** Every handler prints a headline and then asks whether the act applied. Entry 2 is the ordering; `withdraw` never asks at all.
3. **Provenance is derived from the record, never typed.** A reason or a source a person declares at the keyboard is a string nothing can check. Entries 5, 6 and 7 all stay on the Commonwealth side; the one canon-side shape kept is derivation, not declaration.
4. **Progress and prompts are measured changes.** The extraction prompt is an instrument; a change to it lands with a before-and-after on the same notes. Entries 11, 12, 13.

Everything else is a local fix. The batches at the end say what lands together.

## 1. Say when an act took because the canon was open

**What changes.** `canon-core` gains a typed answer beside `may_govern`:

```rust
pub enum Seat { Held, Open, Lacking }
impl Canon { pub fn seat(&self, actor: &str, scope: Option<&Scope>, at: i64) -> Seat }
```

`may_govern` stays as `seat(..) != Lacking`, so nothing that calls it moves. Every gate in the fold goes through one helper that records the answer: a refused act lands in `ungoverned` as now, and an act that took under `Open` lands in a new `Canon::bootstrap: Vec<(ActId, String)>` with a reason in the same voice: `no grant over repo.defaults predates this act, so the scope was open`. That collapses ten hand-written `if !may_govern { ungoverned.push(..); return }` blocks into one decider, which is worth doing on its own.

The `Standing` verdict already carries this for commitments (`how: "nobody holds {here}; it is open"`), but `report_status` throws `how` away and prints `in force`. It will print `in force — {how}` whenever the canon has any grants, which is what `why` does already. The wording of `how` becomes precise about time: `nobody held repo before this was written; it was open`, because in the load test the scope *was* held, just not a second earlier.

**On screen.** A governance act that took while open says so on the line after its headline:

```
human:alex holds repo with no end — `canon overdue` will never mention it
  applied while open: no grant over repo predates this act
```

**Considered and turned down.** Making the CLI wait for the second to turn over before a gated write, so a script's grant would always predate its next act. It removes the surprise, but it also converts the case the rule exists for — a founder granting others before themselves in one sitting — from "all take" into a lock-out, and it backdates nothing while still lying a little about when. The fold rule stands; the fix is to say it.

**Docs.** SPEC rule 7 gains one sentence: an implementation MUST be able to say which acts applied because no grant predated them. GETTING_STARTED and the COOKBOOK founding recipe get the one line a script author needs: acts in the same second cannot govern one another.

**Pinned by.** A core test that a grant in the same second as an earlier grant lands in `bootstrap` and not in `ungoverned`; a CLI test that the line prints.

## 2. A refused act leads with NOT APPLIED

**What changes.** `cmds::report_governed` becomes `cmds::took(d, id) -> Took` with three arms, `Applied`, `Open(why)`, `Refused(why)`, and no printing. Each governance handler folds first and chooses its output from the answer:

```
NOT APPLIED: agent:claude granted agent:claude standing over repo.defaults without holding it
  on the record as can-4c1a9f0e2b71; somebody with standing has to do it.
```

Nothing else prints on refusal. The id stays because `undo` needs it. Applied and Open print the headline as today, plus the open line from entry 1.

`withdraw` prints success and returns 0 without consulting the fold at all, so standing somebody else down without standing says "no longer holds" while the record says NOT APPLIED. It goes through the same path.

**Pinned by.** A CLI test per gated verb (`grant`, `withdraw`, `policy set`, `ratification set`, `retract`, `accept`, `dismiss`, `decide`, `undo`, `allot`, `allocation set`) that a refused act's first stdout line begins `NOT APPLIED` and exit is 1. One table, one loop, so a new gated verb cannot skip it.

The COOKBOOK example at line 430 shows the old order and is updated.

## 3. Nobody at the keyboard, no default actor

**Disposition stands: prove on the Commonwealth side first.** The canon-side design, so the proof has a target:

`store::actor()` returns `Actor { name, defaulted: bool }`. `cmds::write` refuses when the actor was defaulted and stdin is not a terminal:

```
error: CANON_ACTOR is unset and nobody is at a terminal. Set CANON_ACTOR=agent:<name>,
or human:<name> if a person is running this.
```

Every write, not only adjudications. Under `standing` a holder's `add` lands in force directly, so an agent adding as the git user writes a rule, and that is the case the guard is for. Reads never ask who you are. A false refusal costs one environment variable; a false pass writes a person's decision by a machine, and those costs are not symmetric.

The integration tests drive the binary with piped stdin and no `CANON_ACTOR`, so they set one. That is the honest form: a test is automation.

GETTING_STARTED's "don't set `CANON_ACTOR`" stays true for a person at a terminal and gains the second half.

The prefix check on `human:` is unchanged. A label is a claim; `witness` and git are what check claims, and the README says so.

## 4. The agent surface at a thousand commitments

**Disposition stands: retrieval lives on the Commonwealth side.** The promotion contract, so it can be built against:

- `canon_list` takes optional `scope` and `query`. `scope` returns commitments in that room, the rooms above it, and unscoped ones. `query` is a case-insensitive substring over text. Both are filters over the same render, so an empty result says which filter emptied it.
- `canon check` keeps its whole-canon prompt. Narrowing by `--scope` is the obvious next step and is not taken here, because the planted-tension measurement was made against the whole canon and a narrower prompt has to be measured against the same set before it ships. The block-of-twelve result belongs to the same measurement: `check` over blocks, the way `tensions` already runs, is the candidate, and it costs `ceil(N / block)` calls per check.

A silence still matches its subject exactly. That is by design and stays.

## 5. A reason on the first rule

**Disposition stands: history keyed by canon id on the Commonwealth side.** The canon-side change that was considered, a `rationale` on `assert` written by `add -m`, is turned down: it is one more thing a person types at the moment of writing, and the reason a rule exists is better derived from the record it came out of than declared beside it. If the history view on the Commonwealth side ever needs a reason in the record itself, that is the evidence to reopen this with.

## 6. The passage that justified a rule

**Disposition stands: commit the exported sources beside the canon.** The promotion candidate, which is not a quote on the assert:

`source` grows a fingerprint: `notes/evidence.md:12-14#a3f9c1`, the short digest of the cited text, the same digest `seen` already uses. `why` resolves the span against the file when it is present, shows the passage when the digest matches, and otherwise says which of two things is true: the file is not on this machine, or it has changed since the rule was drafted. That is the §18.3 line: say what could not be checked.

The quote itself stays out of the record. A passage names people and incidents, which is exactly what the snapshot design keeps out of what travels by paste, and a record that carries every passage it was drafted from doubles in size for evidence a clone can recover by checking out the sources.

## 7. Provenance on `add` is derived, never typed

**Disposition stands, and the flag is turned down.** A typed `--source` is an interface that asks a person for a string nothing can check, and a string nothing can check is useless to anything deterministic downstream: `why` would render it, and no reader could tell a real span from a typo. The principle, which applies past this entry: never rely on user input the tool can derive.

The shape if the import path proves it needs provenance: `add` takes the file the text came from, locates the text in it the way `draft` already does, and writes the span it found with entry 6's digest. Cite-or-abstain, applied to `add`: text that is not in the file is refused, not annotated. No typed span exists in any form.

## 8. `--resume` resumes every run with something left

**What changes.** `--resume` alone walks every run file with candidates remaining, oldest first, announcing each as it does now. `[q]uit` ends the whole session, not one run, so `review` returns whether the person quit. `--resume <run>` names one; the run is a positional after the flag, which fits the argument grammar without inventing an optional-valued flag. Nothing on disk changes: runs are evidence and are never rewritten.

**Pinned by.** A test with two runs, the older with candidates left, that `--resume` offers the older one first and that quitting in it does not open the newer.

## 9. One room per rule

**Disposition stands, and no canon change.** A scope is who decides, not what a rule is about, and a rule in two rooms would need an answer to which room's ratification rule carries it. That answer is a policy nobody has asked for. Subject anchors keyed by canon id belong on the Commonwealth side, and entry 4's `query` filter is how a subject reaches the agent surface.

## 10. `who` names both rules

**What changes.** The last two lines of `who`:

```
rules made under: standing — set 2026-09-13 by human:alex over repo
proposals judged under: default
```

The first line comes from `adopted_at(scope, now)`; when nothing was adopted it says `standing (shipped)`. A change to the rule that is itself still proposed is shown after it, with what it needs, so "I just set it and it did not take" has its answer on the same screen.

## 11. Ask once for the narrowest span

**Fix candidate; lands with a measurement.** On `TooWide` for a rule, `extract` makes one follow-up call: the cited span alone, re-numbered, with the rule text, asking for the one to three sentences that state it. The answer is cut with `locate::cite` against the original passage, offset by the span's start, so the citation guarantee is unchanged. A candidate that survives this carries `narrowed: true` in the run artifact, because a run has to say how many of its citations were asked for twice. One that cites wide again is dropped with the original reason plus `and again when asked for the narrowest`.

Cost is one call per wide drop, nine of fifty-six chunks in pilot 1. The measurement is the draft bar over the founding corpus and the pilot notes: the bad-citation rate must not rise.

**Turned down.** Raising `SPAN_MAX`. The check's own reason stands: a wide span evidences the passage.

## 12. A rule reads without its passage

Two parts, one model-free.

**Strip emphasis from the text, never the quote.** In `extract`, after trimming: remove `**` and `__` pairs, and a single `*` or `_` wrapping the whole sentence. Backticks stay; they name symbols and a code canon needs them. Underscores inside identifiers stay because only paired wrappers are removed. Pinned by a unit test with `**build_self_manifest must derive…**` and `build_self_manifest` in one input.

**The subject.** No code can decide whether "Do not re-open it as a threshold question." stands alone; the prompt can be told what standing alone means. The `text` instruction gains: *the sentence names its subject; a reader who has not seen the passage must know what "it", "this" and "neither" refer to, and when the section title names the subject, the sentence uses that name.* The three sentences from the pilot are the negative examples, and the same twenty notes read the same way before and after are the measurement. Around thirteen of forty-nine is the number to beat.

**Turned down.** A third model stage that judges self-containment per rule. It costs `ceil(N / 10)` calls to answer a question the first call should already answer, and it would be a second reading of every rule beside `support`'s.

**Trial 2026-09-23: the prompt addition did not earn promotion.** The original
twenty-note pilot's labeled before/after artifacts are not here, so this is a
new, pinned sample: `scripts/subject-pilot.py` selected the shortest notes
with an extracted rule from the Commonwealth export (8 invariants, 6 attempts,
6 memories). Both readings used the same 25 chunks and the endpoint reported
`Qwen3.6-35B-A3B-UD-MTP-IQ4_NL`. The first dry-run checkpoint is
`commonwealth-ai/.canon/draft-runs/1790221432.partial.json` (50 candidates,
0 bad citations; checkpointed after dedupe). The second is
`1790222345.partial.json` (51 candidates, 1 bad citation; stopped after
extraction). Those run files are local evidence, not shipped fixtures. Exactly
28 of the first 50 candidate texts reappeared at the same chunk. The weak
"Do not use `arms/lab/build_estate.py` for this purpose" still said "this
purpose". An unrelated candidate gained "the inference stack", but another
subjectless "Either wider output limit or different tiebreaker" remained.
The new prompt also classified the field-census join and SCIP trait-declaration
guidance as records instead of rules, and called an answered validation
procedure a question. One newly proposed candidate cited five sentences and
was refused. This single paired run cannot assign every wording difference to
the prompt rather than model variance, but it does not clear the named subject
or coverage bar. The prompt was restored; no subject field or third reader was
added. The original ~13/49 pilot score was not reused as this sample's score.

**Source-topic review-card spike (same day).** `scripts/subject-spike.py` reads
an existing draft run and the exported note files. It resolves each candidate's
cited span to a unique source, checks that the cut quote still occurs there,
and gives the reviewer the source's first heading with its actual `file:line`
and text digest. It never treats the heading as an established subject: every
card says `human_confirm_subject`; an ambiguous file, missing quote or changed
source is blocked. A deictic-word signal is advisory only.

On the 20-note baseline: **48/48** rule quotes verified, **48/48** had a source
heading, **25/48** headings ended in the export's truncation mark, and **19/48**
rules triggered the advisory word signal. On the recorded full Commonwealth
load: **1,460/1,460** rule quotes verified; **1,152/1,460** headings were
truncated and **510/1,460** rules triggered the signal. The source heading for
"Do not re-open it as a threshold question" describes Arm A, whereas the
specific referent is the FaithBench recalibration hypothesis in the cited
line. "RE-MINTING IS OWED" does not trigger the word signal at all; its subject
is the stale baselines after a stack-fingerprint change in the source body.
"Neither end of this curve" points to the same broad Arm A heading as other
rules from that note. A verified heading therefore supplies context, but does
not structurally resolve the referent. Do not add it as a commitment subject
or use the word signal to suppress a candidate. The next proof target is
*per-candidate referent evidence* in the source text (possibly more than one
span for a curve comparing two operating points), cut and validated by code;
the person confirms or writes the subject during the existing one-at-a-time
review. Promote only if it fixes these three cases without hiding rules or
adding a second mandatory model stage.

**Local referent-evidence proof (2026-09-23).** `scripts/subject-spike.py`
also tried source-cut, at-most-320-character context beside each quote. Strict
same-line prefix/suffix or an immediately adjacent line gave context for
**1,176/1,460** full-load rules and **401/510** rules containing a possible
deictic reference; **246** snippets were clipped. The original three examples
all had same-line preceding context: FaithBench's falsified recalibration,
the stale quality-check baselines, and the shipped operating point on Arm A's
curve. The last names only ONE end of that curve, so it is not a resolved
subject. An extended arm that crossed one blank line reached **1,355/1,460**
and **483/510**, but its sampled snippets included unrelated recommendations,
run results and caveats from a previous paragraph. Even a snippet whose exact
bytes and coordinates code can prove is not necessarily the referent. These
two bounded local-window strategies are exhausted as AUTOMATIC subject
selectors, not as evidence a reviewer can choose to read.

**Small useful promotion:** `[c]ontext` in `draft` and `draft --resume` now
opens the ORIGINAL recorded chunk (with a source label and a separately named,
non-citable section title), then re-prompts the same candidate. It refuses
context when the recorded chunk id, source or quote does not match. No new
model call, act field, hard gate or unsolicited prompt payload; a reviewer can
inspect the full passage before editing an ambiguous rule, including both ends
of Arm A's curve when one short window is insufficient. The integration test
opens a fixture passage, quits and checks that no act was written. A canonical
subject still awaits a human-reviewed per-candidate source-backed proposal.

## 13. A description is not a standard

**Disposition stands: filter sources on the Commonwealth side first.** The prompt change if it is still needed at full scale, so it is on the shelf: *a description of how something currently is, what a file contains or what a setting is, is neither a rule nor a record unless the passage says it must stay so; return nothing for it.* Not a record: a record is what happened, and a description is what is. The code voice's "a standard the code is held to" keeps its sentence but gains "a rule says what must be so, not what is so." Measured the same way as 12.

## 14. Candidates per note

**Disposition stands.** Nothing changes here until candidates per note is measured at full scale. `--max-chunks` already caps a run; the measurement is the count in each run artifact.

## 15. Plain progress off a terminal

**What changes.** One function, in one place:

```rust
pub fn progress(line: &str)        // \r\x1b[K{line} on a terminal, {line}\n otherwise
pub fn progress_done(line: &str)   // the same prefix, always newline-terminated
```

The five sites in `draft` and `tensions` and the served-model note in `model` call it. The terminal check is stderr's, because that is the stream being written. Pinned by a unit test of the formatting given `is_terminal: bool`, so the rule is tested without a pty.

## 16. A grant names its granter, and says when it leaves you out

**What changes.** With entry 1's `Seat`, the grant handler can say the one thing a founder needs. Every grant line names who granted:

```
human:alex holds repo with no end — `canon overdue` will never mention it
  granted by human:Alex Bryan
```

When the act took while `Open`, the holder is not the actor, and the actor holds nothing over the scope afterwards:

```
  applied while open: no grant over repo predates this act
  human:Alex Bryan does not hold repo. From the next second, their writes here are
  proposals waiting on human:alex — `canon grant human:Alex Bryan repo` if that is wrong.
```

The check is `who_decides(scope, now)` after the fold; it is true rather than inferred. Pinned by a CLI test with `CANON_ACTOR=human:Alex Bryan` granting `human:alex`.

## 17. `init` ignores `config`

**Landed with batch A.** `init` writes the same three entries this repository's own `.canon/.gitignore` carries, and a test in `store` holds the two lists together so they cannot drift again.

## 18. `--resume` never opens a checkpoint

**Landed with batch A, inside entry 8.** Run discovery is one function: finished runs oldest first, `.partial.json` set aside and named on stderr as still being drafted, and a run can be picked by file name, by its timestamp alone, or by any tail of its path.

## Candidate-selection trial (2026-09-23)

`scripts/candidate-coverage.py` tests a deterministic lexical top-k selector
against existing Maple House draft artifacts without replacing any reader. On
the demo-tape and guardfix runs, k=15 offered all 7 train/dev planted pairs
whose source sections survived extraction, using 425/1,035 (41%) and
439/1,081 (41%) possible pairs respectively. On guardfix, k=8 offered 5/7
using 244/1,081 (23%). This is a **chunk-anchored upper bound**: a candidate
from the right section need not state the particular rule. It measures neither
model recall nor false positives. The test split was not used to set k.

The full sweep remains the default. Before promoting selected pairs into
`tensions`, compare live model recall and compatible-pair errors against the
full schedule on the same candidates and served model; count pairs never
offered and record the selector and its input alongside the run. The earlier
near-twin block experiment in `tensions.rs` lost recall, so coverage alone
cannot license the change. Dedupe needs a separate measurement of true/false
folds and group stability under the four-item perturbation from entry 22;
`check` needs a proposal-to-commitment shortlist and an explicit partial
answer when some canon IDs were not offered. None can inherit the full
sweep's completeness claim. Exact-hash `seen` cannot match rewordings without
retaining searchable rejected text, a separate ingest-hygiene decision.

Source time now travels through draft chunks for timestamped chat and git
commits, with unknown left unknown. It does not touch the act log or the
fold's `ts_unix`; a historical replay cannot treat a newly accepted rule as
having been in force when its source was written. The existing prompt already
asks for a sentence that makes sense without its passage; entry 12's prompt
change still waits for the same pilot notes and a before/after reading.
For one-step agent capture, entry 7's provenance rule still applies: the
agent can point at a draft candidate or an existing file, but canon must cut
the passage itself before filing a proposal. A typed citation or a bulk
approval would bypass the evidence and review this experiment is meant to
keep. The existing `draft --resume` path retains citations for human review;
an agent-submitted proposal needs a separate measured path before adding a
write tool to the read-only MCP surface.

## What lands together

**Batch A, model-free, no format change, landed 2026-09-13.** Entries 1, 2, 8, 10, 15, 16, 17, 18, and the emphasis strip from 12. Core: `Seat`, `bootstrap`, the gate helper. CLI: `Took`, every gated handler through it, `who`'s two lines, `progress`, `--resume` over every run. Docs: SPEC rule 7 sentence, the COOKBOOK example, the founding line in GETTING_STARTED.

**Batch C, the reader, lands with numbers.** Entries 11 and the prompt half of 12, each with a draft-bar run and the pilot-notes read before and after. 13 waits for the source filter.

**Contracts only, no code.** Entries 3, 4, 5, 6, 7, 9, 14. Each has its canon-side shape above so the Commonwealth-side proof knows what it is proving toward.
