# Reached for: the rules

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T16-08-41-736Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `34ea068`, the product `a5f82d1`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T16-08-41-736Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `34ea068`, the product `a5f82d1`
- kind: soft (the person's): what they said when asked "reached for"; never both

## Where

the rules. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / --resume [run] finish a review, no model call — every run with / candidates left, oldest first, or the one named

## Who, and how often

- `parties`: 1 of 3 accounts (June run 1)

## What happened

- `parties` run 1, June: passed, did the scene, and said they were done. their last acts: type “canon who house
” into “command line” · type “canon list
” into “command line” · say “June here. Mira — loud hours: yes, let's have it written down, and let's draft it so we all read it first. My plain reading: quiet before 9am and after 11pm on weekdays, later on Friday and Saturday (say 1am), and if you need an early Saturday, say so in the chat the day before and the person with the party keeps it indoors. But I'm not writing it in on my own — someone draft it, I'll read it.

Now the thing I checked, because it matters more: `canon who house` says rules are made under joint:human:June,human:Sam,human:Mira — all three of us must agree. Good, that's what I wanted. But `canon ratification show` still lists the standing entry as "deciding now" too, and proposals are "judged under: default". Mira set joint under standing, so it went in alone — which is exactly the problem we were trying to fix. I'm not undoing it; I agree with joint. But somebody who knows the tools better than I do should confirm the standing entry is properly retired, or we'll keep guessing which rule counts. I won't set ratification myself.

My key is still unbound — anything signed human:June could be anyone who can write the file. Sam, I still want you to walk me through identify. I won't run it blind.” · tell · tell · tell

## What the people said

- June, `parties` run 1: “A plain way to see what my key status is without running identify. `canon review` says the true, alarming thing — "anyone who can write this file could decide as you" — but there's no verb that just shows me the state and what binding a key would take. I keep asking Sam to walk me through it because the help tells me what identify does, not what it does to me, and I won't run something blind that signs in my name.

And the one I keep reaching for and not getting: a way to tell which of two ratification entries wins when both say "deciding now." The show command lists them both and leaves me to work it out. For someone careful like me, that's the missing piece — not a verdict on the thermostat, just "this one governs, the other is superseded." I'd rather it be boring and explicit than something I have to infer from a word like "narrowest."

What I didn't reach for, or refused to: `ratification set` and `grant`. Changing how the house decides is not mine to do alone, and I said so rather than quietly doing it.”

## The product source

None found by its exact text.

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
