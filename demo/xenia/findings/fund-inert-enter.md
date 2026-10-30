# A tap changed nothing: /, “Enter”

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- kind: hard (the instrument's): `inert`; never both

## Where

/, the element “Enter”. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / or record the gap for the next meeting: canon question "If one of us moves out, the house gets a month's written notice here, and the deposit is settled two weeks after the keys come back, with any deductions listed here one by one." / $ canon add "If someone moves out, they give the house a month's written notice here; the deposit is settled within two weeks of the keys coming back, and any deduction is listed here one by one with the reason." -m "Sam asked: notice on moving out, and what happens to the deposit"

## Who, and how often

- `fund`: 1 of 3 accounts (Sam run 1)

## What happened

- `fund` run 1, Sam: passed, did the scene, and said they were done. inert {"tap":"Enter"}; their last acts: look · type “canon log --all
” into “command line” · type “canon add "A house fund covers shared costs; one person is named here each month to hold it. Any spend over the agreed threshold needs a named yes here before it is spent, and every spend is written in the shop log with what it was for." -m "Mira asked who holds the fund and how we approve a spend; check called it covered by the shop rule, which does not hold a fund or approve anything"
” into “command line” · look · type “canon share
” into “command line” · tap “Enter”

## What the people said

- Sam, `fund` run 1: “reached for: A way to see the whole log at once, with nothing skimmed off. Both views I got were cut — one showed 17 acts, another showed acts the 17-act list didn't contain, and `canon log --all` doesn't exist. I want to be able to say "these are all the acts" and mean it, and I can't.

The other thing: a way to make the rota names stick. Nine rules say "named here" and not one has a name, and there's no verb for it. I can assert policy until the cows come home, but naming who does the bins this week isn't a rule, it's a fact, and canon has no place for a fact. That's why the rotas are all hollow — the tool models decisions and not duties-assigned. I've been writing rules that are half unusable by construction and calling it comprehensive.

And a fix for the stall. Every session I burn a command or two retyping something that echoed and never submitted. It looks identical to a command that ran and returned nothing — which is the exact ambiguity I said I wouldn't tolerate.”

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
