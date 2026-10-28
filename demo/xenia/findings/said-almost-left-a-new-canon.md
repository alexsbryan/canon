# Almost left: a new canon

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T16-03-37-837Z`, 1 run(s) by claude-sonnet-5-5, as claude -p, xenia `d7600a5`, the product `0f15ca8`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T16-03-37-837Z`, 1 run(s) by claude-sonnet-5-5, as claude -p, xenia `d7600a5`, the product `0f15ca8`
- kind: soft (the person's): what they said when asked "almost left"; never both

## Where

a new canon. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / $ / $ canon --help

## Who, and how often

- `june-draft`: 1 of 1 accounts (June run 1)

## What happened

- `june-draft` run 1, June: failed, ran out of patience. their last acts: look · tell · tell · tell · tell · tell

## What the people said

- June, `june-draft` run 1: “Yes. The help text kept cutting off with "There's more you skimmed past," and I couldn't tell whether a dry run would change the house's records. Then the screen said I'd given up, and I hadn't meant to. I'd have asked Sam, but nobody was around.”

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
