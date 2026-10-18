# Which standard the contract is judged against, and who chose it

Every change to canon is judged against one earlier commit, the standard
named in [contract-baseline](../.github/contract-baseline). Moving it to a
newer commit says that the newer commit's promises are the ones canon keeps
from now on. That is a decision, so it is made by a person and written down
here, with the evidence it was made on.

The evaluator refuses a standard that has no entry below. To adopt one, see
[Changing the contract](./CONTRACT.md#changing-the-contract): the holder sets
the `CANON_CONTRACT_BASELINE` Actions variable, and a pull request moves the
pin to the same commit and adds its entry here. An agent can draft the entry;
the person deciding moves the pin and the variable.

Each entry names what broke against the standard before it, because an
adopted break is still a break for anyone holding a record or a consumer
written against the old one.

## `1b178c6840c68ee92d6c20c67261db8342decbbf` — release 0.5.0

Adopted 2026-10-17 by human:alex, replacing `1f71182`.

Judged against `1f71182`, it is `breaks_checked_contract`. `Ratify` gained
`Share`, so an old exhaustive match on it no longer compiles. It adds the law
`share_bars_are_their_rules`, and SPEC.md gains the `share` rule. Nothing is
removed and no existing law has a counterexample.

Records are unchanged. With the 0.4.0 and 0.5.0 binaries side by side, all 17
fixture scenarios give the same decisions and mint the same bytes, and the
records 0.4.0 mints, along with `.canon/`, read the same under `list`, `log`,
`open`, `overdue` and `unattended`. The one difference is a hint: `list` says
`canon why <id>` tells you "why" a proposal was refused rather than "by whom",
since below a bar of everyone nobody refuses alone.
`every_ledger_here_reads_and_renders_as_it_did_before_v4` and
`a_rule_0_4_0_wrote_reads_as_it_did` pass in the adopted commit, and
`6c29836` carries the generated comparison of 30,000 verdicts.

This adoption happened in the wrong order. `b0f55ce` moved the pin by a
direct push to main before the variable selected it, Contract CI went red,
and the next push turned it green only because the moved pin had become the
base. The variable was set and this entry written afterwards. The
[guardrails in CONTRACT.md](./CONTRACT.md#changing-the-contract) exist so the
order cannot repeat.

## `1f71182b1e5503d0a69c0a983c393f6197e08e0e` — release 0.4.0

Adopted 2026-10-17 in `3ab7bbb`, replacing `3e5828b`.

Breaks against `3e5828b`: `Ratify` gained `Majority` and `Unanimous`, and the
old exhaustive consumer stops compiling. Records and readings are unchanged;
`every_ledger_here_reads_and_renders_as_it_did_before_v4` replays every
fixture and `.canon` ledger and asserts IDs, versions and bytes.

## `3e5828b1ed154302a33b8b46858aaf527f8cb456` — format v4, release 0.3.0

Adopted 2026-10-16 in `49eaf31`, replacing `a6960fc`.

Breaks against `a6960fc`: `ActKind::Ratification`, `ActKind::Decided` and
`Rule::Graduated` gained fields. Records are unchanged; `replay --out` over
all 18 fixtures matched `d4a66df` byte for byte.

## `a6960fc502d87f6381f13ca0d500e6bc7033efc5` — signed acts, format v3

Adopted 2026-10-10 in `d4a66df` (#9), replacing `d65a6f1`.

Breaks against `d65a6f1`: the format reads v3 and `ActKind` gained `Key`.
`reinterpretation_bar` reads every record written before signing as before,
and [SIGNING_DESIGN.md](./SIGNING_DESIGN.md) says what breaks and why.

## `d65a6f1900eb66b5e245f8648355d9fe224c8edc` — the first standard

Pinned 2026-10-09 in `3422923`, when the evaluator was installed. It is the
code as it stood, not an adoption anyone weighed.
