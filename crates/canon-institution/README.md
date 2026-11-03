# canon-institution

Optional institutional interpretation over canon-core's mechanisms. The library
is pure `no_std + alloc`; the diagnostic runner owns files. One production
`canon` CLI remains. The [boundary plan](../../docs/INSTITUTION_BOUNDARY.md)
records ownership, compatibility and the operator-approved budget.

Start with [the plain-language walkthrough](../../docs/INSTITUTIONS.md):
a house adopts an agent's proposal, then decides whether to remove it. The
[standalone example](../../examples/institution-house/README.md) checks six
historical views using the actual reader, without a model.

## Two explicit interpretations

- `project` / `--basis`: compatible fixed-basis `shared-assent-v1`, including
  its original limitations. Existing calls retain their behavior.
- `interpret` / `--arrangements`: typed temporal `shared-assent-v2`. It selects
  source-linked effects and uses the neutral core projector; there is no
  second native standing gate.

```sh
cargo test -p canon-institution --offline
target/debug/canon-institution --basis before.jsonl --log after.jsonl --at 1793637169
target/debug/canon-institution --arrangements arrangements.json --log acts.jsonl --at 1793637169
```

The v2 manifest is `Arrangements { boundary, effects, pending }`: explicit
initial facts with source IDs and effective times, plus pending source IDs.
It is an agreed trust root, not authority inferred from an empty record or an
automatic migration from any legacy history. Retain it with the record. Both
manifest and record are fingerprinted in the result. Unknown manifest/effect
fields are refused rather than silently ignored.

Exit 0 returns the interpreted view; pending/refused requests are not errors or
blanket permission. Exit 2 is could-not-judge. Neither input is rewritten.

## Temporal admission

V2 compares mechanical before/after facts to identify affected jurisdictions.
Every required scope's procedure must carry the change. It captures the
procedure and filing jurisdictions when a motion is submitted, then admits
changes at assent time. Later admitted membership changes enter eligibility at
their actual times; proposed grants cannot supply votes for themselves.

Amendments take the old rule's assent and do not lower existing pending bars.
Consent can complete without another append. A settled decision is not undone
by a late objection, and restoring authority cannot backdate a previously
unpassable decision. An adopted agent proposal does not remain the agent's to
repeal. Own pending cancellation and own resignation are explicit profile choices.

`Institution` has private construction and a typed `AdmissionState`; raw
mechanical facts and its pending queue are separate. Serde and `to_json` have
the same representation, including pending work. Decisions carry source IDs,
effective times and per-jurisdiction requirements. The core projector's facts
are not a certificate of assent.

## Evidence and deliberate scope

The ten original histories and initial arrangements are frozen under
[tests/fixtures](tests/fixtures/histories.json). Both interpretations satisfy
the ten predeclared outcomes, while tests preserve the six successful native
controls and the native reader's original results. The interpreter reads no
case identifiers, expected outcomes, actor names or commitment text. Tests build
without xenia, Python or a model.

Initial-arrangement capture lives only in fixture tooling, not the product.
The v2 engine requires that input explicitly. The
[v2 replay report](tests/fixtures/temporal-results.json) retains source-linked
results; the older [v1 report](../../demo/xenia/authority/results/interpreter.json)
remains a compatibility reference.

V2 is an experimental scoped-institution profile, not a complete rights
catalogue. It requires explicitly scoped changes and an established human
electorate. In a known domain, standing is the default if no procedure was
declared. Authority comes from explicit grants, not legacy personal-name
namespace expansion; legacy `governs` shortcuts do not bypass v2 admission.
Objects it cannot classify are reported as unsupported, not assigned invented
authority. Transfers do not automatically reroute already-filed motions.

The production CLI and existing APIs still use the compatible legacy adapter,
with their former clock choices. Adopting v2 for a community is a deliberate
interpretation choice; this work adds no production adoption command and
retires no legacy convenience.
