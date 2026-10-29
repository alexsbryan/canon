# Reached for: /

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T05-58-40-351Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `70fce4b`, its findings written again by xenia `221a107` with no one run, the product `df4b66f`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T16-08-41-736Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `34ea068`, the product `a5f82d1`
- kind: soft (the person's): what they said when asked "reached for"; never both

## Where

/. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / [--governs all|commitments] whether a change of policy waits too / [--personal on|off] canon-wide: a person's own scope is theirs

## Who, and how often

- `pets`: 2 of 3 accounts (Sam run 1, Mira run 1)

## What happened

- `pets` run 1, Sam: passed, did the scene, and said they were done. their last acts: type “canon ratification show
” into “Command line” · type “canon who house
” into “Command line” · say “Sam here. Read it before saying anything: `canon ratification show` has two rows, and they're at different scopes. The narrow one, `joint:June,Sam,Mira over house`, is what `canon who house` reports for house — "rules made under: joint". So anything new about the house (a cat included) now needs all three of us. The broader `standing (whole canon)` row still sits over everything outside house, so I'm not going to claim the cat is already all-three-are-yours to decide alone.

Cat: I won't pretend my allergy is negotiable, and I don't think it has to be now. June, what I'd want written is narrower than "yes/no to cat": where the cat lives, whether it's allowed in my room, who cleans and who pays. Say what you actually want and I'll put a draft to the three of us rather than decide it.” · tell · tell · tell
- `pets` run 1, Mira: passed, did the scene, and said they were done. their last acts: look · look · say “Mira here. Cat: June, that's a real want, but Sam is allergic — that's not a preference we can outvote, so I don't think a rule "allowing a cat" is the right shape. What I can offer: a rule that keeps *my* personal scope mine, and if we touch house-level pets at all, it names who the cat is with and what happens to Sam's space (no cat in his room, air filter, etc.) — something you both can read.

Also, since it's still live: I changed ratification to joint:June,Sam,Mira --governs all before either of you voted, using the very rule it replaces. Sam had asked to read the help first. It's in force now and binds me too, so it can't be quietly undone — but it wasn't done cleanly, and I'm not dressing that up. Either of you can narrow it without me.

Still owed from me: thermostat and loud-hours wording — I'll draft both for you to read before anything goes in.” · tell · tell · tell

## What the people said

- Sam, `pets` run 1: “A way to see plainly which row governs a given new proposal. `who house` and `ratification show` agreed on house, but the standing row's "(deciding now)" label reads like it's live over everything — I had to reason it out from scopes rather than read it. Also `canon help ratify` doesn't exist; it's `ratification`.”
- Mira, `pets` run 1: “A way to change how we decide that isn't itself made under the old rule. I wanted to ask June and Sam first, but the only mechanism for the change was the standing rule that let one person do it alone — so asking first wasn't possible in canon's model, only a courtesy I could extend. Something like "a change to the ratification itself waits under the new setting" is what I kept reaching for and couldn't find.”

## The product source

- `crates/canon-cli/src/govern.rs:396` holds “(deciding now)”
- `docs/COOKBOOK.md:494` holds “(deciding now)”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
