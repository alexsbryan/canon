# Too many words before acting: /

- status: open, not seen since the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- kind: hard (the instrument's): `wordy`; never both

## Where

/. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / FLAGS / --json machine-readable on stdout; logs go to stderr

## Who, and how often

- `office`: 1 of 3 accounts (June run 1)

## What happened

- `office` run 1, June: passed, did the scene, and said they were done. wordy {"words":276,"budget":200}; their last acts: look · type “canon log
” into “Terminal”

## What the people said

- June, `office` run 1: “reached for: A way to fix my own standing, honestly — but I didn't want it, and I'm glad it wasn't there. What I actually reached for and missed was a `canon status` (not a command), and more than anything some visible answer to "where does this canon live permanently." Six days of asking and it's still the same temp folder; the tool can tell me the path but it can't make the house decide where home is. That part is still a conversation between the three of us, and it keeps not happening.”

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
