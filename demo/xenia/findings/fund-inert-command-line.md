# A tap changed nothing: /, “Command line”

- status: open, not seen since the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T02-56-03-707Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `ed1d9ea`, the product `74265cd`
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T02-56-03-707Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `ed1d9ea`, the product `74265cd`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T02-56-03-707Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `ed1d9ea`, the product `74265cd`
- kind: hard (the instrument's): `inert`; never both

## Where

/, the element “Command line”. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / $ / $

## Who, and how often

- `fund`: 1 of 3 accounts (Sam run 1)

## What happened

- `fund` run 1, Sam: failed, gave up. inert {"tap":"Command line"}; their last acts: tell · say “Before what it covers, whether there is a fund at all — I don't know that anyone's holding anything. If there isn't, the rule should say so and not describe a pot we'll never fill. On the rest, my shape: it covers the house, not personal shopping (the receipt rule already handles that); someone named holds it and posts the balance monthly; spends under a small amount by whoever's shopping, anything bigger agreed by all three first. Mira, June — is there a fund, and what number?” · say “Mira's shape is right and I'll write it: shared consumables and repairs to shared things, not anyone's own shopping; held by Mira, balance posted monthly in the chat; under £30 buy and post the receipt, £30 or over gets a yes in the chat first. June — before it goes in: is £30 the right line, and does anyone object to Mira holding it? I'd rather have your yes than a rule I can point to that only two of us agreed.” · wait · look · tap “Command line”

## What the people said

- Sam, `fund` run 1: “gave up: June hasn't answered on the £30 line or on Mira holding the fund, so I've stopped short of writing it — I said I'd rather have her yes than a rule only two of us agreed, and that still holds. Eleven rules are live and verified. The real limit on this whole task is that the house's facts live outside canon: the tenancy, the landlord's number, the collection day, whether a bike rack exists, which three days the office is in use, and now whether there's a fund at all. I can write what the house agrees; I can't verify what it doesn't know.”

## The product source

- `demo/xenia/README.md:143` holds “Command line”
- `demo/xenia/README.md:203` holds “Command line”
- `demo/xenia/README.md:209` holds “Command line”
- `crates/canon-cli/src/sources/tests.rs:441` holds “Ctrl-C”
- `crates/canon-cli/src/ui.rs:145` holds “Ctrl-C”
- `demo/xenia/README.md:187` holds “Ctrl-C”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
