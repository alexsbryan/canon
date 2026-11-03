# Ship a UI people can use daily

Implementation update: browser-first setup and UI-1–4 are implemented in this
working tree, with setup prioritized alongside the participant loop. The
operator approved a single +300-line CLI/aggregate allowance for the combined
slice. Verification covers native initialization parity, signed browser writes,
decision/history routes, and injected failures. The task breakdown below is
retained for reference; notes to proposals is the next unimplemented journey.
Real-user observation remains follow-up work.

**First release: a person can open an existing canon, find what needs their
decision, understand it, act, and follow the actual result into history.**

Build this in four small PRs, try it with the group, then choose the next
release from what blocked them. Estimate: **4–6 engineering days for one
implementer**, including checks and a feedback session. This is a planning
estimate, not a delivery promise. Keep the scope small enough to demo daily.

The [architecture report](../reports/Canon%20UI%20front%20door%20architecture.md)
contains the investigation and comparisons. This file is the execution plan.

## Working decisions

- Keep the existing Rust server, HTML forms, single binary, and local browser
  delivery. First release adds no frontend framework, package manager, or JS.
- Keep decision-making in the existing Rust mechanisms. Reuse
  `review`, `explain`, and the shared write helpers; extract a helper when the
  task needs it. There is no prerequisite application-layer rewrite.
- Use one selected record and acting identity per server. Show that context
  on every task page. Ordinary work keeps its current interpretation.
- Ship each PR when its behavior check and the existing gate pass. Keep each
  change independently reviewable and easy to revert.
- Watch two or three actual users do the task with their own record. Note
  where they need help; fix the biggest blocker before adding another area.

Borrow responsibility patterns from the reference apps as they become useful:
shared Rust use cases and explicit request outcomes from svrngs; connected
record/source/history views and recoverable long work from the Commonwealth
desktop client. Apply them to a feature rather than porting their infrastructure.

## Release 1: finish the participant loop

Start with `canon ui`: open the discovered record or explicitly create one in
the browser. Creation uses the launch target and acting identity, never paths
or author names posted by the browser. The participant demo is:

> Open the house record as Sam. Find an agent's proposed rule or policy change.
> Read its wording, source information, and required approvals. Approve or
> object with a reason. See whether it is now effective or still waiting.
> Follow its history and supporting entries without a dead link.

These tasks are ordered. Each PR ships useful behavior; refactoring stays
inside the PR that needs it.

### UI-1 — Make history and failed reads reliable · 0.5–1 day

**Change:** In [ui.rs](../crates/canon-cli/src/ui.rs), make `why` use
`explain::resolve_why(&st, &log, needle)` instead of the narrower `resolve_any`.
Use the existing [explanation implementation](../crates/canon-cli/src/explain.rs).
Give failed record loads an error response and a retry path. Remove the
`/decide` fallback that turns a read failure into an empty queue.

**Done:** A rule, grant, decision-procedure entry, and silence each open through
`/why/<id>`. A missing ID is still a 404. An unreadable record shows an error,
never “nothing waiting”; submitting against that failure writes nothing.
Add focused route regressions to the existing UI tests.

### UI-2 — Include every supported pending decision · 0.5–1 day

**Change:** Extend `Kind`, `queue`, `standing`, and `outcome` in
[review.rs](../crates/canon-cli/src/review.rs) for `pending_policies`, using the
same interpretation and hypothetical-position calculation as other proposals.
Render the added kind in the existing UI card. Keep proposals waiting on
other people visible without offering an unauthorized action.

**Done:** A pending policy appears alongside rule, amendment, repeal, and
procedure proposals. An eligible person's approval or reasoned objection
writes the same position as the equivalent terminal action and displays the
actual result. Terminal review and UI share the queue. Use a governed fixture;
do not hand-code eligibility in the renderer.

### UI-3 — Tell the truth after a write and retain input · 1–2 days

**Change:** Audit `propose`, `replace`, `decide`, `did_note`, and their error
rendering in [ui.rs](../crates/canon-cli/src/ui.rs). The shared
[propose/replace helpers](../crates/canon-cli/src/cmds.rs#L544-L604) can append a
rule before failing to append its room. Replace blanket “Nothing was written”
on uncertain failures with an accurate result and a history/reload path.
Never infer “applied” from a failed reread, including the `cmds::took` fallback;
reuse or extract a fallible observation helper without changing normal CLI
output. Retain submitted wording/reasons, escaped, when rendering errors.

**Done:** Validation failure retains the form and writes nothing. An injected
second-append failure says the request may be partly recorded, identifies
known recorded IDs, and offers inspection rather than blind resubmission.
A failed post-write reread says the outcome is unknown, not successful or
unwritten. Completed reads distinguish recorded, pending, effective, and
refused. The UI never automatically retries an uncertain write.

Refresh and check eligibility at submission, using the shared behavior. This
does not make multiple writers transactional: test stale-form refusal and
partial-write reporting, and keep cross-process locking/atomic multi-act
operations as a separately sized issue if the pilot needs that guarantee.
Browser requests keep the existing host, token, origin, and signing gates.

### UI-4 — Make the existing page understandable · 0.5–1 day

**Change:** Tighten `home`, `why`, and existing page helpers in
[ui.rs](../crates/canon-cli/src/ui.rs): show record, acting identity, and current
reading context consistently; provide simple links/anchors for “For you”,
“Rules”, and the current proposal/history. Make the requirement and return
path easy to find. Capture one current-view instant per render where needed;
this introduces no historical-view picker or change to another adapter's clock.

Label source material accurately: a current local-file excerpt, a source
reference, or unavailable context. A fresh excerpt must not claim to be the
original saved quotation. Empty decision queues describe decisions, not the
overall health of the record.

**Done:** Complete the release demo using keyboard only and at a narrow
viewport. Names, requirements, status, and evidence remain readable; errors
and outcomes are announced. No new navigation destination is empty or a stub.

### Release and feedback · about half a day

Run the existing UI tests and `./scripts/pre-push.sh`. Exercise the demo against
a real disposable record through the real server; verify resulting acts and
outcomes, not just rendered strings. Include the failure cases above. Add
the smallest browser journey needed for focus/form retention; use existing
test tooling rather than a new testing platform.

Watch two or three people use it without terminal help after launch. Record
whether they can say **which record, who acted, what they agreed to, and
whether it took effect**. Fix misleading status, lost input, or broken evidence
links before broadening the release. Write one short note: biggest blocker,
what changed, and the next task. No analytics service or formal study required.

## Setup and the next release

Browser setup below is now implemented with release 1. Notes to proposals is
the next proposed release; use feedback from the combined journey to keep it
bounded to what people need next.

| Release | Concrete scope and code seam | Done when | Rough effort |
|---|---|---|---|
| **2. Enter through the browser** | Let `ui::run` reach setup in an empty launch directory. Offer explicit creation using the existing `cmds::init` behavior, extracting its filesystem/record work from CLI printing only as needed. Show profile, actor and founding authority before creation; keep offered-key identification. Reuse current record resolution for opening. | From an empty directory, `canon ui` opens setup; a deliberate submit creates the same initial record as the equivalent `canon init`. An existing record opens normally. Someone can identify, manually propose, and return to review. No model needed. | **2–3 days** |
| **3. Notes → checked proposal** | Add one bounded text-source selection flow and minimal model configuration using `config.rs`/`model.rs`. Extract the needed noninteractive orchestration from `draft.rs`, reusing its saved runs and candidate review. Show saved quote/context beside each candidate; accept/edit/reject/skip individually. Start with one run per record, a run ID, saved status, and simple status refresh. | Choose a text source, see the model destination, start a run, leave/reopen its page, and review candidates without rerunning extraction. A failed extraction retains its run and explains recovery. Accepting a candidate records what the terminal flow would; assent to a proposal stays a separate step. | **3–5 days** |

Creation uses a server-chosen launch target, not arbitrary filesystem paths
posted by a browser. Source selection uses bounded uploads or an explicitly
selected server-side source root. Model calls retain current endpoint/remote
permission rules. Signing, omissions, and errors stay visible.

Release 3 is the first point where a small amount of progressive-enhancement
JS may pay for itself. Count it in that PR. Persist the draft/run, not a
framework's component state. Reconsider a renderer only if this actual screen
is materially harder to build and test with the current approach.

## Queue the rest by observed need

Keep these as independently shippable issues, not prerequisites for the first
release. Each starts with one existing capability and its shared Rust behavior:

1. **Change your mind:** cancel your own pending work, revise a position, or
   undo an act where existing rules allow it. Reuse the shared write/answer
   helpers and explain the result; do not invent new author privileges.
2. **Questions and due work:** list, ask, answer, revisit; distinguish private
   checking from public recording. Reuse `check.rs`, `cmds.rs`, `govern.rs`.
3. **People and how we decide:** read seats, terms, rooms and procedures first;
   then one seat/procedure-change journey. Reuse `govern.rs` and explanations.
4. **Share a record:** export and inspect a snapshot/lineage first; then scoped
   fork/adopt/upgrade flows. Reuse `lineage.rs`; keep local notes/run artifacts
   out of incidental exports.
5. **Shared things:** show a rotation/turn, then complete one draw using
   `govern.rs` and `draw_cmd.rs`.
6. **Conflicts and comparisons:** existing conflict/revisit workflows and
   read-only `replay.rs` comparison before additional editing surfaces.
7. **Institution preview:** the six-date house example, read-only, with explicit
   arrangements and fingerprints. Product adoption is a separate interpretation
   decision; a UI setting must not silently change existing records' meaning.

Keep agent setup lightweight: explain how to connect the existing four
read-only MCP tools. A write-capable agent API is separate product work.

## Budget and shipping discipline

The operator approved the combined setup/UI-1–4 allowance after an economizing
pass: CLI cap **15,597**, aggregate **21,650**, each raised by 300 in the
[lean guard](../scripts/lean_check.py). Shared staged initialization replaces
the CLI-only setup body; existing review, explanation, and native-form rendering
are reused. The ceiling is a stop/re-scope point, not a target to fill; no core
cap or public API allowance is added.

Combined implementation actuals: **227 net Rust product lines** above the
pre-UI baseline: CLI **15,293 → 15,520**, aggregate **21,349 → 21,576**.
Native forms and styles remain embedded and counted; no runtime frontend
dependencies were added. Checks include 23 UI unit tests, three socket
journeys, and a real Chromium walkthrough at 320px and 200% text size.

Review each PR's actual additions, deletions, and net product delta with the
existing counter. HTML/CSS embedded in Rust already count; moving them into
files does not buy growth. The first release needs no separate frontend
inventory project. When the first external assets appear, add their authored
size/dependencies to the same review and a small enforceable guard. Re-cost
releases 2 and 3 after the first release's actuals, rather than fund a speculative
multi-week platform up front.

Use a few behavior checks that pin the task and its meaningful failure case;
reuse existing fixtures and the required gate. Do not add abstractions, screens,
or tests just to complete a pattern. If a PR needs more than two days, split
out a smaller usable result. Drop cosmetic work before failure recovery.

The first release is scoped to private UI behavior and shared bug fixes, with
no planned core API or record-format change. If a contract test identifies an
intentional change, show its exact diff and follow [the existing contract
rules](../CLAUDE.md#the-contract). Keep that exception distinct from everyday
implementation choices.

**Next: try the combined browser-first journey with the group**, then prioritize
the biggest blocker before starting notes-to-proposal work.
