# A tap changed nothing: /, “Command line”

- status: open, not seen since the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- kind: hard (the instrument's): `inert`; never both

## Where

/, the element “Command line”. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / FLAGS / --json machine-readable on stdout; logs go to stderr

## Who, and how often

- `office`: 1 of 3 accounts (Mira run 1)

## What happened

- `office` run 1, Mira: passed, did the scene, and said they were done. inert {"tap":"Command line"}; their last acts: look · scroll · tap “Quit” · scroll · look · tap “Command line”

## What the people said

- Mira, `office` run 1: “reached for: A path. `canon config show` told me endpoint and model but not where the canon actually lives, so I still can't confirm whether my house is the temp folder or has moved. I also wanted a plain "where am I / is this permanent" check and the exact init line for a permanent home — neither was on the screen. And I reached for a way to look at "my" house versus the shared one; the log reads as one, so I couldn't tell if my temp copy has Sam's rules or only Sam's does.”

## The product source

- `demo/xenia/README.md:146` holds “Command line”
- `demo/xenia/README.md:256` holds “Command line”
- `demo/xenia/README.md:262` holds “Command line”
- `crates/canon-cli/src/sources/tests.rs:441` holds “Ctrl-C”
- `crates/canon-cli/src/ui.rs:145` holds “Ctrl-C”
- `demo/xenia/README.md:240` holds “Ctrl-C”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
