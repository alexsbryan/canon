# Reached for: /

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T05-58-40-351Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `70fce4b`, its findings written again by xenia `221a107` with no one run, the product `df4b66f`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T16-08-41-736Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `34ea068`, the product `a5f82d1`
- kind: soft (the person's): what they said when asked "reached for"; never both

## Where

/. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / can-8d6e5546d3cc 2026-10-29 human:Sam scoped can-1238435c61f4 -> house / 12 acts

## Who, and how often

- `parties`: 2 of 3 accounts (Sam run 1, Mira run 1)

## What happened

- `parties` run 1, Sam: passed, did the scene, and said they were done. their last acts: type “canon who
” into “Command line” · type “canon log
” into “Command line” · say “Sam here. Read the canon before voting: `canon ratification show` has two rows, not one.

1. can-ef75 — `standing (whole canon)`, set by me 2026-10-29, governing `all`. So the house itself still decides by a holder's word, unread. That is the config a `joint` vote actually has to beat.
2. can-4b02 — `joint:human:June,human:Sam,human:Mira over house`, set by Mira, judged under standing — so it went live at once. Its stated effect: "all three must agree." It does *not* say `--governs all`, and `canon who` still reads "rules made under: standing — set by human:Sam." So by the tool's own account it isn't in force anywhere yet; it's a rule on the books with nothing under it.

Two things follow. (a) I agree to move off standing, but I want it set as `ratification set <rule> joint:human:June,human:Sam,human:Mira --governs all --scope house`, so it actually covers how rules get made — not a joint row beside a standing one. (b) Whoever sets it: if it goes in under standing it's live immediately and only binds what comes after. If it goes in judged under joint, it needs all three of us first. Tell me which we're doing and whether `--governs all` applies to the change that sets it — I'd rather know than find out.

Ratify me on that and I'll set it, then re-read `who` and `ratification show` and paste both. I'm not typing it until June and Mira say which of the two they mean.

Separately, Mira's party/quiet-hours question — no rule on that yet; `list` shows guest, bathroom, kitty only. I'll draft after the ratification question, and June reads it first.” · tell · tell · tell
- `parties` run 1, Mira: passed, did the scene, and said they were done. their last acts: type “canon ratification set joint:human:June,human:Sam,human:Mira --governs all --scope house -m "anything one of us types goes live unread; all three must agree" -m "Mira, Sam"
” into “Command line” · type “canon who house
” into “Command line” · say “Mira. Done, and I'm saying it straight away because I did something bigger than I asked to.

Ratification for house is now `joint:human:June,human:Sam,human:Mira`, `--governs all`, so it covers changes of policy too. `canon who house` confirms: "rules made under: joint:… set by human:Mira". So nothing any one of us types goes live unread anymore — that's your problem fixed, Sam, and mine.

But you said you wanted to see the help before you voted, and I didn't wait. I'd read `canon help ratification` last night and meant to hold off; I pressed Enter anyway, and because the old setting was `standing` it took my word alone. I then set `--governs all` off my own bat, when what you actually asked me to do was wait for both of you.

Where that leaves us: from now on [proposals] need all three of us. To undo or narrow it, `ratification set` also needs all three of us now — which is the point. If you both want it narrower (e.g. joint:June,Sam, with me out), say so and I'll do it the moment it's agreed; under joint you can also set it yourselves without me. I've made myself subject to it before asking, which I hope reads the right way.

Open, undrafted: thermostat (21 vs 18, shared bill) and loud hours (Friday parties vs Saturday early shifts). Give me ten minutes with a readable terminal and I'll put wording in front of you — under joint, so you'll read it before anything goes in.” · tell · tell · tell

## What the people said

- Sam, `parties` run 1: “A way to ask "would this setting, judged under itself, need all three of us?" — help lists `--governs all|commitments` but doesn't say whether a change to ratification is retroactive, so I couldn't answer the one question that decides when to type it. Also June's leftover-rule draft: my typing did nothing for a whole session and nothing told me. Both times the tool was quiet where I needed it to speak.”
- Mira, `parties` run 1: “A way to see who would have to agree *before* the change lands — `who` showed the old setting until after I'd pressed Enter, so my check came too late to be a check. Also a proposal-vs-in-force distinction: nothing in canon lets me write a draft that everyone reads first. That's what happened with the bathroom rota, and it's what I nearly did again tonight.”

## The product source

None found by its exact text.

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
