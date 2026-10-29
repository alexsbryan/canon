# A tap changed nothing: the rules, “Command line”

- status: open, not seen since the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T02-56-03-707Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `ed1d9ea`, the product `74265cd`
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T02-56-03-707Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `ed1d9ea`, the product `74265cd`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T02-56-03-707Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `ed1d9ea`, the product `74265cd`
- kind: hard (the instrument's): `inert`; never both

## Where

the rules, the element “Command line”. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / $ canon init --profile house / $ canon initialised at /private/var/folders/qf/4ntssn_10d598hw_5rjhx3740000gp/T/canon-house-HyGeiX/house/.canon (house)

## Who, and how often

- `pets`: 1 of 3 accounts (Sam run 1)

## What happened

- `pets` run 1, Sam: passed, did the scene, and said they were done. inert {"tap":"Command line"}; their last acts: tell · tell · say “Then I'll decide the part that's mine: no cat in the house while I live here. Not because June's wrong to want one, but because an allergy isn't a preference I can trade — it's my body reacting in a room I have to sleep in, and no rota or chat message fixes that. I'm not asking June to drop the wish.” · say “Honest answer: a cat kept out of my room, off the soft furnishings, with a filter, sounds survivable on paper and awful in practice — dander travels on clothes and on whoever held the cat, and I'm the one who'd be breathing it and second-guessing whether it's fair to complain. I won't run a trial whose failure lands on my lungs. So it's a no from the allergy. That's not me vetoing June's wish, it's me declining to be the subject of the experiment.” · type “canon add "No cat in the house while Sam lives here." -m "Sam's allergy: a medical fact, not a preference to trade, so Sam decides this one himself rather than putting it to a vote. June wants a cat and the wish stands; Mira declined to decide, rightly, because one person's air is not hers to overrule. Considered and rejected: keep the cat out of Sam's room, off the soft furnishings, an air filter, or a trial — dander travels and a failed trial lands on Sam's lungs. Can be revisited if Sam's allergy changes or Sam moves out." --cite "house chat, this thread; situations.md item 9"” into “Command line” · tap “Command line”

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
