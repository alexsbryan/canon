# A tap changed nothing: /, “Command line”

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T02-56-03-707Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `ed1d9ea`, the product `74265cd`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T18-38-33-385Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `87cf6e3`, the product `cd168e3`
- kind: hard (the instrument's): `inert`; never both

## Where

/, the element “Command line”. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / $ / $

## Who, and how often

- `setup`: 1 of 3 accounts (Sam run 1)

## What happened

- `setup` run 1, Sam: passed, ran out of patience; brought back by a message, then did the scene, and said they were done. inert {"tap":"Command line"}; their last acts: look · tap “Command line”

## What the people said

None.

## The product source

- `demo/xenia/README.md:146` holds “Command line”
- `demo/xenia/README.md:265` holds “Command line”
- `demo/xenia/README.md:271` holds “Command line”
- `crates/canon-cli/src/sources/tests.rs:441` holds “Ctrl-C”
- `crates/canon-cli/src/ui.rs:145` holds “Ctrl-C”
- `demo/xenia/README.md:249` holds “Ctrl-C”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
