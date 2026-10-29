# A tap changed nothing: the rules, “Command line”

- status: open, not seen since the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T02-56-03-707Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `ed1d9ea`, the product `74265cd`
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T02-56-03-707Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `ed1d9ea`, the product `74265cd`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T02-56-03-707Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `ed1d9ea`, the product `74265cd`
- kind: hard (the instrument's): `inert`; never both

## Where

the rules, the element “Command line”. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / $ canon init --profile house / $ canon initialised at /private/var/folders/qf/4ntssn_10d598hw_5rjhx3740000gp/T/canon-house-HyGeiX/house/.canon (house)

## Who, and how often

- `parties`: 1 of 3 accounts (Sam run 1)

## What happened

- `parties` run 1, Sam: passed, did the scene, and said they were done. inert {"tap":"Command line"}; their last acts: say “The quiet rule already covers what carries next door, and 11pm–7am — a Friday party doesn't override it. What's missing is warning: who's working early Saturday, so the party knows its end before it starts. Proposal: Friday loud is allowed, within the quiet rule, but whoever's hosting says in the chat by Thursday what's planned and who's on early shift; if someone's on an early Saturday, the loud part ends at their bedtime, not at 11pm on a technicality.” · say “Then I won't write a new rule either — June's right, quiet hours are the rule and a second one would be two places to argue from. One caveat: "anyone who needs an early night says so that day" is a statement of intent, not a rule, unless someone with an early shift actually does say. Which of us has Saturday early shifts? If it's none of us, I'd rather say that plainly in -m than write a rule for a problem we don't have.” · wait · look · tap “Select” · tap “Command line”

## What the people said

None.

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
