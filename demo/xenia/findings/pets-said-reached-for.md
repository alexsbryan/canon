# Reached for: /

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T05-58-40-351Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `70fce4b`, its findings written again by xenia `221a107` with no one run, the product `df4b66f`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- kind: soft (the person's): what they said when asked "reached for"; never both

## Where

/. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / {"id":"can-987d7fccbdc5","v":4,"ts_unix":1793323464,"actor":"human:Sam","op":"ratification","text":"A holder's word is a rule at once. A change of policy waits like any other change, and a person's own scope is theirs.","rul$ checking on http://localhost:9741/v1 (local) / note: `primary` answered by deepseek-flash

## Who, and how often

- `pets`: 2 of 3 accounts (Sam run 1, Mira run 1)

## What happened

- `pets` run 1, Sam: passed, did the scene, and said they were done. their last acts: look · wait · type “canon list” into “Command line” · tell · tell · tell
- `pets` run 1, Mira: passed, did the scene, and said they were done. their last acts: look · say “Mira here. June — a cat is a Sam question first: he's allergic, so it can't be a house vote that outrules his body. The rule I'd write: no pets in the shared house without the agreement of everyone living here, written in canon, and an allergy counts as a no unless that person says otherwise here. That's also the answer to "Mira doesn't want to decide" — nobody decides alone; it's written down and it's unanimous. I'm not adding it myself; Sam's still the one with the keys, and my canon is in the same temp folder as before.” · tell · tell · tell

## What the people said

- Sam, `pets` run 1: “A way to ask "where do my rules actually live, and can I move them?" — there's no `where am I` verb and no export/move; relocation is only adopt/upgrade/rebase, so the temp-dir worry cost us days before we settled on copies. Also a reliable write: `canon add` echoing and writing nothing, and now check and `canon list` echoing and returning nothing at all, with the screen still showing an older check as if it were current. And help/verb agreement: init's help lays check's line out broken, and the output now disagrees with what the help implies the verbs return.”
- Mira, `pets` run 1: “A permanent home. We've been asking Sam since day one for a path outside /var/folders, and the rules keep going into the temp folder anyway. Also a way to move the existing rules with it, and something in the help that plainly says where the canon lives — I shouldn't have to run pwd to find out.”

## The product source

None found by its exact text.

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
