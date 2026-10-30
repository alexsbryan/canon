# A tap changed nothing: /, “Command line”

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- kind: hard (the instrument's): `inert`; never both

## Where

/, the element “Command line”. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / noticing." / $ canon add "The bathroom is cleaned on a rota: one person each week, named here, covering shower, sink, toilet and floor; anyone who can't take their week swaps it here, not only in the group chat."

## Who, and how often

- `bins`: 1 of 3 accounts (Sam run 1)

## What happened

- `bins` run 1, Sam: passed, did the scene, and said they were done; brought back by a message, then the scene ran out of time while they were acting. inert {"tap":"Command line"}; their last acts: say “Recycling rule written: can-1668914c68d7 — one owner each week, named here; bins out the night before collection; away means handing it to a named person here, not just the group chat. Checked before writing (THIS NEEDS A NEW RULE, deepseek-flash on localhost:9741 — a judgement, not canon recalling), and it's in force now. Snapshot c2962e, 4 live — run `canon adopt --paste` to make your houses match. Recycling still needs an owner for this week: name someone.” · tell · tell · tell · look · tap “Command line”

## What the people said

- Sam, `bins` run 1: “reached for: A way to say "this week's owner is June" as an act, not a new rule. Both rotas require a name recorded here, and canon has no verb for that — so it goes in the chat and the rule stands there unfilled. Same gap for who paid for the last shop: a pattern of acts that the model will paraphrase at me whenever I check, with nothing recorded to point at.

Also a verb to move the canon somewhere durable (none, only lineage), and a reliable `add`: my first one on day 4 echoed and wrote nothing, and I still don't know whether that was the tool or my typing. I re-run whenever add returns no rule id — a habit, not a fix.”

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
