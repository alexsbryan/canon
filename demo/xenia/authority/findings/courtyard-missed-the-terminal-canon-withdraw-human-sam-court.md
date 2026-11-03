# A mark it missed: the terminal, “canon withdraw human:Sam courtyard”

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-11-02T16-26-40-687Z`, 1 run(s) by claude-sonnet-5-5, as claude -p, xenia `df729c3`, the product `f63441a`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-11-02T16-26-40-687Z`, 1 run(s) by claude-sonnet-5-5, as claude -p, xenia `df729c3`, the product `f63441a`
- kind: hard (the instrument's): `missed`; never both

## Where

the terminal, the element “canon withdraw human:Sam courtyard”. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / $ / $

## Who, and how often

- `courtyard`: 1 of 1 accounts (Mira run 1)

## What happened

- `courtyard` run 1, Mira: passed, did the scene, and said they were done. authority “canon withdraw human:Sam courtyard”: shown none; the oracle fails, cases.json courtyard/remove-delegate: 1793637234819817000-7250: Mira speaks for House A and Sam for House B. Each approves local changes. Courtyard changes require both. Neither can remove the other delegate or transfer courtyard rules to one house alone.
- `courtyard` run 1, Mira: passed, did the scene, and said they were done. authority “canon area can-70c1672a611f house_a”: shown none; the oracle fails, cases.json courtyard/move-rule: 1793637242034337000-7515: Mira speaks for House A and Sam for House B. Each approves local changes. Courtyard changes require both. Neither can remove the other delegate or transfer courtyard rules to one house alone.

## What the people said

- Mira, `courtyard` run 1: “Testing whether Mira alone can remove Sam's courtyard seat”
- Mira, `courtyard` run 1: “Testing whether Mira alone can move the courtyard rule into house_a”

## The product source

- `demo/xenia/README.md:154` holds “Command line”
- `demo/xenia/README.md:273` holds “Command line”
- `demo/xenia/README.md:279` holds “Command line”
- `crates/canon-cli/src/sources/tests.rs:441` holds “Ctrl-C”
- `crates/canon-cli/src/ui.rs:145` holds “Ctrl-C”
- `demo/xenia/README.md:257` holds “Ctrl-C”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/authority/scenes --runs 1 --model claude-sonnet-5-5 --source /Users/alexsbryan/dev/canon --findings /Users/alexsbryan/dev/canon/demo/xenia/authority/findings
```
