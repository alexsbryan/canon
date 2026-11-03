# Mechanisms and institutional interpretation

Status: slices 1–4 implemented as an experimental optional interpretation; the
production CLI retains its legacy semantics. This records the operator's
2026-11-02 decision: canon offers minimally opinionated governance primitives;
institutional batteries are an optional library in canon, not part of xenia.
One production CLI remains the product. A second executable is packaging, not
the architectural boundary.

## Ownership and target dependencies

```
canon-cli ────────────────→ canon-core
    └─→ canon-institution ─→ canon-core

demo/xenia ─→ the actual application surface
```

The library dependency is established. CLI, UI and MCP reads use the named
legacy adapter with their original clock choices. No production command or
default record interpretation has changed; temporal v2 is an explicit API and
diagnostic-runner choice.

- **canon-core:** record format, normalization, identities, verified evidence,
  references, scopes and grants as data; reusable procedural combinators;
  deterministic mechanical effects. It must offer a path that does not choose
  an institution's electorate, authorship privileges or amendment procedure.
- **canon-institution:** selects requirements from adopted arrangements,
  evaluates assent, owns the meaning of institutional adoption and effective
  authority, and explains each decision. Its library is pure, `no_std + alloc`;
  its diagnostic runner owns files and argument parsing.
- **canon-cli:** identity/time/storage adapters and presentation. CLI, UI and
  agent requests use the same interpretation context. No new production verbs
  or silent change of interpretation is part of this extraction.
- **xenia:** scenes, people, browser/terminal bridges, observations and findings.
  It consumes the product. The interpreter and its semantic tests must build
  without xenia, a browser or a model.

Voting algorithms are not all institutional policy: a tally over an explicitly
supplied electorate is a reusable primitive. Choosing that electorate, deciding
which changes require it, or allowing an author to bypass it is policy. Scope
containment is mechanical; jurisdictional power is not implied by containment.

## Concrete target seam

The neutral API was approved by the operator during implementation. The actual
source-linked seam is:

```rust
// core: kind and signed envelope come from the referenced source record
struct Effect { source: ActId, effective_at: i64 }
Projection::from_effects(history, effects, now) -> Result<Projection, ProjectionError>;

// institution: the interpretation owns adoption and authority
interpret(history: &Log, arrangements: &Arrangements, now: i64)
    -> Result<Institution, String>;
```

`interpret` processes authority changes at their effective moments; a change
cannot supply its own authority. It selects effects, then asks the mechanical
projector to apply them. The projector does not authorize them a second time.
Institutional results name the interpretation/version, evidence, relevant rule,
clock and source history. Raw requests remain in the record.

This is not permission to expose a mutable `Canon` or mint public verdicts.
The institution owns its effective-decision type. A mechanical projection does
not certify that a community adopted a rule. Typed source references survive
application; a derived view is not a rewritten or re-signed source log.

## Compatibility and the dependency-cycle trap

Today's `Log::derive[_at]` and `Canon` expose an opinionated fold. Moving their
implementation wholesale into a crate that depends on canon-core would create
a cycle. A re-export alone does not solve it.

The transition is explicit:

1. Extract mechanical internals behind private functions, used by the current
   fold; preserve every public signature and existing result.
2. Propose a neutral public projection API with exact `contract/api.txt` changes.
   Its approval and any bless are the operator's. The old entry points remain
   a legacy-compatible path during this transition.
3. Implement the temporal institutional interpretation using that neutral path.
   Reuse procedural algorithms; do not copy the old fold into another crate.
4. Route application contexts deliberately. Retiring legacy conveniences is a
   separately versioned API decision, not a side effect of moving files.

No reverse dependency, global interpreter registration or optional-feature
dependency cycle is introduced. The existing core's legacy path remains
temporarily; that is migration debt, not the target policy-free interface.

## Measured cost

Rust product counts use the existing [lean counter](../scripts/lean_check.py): nonblank,
noncomment product lines, excluding test files and inline test modules. They
are not raw file lengths. The reproducible inventory is
[`institution-size.py`](../scripts/institution-size.py), and
[the before snapshot](institution-split-size-before.json) fixes this baseline:

| Component | Product LoC |
|---|---:|
| canon-core | 4,815 |
| canon-cli | 15,293 |
| existing fixed-basis prototype | 295 |
| core fold / ratification / policy, included above | 1,333 / 1,193 / 459 |

The last row is 2,985 lines to classify, not 2,985 lines to rewrite or move.
Types, algorithms and institutional choices share those files today.

| Slice | Existing code relocated/reused | Estimated **net new** product LoC | Tests, separate estimate |
|---|---:|---:|---:|
| 1. First-class library, independent fixtures, private reversal mechanics | 295 prototype + about 60 core | 0–40 | 60–120 |
| 2. Typed mechanical effect/application seam and source/effective-time handling | about 150–300 core | 120–220 | 200–350 |
| 3. Temporal admission, authority updates and pending decisions | existing 269-line prototype and procedural primitives | 120–400 (300–500 added, 100–180 prototype lines retired) | 350–650 |
| 4. Legacy/application adapters and explicit interpretation provenance | existing call paths | 50–100 | 100–200 |

Planning envelope: roughly **290–760 net new product lines**, plus **710–1,320
test lines**, while the legacy path remains. These are estimates, not approved
cap increases. Final cleanup may remove another 100–250 legacy orchestration
lines, but that saving is not credited before it happens. A newly discovered
temporal or wire-format requirement requires re-costing, not quietly expanding
the plan. No plugin framework, second production CLI or complete rights
catalogue is budgeted.

## Slice 1: what starts now

- Move the existing 295-line prototype into `crates/canon-institution` with a
  pure library entry and thin diagnostic binary. Its fixed-basis semantics and
  limitations remain explicit; relocation does not promote it into the full
  temporal interpreter.
- Move semantic case declarations into that crate's test fixtures. Freeze the
  ten original source histories and check them in Rust, without the xenia
  runtime or the Python discovery driver.
- Extract reversal liveness into a private mechanical module in canon-core.
  Its inputs are acts and an explicit admitted-reversal mask. It never asks
  who an actor is, holds standing, or may undo something. The current fold
  still supplies its unchanged legacy admission decisions.
- Keep all existing core/CLI caps. The promoted crate's initial guard is at
  most **320 product lines**: its already-built 295 plus at most 25 lines of
  library-boundary overhead. This is not a budget for new institutional
  features. Extraction should not increase core size; simplify and test the
  mechanical walk rather than expanding its cap.
- Include the crate in normal workspace tests and guard its isolated dependency
  graph. Reuse one LoC counter and one procedural implementation.

Done for this slice: public contracts and native records unchanged; original
thirteen interpreter checks plus ten portable histories green; mechanical
reversal tests green; no interpreter source/build dependency under xenia;
the full gate passes; after counts and remaining seams are reported.

## Slice 1 actuals

[The after inventory](institution-split-size-after.json) uses the same counter
as the before snapshot:

| Component | Before | After | Delta |
|---|---:|---:|---:|
| canon-core | 4,815 | 4,812 | -3 |
| canon-cli | 15,293 | 15,293 | 0 |
| prototype → canon-institution | 295 | 306 | +11 |
| **Total** | **20,403** | **20,411** | **+8** |

The new module contains 37 product lines of reversal mechanics. The old fold
lost 41 and its module declaration adds one. The cache distinguishes unknown
from known liveness; provisional live retains the existing cycle fallback.
Reference-order and undo-of-undo behavior are tested. Authorization remains
entirely with the unchanged legacy gate supplying the mask.

The library/runner move adds eleven lines, stays within its initial 320-line
guard, and adds no model, network or filesystem dependency to the library. The
isolated dependency gate now checks both pure libraries. Its new std-import
guard was watched failing on an unguarded `extern crate std` in the institution
library, then restored and watched passing.

All ten original histories are product-owned frozen fixtures. A Rust integration
check verifies their raw hashes, unchanged native outcomes and all ten optional
outcomes, including the six original controls. It needs no xenia checkout or
Python. The original thirteen interpreter checks also run in the workspace;
four mechanical checks cover the extracted primitive.

Test cost is **134 new Rust code lines**: 55 for the mechanical checks and 79
for the portable history check, against the 60–120 estimate. The 14-line
overrun buys byte-hash verification and comparison of both native and optional
views. Another 504 test-code lines moved with the prototype; those are not new
growth. Fixture JSON, design prose and build/measurement scripts are excluded
from these Rust code counts.

At completion of slice 1, the remaining work was the neutral public application
seam, typed institutional results, effective-time semantics and temporal
admission. The compatibility
`canon_institution::project` retains the prototype's diagnostic JSON interface
and fixed-basis limits. The following slices add the distinct typed
`Institution` API rather than silently changing that probe or the default CLI.
Their public core contract changes were approved separately as recorded below.

## Slices 2–4 implemented

The operator approved additive core API changes, their contract snapshot, and
up to 720 net new Rust product lines beyond slice 1. The generated public
surface was shown before blessing; the committed v0.5 API has no removals or
changed entries. Existing native fixture decisions and minted records remain
the compatibility reference.

### Mechanical projection

`Effect` selects an immutable source record and its effective moment. The core
validates source existence, full-source signatures, duplicate selection,
source/effect time order and structural target order before returning a view.
It applies typed facts without authorship or standing gates. Commitments keep
their original source IDs and written timestamps; grants begin at their
admitted effective times. No source envelope is rewritten or re-signed.

Fact application is shared with the legacy fold for commitments, questions,
grants, withdrawals, positions and annotations, including resource pools and
draw records. The legacy gate still decides admission for legacy readers.
The mechanical serializer omits institutional judgments it did not perform;
selected procedure declarations are not presented as certified human assent.

### Temporal optional interpretation

`Arrangements` supplies an explicit activation boundary, initial effects and
pending source IDs. Initial authority is a declared trust root, not inferred
from an empty record. Inputs and outputs are source-linked and fingerprinted.
The interpreter returns a private-construction `Institution` with typed
`Decision`/`AdmissionState`, mechanical facts and a separate pending queue.
Its serde representation and diagnostic JSON are the same representation.

The v2 admission procedure compares before/after mechanical facts, derives
the affected jurisdictions, and asks the existing ratifier for every one.
Rules and filing jurisdictions are captured at submission. Later admitted
membership changes enter eligibility at their effective times; a requested
grant cannot supply its own vote. Amendments use the old procedure and do not
lower already-filed requirements. Assent deadlines can complete without a new
record. Previously settled decisions are not reopened by late objections.
A later authority restoration cannot backdate a decision previously observed
to be unpassable.

Cancelling one's own pending motion and resigning one's own seat are explicit
v2 policies. Undoing an institutional transfer requires both affected domains.
Same-time cancellation of a still-pending motion is processed before its
same-time adoption. Transfers do not automatically reroute already-filed
motions; their filing jurisdictions and procedures remain explicit in their
requirements.

### Compatibility and limits

`project` and `--basis` retain fixed-basis v1, including its named limitations.
`interpret` and `--arrangements` select temporal v2 explicitly. The production
`canon` surface is unchanged. Its current-clock and last-verified-act-clock
choices are preserved, including the pre-existing MCP clock distinction;
this extraction is not a clock-semantics migration.

V2 requires explicitly scoped institutional changes and an established human
electorate. Within such a domain its default procedure is standing when none
has been declared. It uses explicit grants, not legacy personal-name namespace
expansion; the legacy `governs` shortcut does not bypass v2 admission. These
are choices of the named optional interpretation, not new core laws.
Institutional objects this profile cannot classify produce an error rather
than invented authority. This is not a complete rights catalogue or automatic
migration of arbitrary institutions.

Initial arrangements must be agreed and retained alongside the record. The
library accepts them as an explicit input and reports their fingerprint; it
does not claim to establish the social legitimacy of that root. No new
institutional-adoption production command is introduced here. Legacy reader
retirement remains a separately versioned decision.

### Verification

The original thirteen v1 checks and ten portable cases remain. V2 replays the
same ten histories from frozen explicit arrangements, independently of xenia.
Additional checks cover authorized agent grants, exact effective times,
adoption then cancellation, amendment/non-retroactivity, consent without an
append, changing membership, restored-authority timing, two-domain transfer
undo, strict manifest parsing and input immutability. Both diagnostic modes
are exercised through the real executable. Full native contract records are
required to remain unchanged.

The implementation's measured final inventory is
[institution-split-size-final.json](institution-split-size-final.json).

## Remaining-slice actuals and budget

The approval was for at most 720 net new Rust product lines beyond slice 1,
with legacy results preserved. Actual growth is **719**:

| Component | Slice 1 | Implemented | Delta |
|---|---:|---:|---:|
| canon-core | 4,812 | 5,093 | +281 |
| canon-institution | 306 | 744 | +438 |
| canon-cli | 15,293 | 15,293 | 0 |
| **Total** | **20,411** | **21,130** | **+719** |

Core exceeded its individual 120–220 estimate because it gained validated
source/effective-time application and shared annotation mechanics, not just a
thin filter. The later slices used less than their combined upper estimate:
the compatibility adapter delegates instead of adding another application
drive. Existing v1 remains for API compatibility; its requirement conjunction
is shared with v2, not another quorum implementation.

New test code is **760 lines** beyond slice 1 (211 core, 549 institution), within
the remaining 650–1,200 estimate. Captured JSON histories and initial facts are
data, not Rust code. The initial arrangements are frozen inputs, not expected
results generated by the new interpreter.

Per-crate guards are now 5,093 core, 747 institution, 15,297 CLI and 220 contract.
There is also an aggregate **21,350** product-line guard: slice-1 actual 20,630
including the contract crate, plus the approved 720. Actual is **21,349**.
Existing headroom in another crate cannot silently spend beyond that approval.
Further work needs real deletions or another operator re-costing decision.

The final core surface diff is **23 added entries, zero removed/changed
pre-work entries**. It was reviewed through the failing contract check and
blessed under the operator's explicit authorization. Native CLI and fixture
contracts remain unchanged. The changelog distinguishes mechanical projection
from institutional assent and documents the opt-in interpretation.
