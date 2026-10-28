# Too many words before acting: the terminal

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T18-55-28-602Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `fc36366`, the product `963e1cd`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T18-55-28-602Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `fc36366`, the product `963e1cd`
- kind: hard (the instrument's): `wordy`; never both

## Where

the terminal. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / $ / $

## Who, and how often

- `sam-first-hour`: 1 of 1 accounts (Sam run 1)

## What happened

- `sam-first-hour` run 1, Sam: passed, reached their goal. wordy {"words":1038,"budget":300}; their last acts: type “canon help all” into “the command line” · tap “Enter” · scroll · scroll · scroll · type “mkdir -p ~/house && cd ~/house && canon init --profile house” into “the command line”

## What the people said

- Sam, `sam-first-hour` run 1: “almost left: When the help said `add` only *proposes* but the rule came back "in force" immediately off one unstamped act — I nearly stopped to ask whether it silently ratified itself. The log does show a separate `ratify standing` act, so nothing is hidden.”

## The product source

- `demo/xenia/README.md:101` holds “Command line”
- `demo/xenia/findings/findings.json:8` holds “Command line”
- `demo/xenia/findings/findings.json:13` holds “Command line”
- `crates/canon-cli/src/sources/tests.rs:441` holds “Ctrl-C”
- `crates/canon-cli/src/ui.rs:145` holds “Ctrl-C”
- `demo/xenia/README.md:86` holds “Ctrl-C”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/scenes --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon --debug
```
