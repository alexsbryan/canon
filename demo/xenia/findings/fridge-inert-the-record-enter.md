# A tap changed nothing: the record, “Enter”

- status: open, not seen since the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T00-38-52-588Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `e095e4e`, the product `ab59c50`
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T00-38-52-588Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `e095e4e`, the product `ab59c50`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T00-38-52-588Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `e095e4e`, the product `ab59c50`
- kind: hard (the instrument's): `inert`; never both

## Where

the record, the element “Enter”. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / GOVERN (no model needed) / who [<scope>] who may decide this, and under what;

## Who, and how often

- `fridge`: 1 of 3 accounts (June run 1)

## What happened

- `fridge` run 1, June: passed, did the scene, and said they were done. inert {"tap":"Enter"}; their last acts: tell · tell · tell · type “canon who; canon list; canon log” into “Command line” · tap “Enter” · tap “Enter”

## What the people said

None.

## The product source

- `demo/xenia/README.md:143` holds “Command line”
- `demo/xenia/README.md:172` holds “Command line”
- `demo/xenia/README.md:178` holds “Command line”
- `crates/canon-cli/src/sources/tests.rs:441` holds “Ctrl-C”
- `crates/canon-cli/src/ui.rs:145` holds “Ctrl-C”
- `demo/xenia/README.md:156` holds “Ctrl-C”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
