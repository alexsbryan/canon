# Didn't do what they expected: the raw acts

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T16-03-37-837Z`, 1 run(s) by claude-sonnet-5-5, as claude -p, xenia `d7600a5`, the product `0f15ca8`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T16-03-37-837Z`, 1 run(s) by claude-sonnet-5-5, as claude -p, xenia `d7600a5`, the product `0f15ca8`
- kind: soft (the person's): what they said when asked "not as expected"; never both

## Where

the raw acts. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / why <id> what replaced what, when, and why / log the raw acts, oldest first

## Who, and how often

- `june-explore`: 1 of 1 accounts (June run 1)

## What happened

- `june-explore` run 1, June: passed, explored, and said they were done. their last acts: tap “Enter” · tell · tell · tell · tell · tell

## What the people said

- June, `june-explore` run 1: “Only one thing. I typed a command, and the screen didn't change until I pressed Enter, so I had to tap Enter as a separate step. After that, everything did what I expected. I never saw an error, and I didn't try any command that writes.”

## The product source

- `demo/xenia/world/scene.mjs:45` holds “Command line”
- `demo/xenia/world/scene.mjs:47` holds “Command line”
- `demo/xenia/world/scene.mjs:61` holds “Command line”
- `crates/canon-cli/src/sources/tests.rs:441` holds “Ctrl-C”
- `crates/canon-cli/src/ui.rs:145` holds “Ctrl-C”
- `demo/xenia/README.md:73` holds “Ctrl-C”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/scenes --runs 1 --model claude-sonnet-5-5 --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
