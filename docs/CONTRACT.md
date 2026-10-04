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

Declare the effect in the [pull request template](../.github/PULL_REQUEST_TEMPLATE.md):

- **Unchanged:** another implementation of the same promised behavior.
- **Extended:** new vocabulary or behavior that preserves the promises to
  existing records and consumers. An extra enum variant is not automatically
  compatible with an exhaustive caller.
- **Breaking:** existing records or consumers lose a promised meaning. Name
  the affected laws and include an old-record-to-new-interpretation test, plus
  the migration or explicit reinterpretation plan.

Meaning can change without changing a field or format version. The
[0.2.0 release](../CHANGELOG.md#020--2026-10-03), which made old agent writes
in open scopes read as proposals, is one example. Review determines the
classification; passing the same tests alone does not determine it.
