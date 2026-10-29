# Too many words before acting: /

- status: open, not seen since the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T00-38-52-588Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `e095e4e`, the product `ab59c50`
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T00-38-52-588Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `e095e4e`, the product `ab59c50`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T00-38-52-588Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `e095e4e`, the product `ab59c50`
- kind: hard (the instrument's): `wordy`; never both

## Where

/. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / $ / $

## Who, and how often

- `setup`: 1 of 3 accounts (Sam run 1)

## What happened

- `setup` run 1, Sam: could-not-judge, gave up; brought back by a message, then the scene ran out of time while they were acting. wordy {"words":523,"budget":300}; their last acts: tap “q” · tap “Ctrl-C” · scroll · scroll · scroll · tap “q”

## What the people said

- Sam, `setup` run 1: “gave up: Waiting on the house chat to decide whether June runs draft and we adopt hers, or we draft separately — no reply yet. I've reported the find (canon is per-directory, mine is empty) and given my vote. I'd pick this back up when someone answers.”

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
