# Reached for: /

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T16-08-41-736Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `34ea068`, the product `a5f82d1`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- kind: soft (the person's): what they said when asked "reached for"; never both

## Where

/. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / noticing." / $ canon add "The bathroom is cleaned on a rota: one person each week, named here, covering shower, sink, toilet and floor; anyone who can't take their week swaps it here, not only in the group chat."

## Who, and how often

- `groceries`: 2 of 3 accounts (Sam run 1, June run 1)

## What happened

- `groceries` run 1, Sam: passed, did the scene, and said they were done. their last acts: look · say “Sam here. Two things at once: the shelf/shop question, and a caveat I owe you about canon check.

The caveat first, because it changes how you should read it: check is not a lookup. It asks deepseek-flash on localhost:9741 for a judgement. Same noise question, twice, days apart, got the same rule both times — but the bathroom question just now was answered by can-8f5fcc8a03dc only because I'd written that rule minutes earlier. It names the model and the reason, which is what I want; just don't treat ALREADY COVERED as canon remembering. The rules and the acts are the memory. The check is advice.

So for the shelf I'm not running check first — I'll write the rule and then check it as a sanity pass. Proposal: whoever does the shop logs it here with the amount and what it was for, so it isn't "nobody knows who paid" again. Tell me if you'd rather a kitty or a rota instead and I'll write that.” · tell · tell · tell
- `groceries` run 1, June: passed, did the scene, and said they were done. their last acts: scroll · type “pwd
” into “Command line” · say “June here. I checked again: pwd is still /private/var/folders/qf/.../T/canon-house-l5gwOk/house — the temp folder. Three rules are live now, but they're sitting somewhere macOS can clear out. I'm still not going to pour the house's decisions into a throwaway directory, so I'm holding my say until it's moved.

Sam, two things back to you:
1. Where's the permanent home? If you tell me plainly, I'll re-init mine there myself — I won't choose it for the house.
2. Your rota rule is live but has no name for week 1, as your own check said. I'll take week 1 if that's alright with everyone — that's me putting my hand up, not writing policy.

On the shelf: that one I don't think canon should settle. "Nobody knows who paid for the last shop" is a conversation, not a rule. What I'd want before any rule is written is just: was there a shared pot, or does someone keep a list? I'd rather ask than invent an answer and have it count as the house's word.” · tell · tell · tell

## What the people said

- Sam, `groceries` run 1: “A move verb. I went looking for where the house lives and whether I could put it somewhere we all agree on; canon has no export or relocation, only lineage — adopt, upgrade, rebase --onto, share. So the answer to "is this temp folder safe?" is "make copies", not "put it elsewhere". That's defensible, but I had to find it by reading help twice and thinking about it, not by asking canon where it is.

Also a way to know that `add` actually wrote. My first one echoed, wrote nothing, no error — I re-ran the identical command and it worked. There's no confirmation except the rule id appearing, and when it doesn't appear I can't tell a dropped keystroke from canon swallowing the write. That's the one I'd chase next.

And a check that cites rather than judges. What I wanted was "this text maps to rule X", fixed and repeatable, not a model's opinion that reshuffles between runs.”
- June, `groceries` run 1: “A way to move the canon, or at least a clear "you are here, and this path is safe / not safe" answer. I reached for `pwd` and got the temp path, but there was nothing to tell me whether the house lived somewhere permanent or whether that path was the real one — I had to reason that /var/folders/.../T means macOS can sweep it. I tried `canon status` on an earlier day and it wasn't a command. There's no "where am I, and is this durable" check. The help mentions CANON_DIR pins the canon "instead of searching upward," which worried me — it may mean re-init alone won't move it, and I had no way to confirm that without guessing. And there's no way for a person to say "I'm holding my say, here's my real answer, keep it until the house has a home" — so everyone's honest caution this week just looks like silence.”

## The product source

None found by its exact text.

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
