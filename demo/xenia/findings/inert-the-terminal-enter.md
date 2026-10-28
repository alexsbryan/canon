# A tap changed nothing: the terminal, “Enter”

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T18-55-28-602Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `fc36366`, the product `963e1cd`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T18-55-28-602Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `fc36366`, the product `963e1cd`
- kind: hard (the instrument's): `inert`; never both

## Where

the terminal, the element “Enter”. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / $ / $

## Who, and how often

- `sam-draft`: 1 of 1 accounts (Sam run 1)

## What happened

- `sam-draft` run 1, Sam: failed, ran out of patience. inert {"tap":"Enter"}; their last acts: tap “Enter” · type “which canon; canon --help” into “Command line” · tap “Enter” · type “canon help all” into “Command line” · tap “Enter” · tap “Enter”
- `sam-draft` run 1, Sam: failed, ran out of patience. inert {"tap":"Enter"}; their last acts: scroll · type “pwd; ls -a; canon config” into “Command line” · tap “Enter” · type “canon init --profile house; canon config; curl -s http://localhost:9741/v1/models” into “Command line” · tap “Enter” · tap “Enter”

## What the people said

- Sam, `sam-draft` run 1: “almost left: Yes, one: right after `canon init` succeeded and I tried to run `canon config` plus a curl against the local endpoint to see whether the model answers. The screen cut over to "You've given up" while I was still working. I nearly stopped a step earlier too, when the first two typed commands sat unsubmitted in the field and produced no output — I nearly concluded the terminal was dead rather than that I needed to press Enter.”
- Sam, `sam-draft` run 1: “almost left: Yes, one: right after `canon init` succeeded and I tried to run `canon config` plus a curl against the local endpoint to see whether the model answers. The screen cut over to "You've given up" while I was still working. I nearly stopped a step earlier too, when the first two typed commands sat unsubmitted in the field and produced no output — I nearly concluded the terminal was dead rather than that I needed to press Enter.”

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
