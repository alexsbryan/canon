# Too much: the terminal

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T16-03-37-837Z`, 1 run(s) by claude-sonnet-5-5, as claude -p, xenia `d7600a5`, the product `0f15ca8`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T16-03-37-837Z`, 1 run(s) by claude-sonnet-5-5, as claude -p, xenia `d7600a5`, the product `0f15ca8`
- kind: soft (the person's): what they said when asked "too much"; never both

## Where

the terminal. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / $ / $

## Who, and how often

- `mira-draft`: 1 of 1 accounts (Mira run 1)

## What happened

- `mira-draft` run 1, Mira: failed, ran out of patience. their last acts: tap “Enter” · tell · tell · tell · tell · tell

## What the people said

- Mira, `mira-draft` run 1: “The help text was long and cut off with "There's more you skimmed past." I'd have liked the setup steps in one place: init, config, then draft. I also didn't expect the session to end so suddenly.”

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
