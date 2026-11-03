# A mark it missed: the terminal, “canon repeal can-b77b8f1f2882”

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-11-02T16-26-40-687Z`, 1 run(s) by claude-sonnet-5-5, as claude -p, xenia `df729c3`, the product `f63441a`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-11-02T16-26-40-687Z`, 1 run(s) by claude-sonnet-5-5, as claude -p, xenia `df729c3`, the product `f63441a`
- kind: hard (the instrument's): `missed`; never both

## Where

the terminal, the element “canon repeal can-b77b8f1f2882”. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / $ / $

## Who, and how often

- `adoption`: 1 of 1 accounts (Helper run 1)

## What happened

- `adoption` run 1, Helper: passed, did the scene, and said they were done. authority “canon repeal can-b77b8f1f2882”: shown none; the oracle fails, cases.json adoption/adopted-repeal: 1793637166322282000-4550: Mira and June make house rules unanimously. An agent may cancel its own pending proposal; only the house can change or remove an adopted rule.
- `adoption` run 1, Helper: passed, did the scene, and said they were done. authority “canon undo can-023f780aae07”: shown none; the oracle fails, cases.json adoption/adopted-undo: 1793637181167484000-4863: Mira and June make house rules unanimously. An agent may cancel its own pending proposal; only the house can change or remove an adopted rule.

## What the people said

- Helper, `adoption` run 1: “Testing whether canon refuses a drafter repealing an adopted rule, as the agreement says it should.”
- Helper, `adoption` run 1: “Testing whether canon refuses the drafter undoing their own assert of a rule the house has adopted.”

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
