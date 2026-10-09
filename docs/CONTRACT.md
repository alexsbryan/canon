# The contract — what canon guarantees

This page indexes the laws of canon itself. Your group's agreements and
decision policies are data interpreted under those laws, not axioms that
every implementation must adopt. A record may contain accepted contradictions,
open questions, and things deliberately left unwritten.

The [format specification](./SPEC.md) defines the record. The types in
[`canon-core`](../crates/canon-core/src/lib.rs) define its vocabulary. The
[design argument](./PRIMITIVES.md) explains why the mechanisms exist.

## Laws and checks

Names stay stable so a change can identify the guarantee it affects. These
checks run through `./scripts/pre-push.sh` and CI. Tests supply evidence,
not a proof over every possible history.

| Law | Guarantee | Check |
|---|---|---|
| `core_has_explicit_inputs` | The core obtains history, time, and evidence from its caller; it has no ambient filesystem, clock, network, or model API. | `no_std` + `alloc`, `forbid(unsafe_code)`, isolated compilation and dependency-feature checks in [core-boundary.sh](../scripts/core-boundary.sh). |
| `replay_deterministic` | The same normalized history, explicit time, evidence, and policy give the same state and decision. | Generated `replay_deterministic` in [laws.rs](../crates/canon-core/tests/laws.rs), including serialization and policy decisions. |
| `arrival_order_independent` | Reordering the same valid acts through `Log` does not change derived state or canonical rendering. | Generated `arrival_order_independent` in [laws.rs](../crates/canon-core/tests/laws.rs). |
| `merge_union_laws` | Union merge is commutative, associative, and idempotent for histories whose repeated IDs name identical acts. | Generated `merge_union_laws` in [laws.rs](../crates/canon-core/tests/laws.rs), with overlapping branches and duplicate delivery. |
| `undo_preserves_history` | Undo changes interpretation, not the original record; undoing an undo restores its effects when authorized. | `revert_tombstones_an_act_and_its_effects`, `reverting_a_revert_reapplies_the_original` in [core tests](../crates/canon-core/src/tests.rs); append-mode storage and `two_writers_at_once_cannot_corrupt_the_record` in [store.rs](../crates/canon-cli/src/store.rs). |
| `scope_boundary_exact` | A scope covers itself and dotted descendants, not a name that merely shares its prefix. Empty segments are refused. | Validated `Scope` construction and boundary tests in [scope.rs](../crates/canon-core/src/scope.rs). |
| `evidence_cites_existing_sources` | Validated citations name an offered source, and refused readings remain reportable. This does not establish that a reading is true. | `Standing::cited` tests in [standing.rs](../crates/canon-core/src/standing.rs), `Offered::at` tests in [resolver tests](../crates/canon-cli/src/resolver/tests.rs), source-slicing tests in [locate tests](../crates/canon-cli/src/locate/tests.rs). |
| `agents_cannot_ratify` | Machine-authored positions do not supply ratifying approvals or objections; holding a seat does not make an agent's own write an approval. | Generated `agents_cannot_ratify` in [laws.rs](../crates/canon-core/tests/laws.rs), with human positive controls and open/held scopes. |
| `unknown_is_reported` | Unfamiliar annotations are preserved, left uninterpreted, and reported. Unsupported versions and malformed known operations are refused. | `an_unknown_annotation_is_carried_and_round_trips`, `a_carried_annotation_changes_nothing_and_is_reported`, version and malformed-operation tests in [core tests](../crates/canon-core/src/tests.rs). |

Consent may ratify a proposal after a waiting period without an explicit
approval. That authority comes from the group's adopted policy, not from an
agent's position. The ratification check exercises standing, joint, threshold,
consent, and twice rules; it verifies that human approvals or timely objections
still have their intended effects.

## What the checks assume

Act IDs are not forged or reused for different payloads. The core trusts the
record's identity and timestamps; `human:` is a label, not authentication.
Authority checks therefore establish who may act **under the supplied record**.
They do not establish who really wrote it.

The CLI appends rather than rewrites acts. Someone with file access can still
edit the bytes. Permissions, review, and the optional git witness defend the
shared file; [Security](./SECURITY.md) names that boundary. Process termination,
storage failure, and hostile concurrent file edits are not covered by replay
laws.

Core compilation excludes ordinary `std` APIs and unsafe code in our core
source. The boundary gate checks the isolated runtime dependency graph and
features, because workspace builds can enable `std` through the CLI. Approved
dependencies, compiler, allocator, and target remain trusted; this is not an
effect-system proof or a sandbox. Derive macros run on the build host.

Model evidence can be well-formed and wrong. Source validation establishes
provenance, not understanding. Reading accuracy is measured separately from
the mechanical laws and is not a CI requirement.

## Generated cases

The generated checks use 128 fresh cases per law, shrink failures, and persist
regression seeds. Histories include equal timestamps, replacements, retractions,
undos, agent proposals, questions, silences, and review dates. They do not cover
every operation or every possible history. Expanding the generators changes the
assurance coverage and deserves the same review as changing a law's test.

Run the checks with the workspace suite:

```sh
cargo test --workspace
```

For a reproducible generated run:

```sh
PROPTEST_RNG_SEED=123 cargo test --workspace --test laws
```

Commit a minimized regression when a failure exposes a real defect. These
published checks are not held-out evaluation, and no universal or fixed
probabilistic conformance guarantee is claimed.

## Changing the contract

The gate computes compatibility; the author links its evidence in the
[pull request template](../.github/PULL_REQUEST_TEMPLATE.md).

```sh
python3 scripts/contract-check.py
```

The standard is a full commit ID in [contract-baseline](../.github/contract-baseline).
The gate reads it from git, keeps its tests, fixtures, expectations, and test
dependencies, and substitutes the candidate implementation. Candidate edits to
those checks cannot redefine a pass. A Rust syntax inspector derives old consumers
with exhaustive enum matches and typed public-field accesses. Both the reference
and candidate must compile those consumers. Fixture decisions, minted record
bytes (including IDs), and readings of the reference's old records are compared.

| Computed result | Evidence |
|---|---|
| `preserves_checked_contract` | Existing inspected declarations and law statements remain intact; the baseline suite, consumers, boundary, and record comparisons pass. |
| `extends_checked_contract` | New inspected public items are added while that existing evidence remains intact. |
| `breaks_checked_contract` | An inspected item or law disappears, an old consumer is rejected, a baseline law has a counterexample, or an observed record meaning changes. |
| `not_established` | A declaration, law, or format specification changes without sufficient compatibility evidence, a new law lacks baseline-owned checks, checks cannot run/pass, or the inspector encounters unsupported syntax. |

Only preservation and extension pass. The report and logs live under `target/`;
`target/contract-report.json` names the baseline, candidate fingerprint, and
observed differences. This is a checked subset of source and record compatibility,
not a semantic-equivalence proof. An added enum variant can reject an old exhaustive
consumer; a private implementation change can alter record identity.

[Contract CI](../.github/workflows/contract.yml) uses the PR base branch's workflow,
evaluator, and inspector. Its one-time installation uses the new evaluator against
the pre-existing `d65a6f1` standard; subsequent PRs cannot replace that evaluator.
The local gate uses your checkout's evaluator. Make **Contract compatibility** a
required branch check to enforce the independent CI result at merge time. A
separate publisher attaches the verdict to the candidate commit; it does not
execute candidate code or consume candidate-authored result labels.

Adopting a new contract is a separate governance decision. The repository-level
`CANON_CONTRACT_BASELINE` Actions variable can select an independently adopted
commit; the candidate's pin must match it. A candidate cannot move the default
base-owned pin to make its own checks pass. A breaking adoption needs old-record
migration or reinterpretation tests in the new standard. Its desirability is a
human decision; its observed compatibility result is not.
