# A tap changed nothing: the rules, “Command line”

- status: open, not seen since the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T16-08-41-736Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `34ea068`, the product `a5f82d1`
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T16-08-41-736Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `34ea068`, the product `a5f82d1`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T16-08-41-736Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `34ea068`, the product `a5f82d1`
- kind: hard (the instrument's): `inert`; never both

## Where

the rules, the element “Command line”. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / and never exits 1. A contradiction you are carrying on purpose is a / first-class state here, and nothing is ever force-resolved.

## Who, and how often

- `fridge`: 1 of 3 accounts (Sam run 1)

## What happened

- `fridge` run 1, Sam: passed, did the scene, and said they were done. inert {"tap":"Command line"}; their last acts: say “Sam here. June — I'll draft it; you read it before it goes in. Before that I want to check something: `canon help add` says "a rule when its scope ratifies it", but my shop add came back "in force — Sam holds house", which isn't obviously the same thing. Let me read `canon help grant` / the ratification part of `help all` so I can tell you honestly whether adding it puts it in force at once or waits for a vote. Draft text as I'd write it: "Anything labelled with a name belongs to that person and is not eaten by anyone else until they say otherwise. Anything unlabelled in the fridge for more than two days is shared and anyone may eat it."” · type “canon help all 2>&1 | head -60” into “Command line” · wait · type “canon help all 2>&1 | grep -A3 -i ratif” into “Command line” · scroll · tap “Command line”
- `fridge` run 1, Sam: passed, did the scene, and said they were done. inert {"tap":"Command line"}; their last acts: type “canon list 2>&1 | tail -6” into “Command line” · wait · tap “canon list 2>&1 | tail -6” · scroll · type “canon list 2>&1 | tail -8” into “Command line” · tap “Command line”

## What the people said

- Sam, `fridge` run 1: “reached for: A working command line: after a few commands, my typing is echoed but never runs — no new output, no error, no prompt movement. I reached for `canon help all` to read how ratification and grants work and got no answer at all, so I can't honestly tell June whether adding her rule puts it in force immediately. I also wanted a way to see the acts and the reason in the same view — `list` shows rules, `log` shows acts — but I couldn't even run those by the end.”
- Sam, `fridge` run 1: “reached for: A working command line: after a few commands, my typing is echoed but never runs — no new output, no error, no prompt movement. I reached for `canon help all` to read how ratification and grants work and got no answer at all, so I can't honestly tell June whether adding her rule puts it in force immediately. I also wanted a way to see the acts and the reason in the same view — `list` shows rules, `log` shows acts — but I couldn't even run those by the end.”

## The product source

- `demo/xenia/README.md:146` holds “Command line”
- `demo/xenia/README.md:243` holds “Command line”
- `demo/xenia/README.md:249` holds “Command line”
- `crates/canon-cli/src/sources/tests.rs:441` holds “Ctrl-C”
- `crates/canon-cli/src/ui.rs:145` holds “Ctrl-C”
- `demo/xenia/README.md:227` holds “Ctrl-C”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
