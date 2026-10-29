# Reached for: the rules

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T05-58-40-351Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `70fce4b`, its findings written again by xenia `221a107` with no one run, the product `df4b66f`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T16-08-41-736Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `34ea068`, the product `a5f82d1`
- kind: soft (the person's): what they said when asked "reached for"; never both

## Where

the rules. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / $ can-16df375f7a6b Spare room: it is an office on its claimed days, and anyone else who needs it books it in the house chat the day before, saying which hours. Whoever booked it has it for those hours; whoever wants it outside the office days and hasn't booked gets it only if nobody has booked that time. If two people want the same hours, the one who asked first has it and the other says so in the chat. Office days are fixed and whoever holds them says in the chat when they change. / PROPOSED, not yet a rule — needs approval from human:June, human:Sam

## Who, and how often

- `fund`: 2 of 3 accounts (Sam run 1, Mira run 1)

## What happened

- `fund` run 1, Sam: passed, did the scene, and said they were done; brought back by a message, then did the scene, and said they were done. their last acts: tell · tell · say “Sam here. June — that's the right three questions and your answers are close to mine. Two additions before Mira drafts: (1) "a set figure" should be a number in the text, £30 — and I'd want the figure changeable only the same way as the rest, not by whoever holds the cash; (2) handover needs to say who holds it *now*, by name, or the rule is a shape with no person in it.

One thing to settle before it goes in, not after: I re-read `canon who` and `canon ratification show` just now — "rules made under: standing — set 2026-10-29 by human:Sam", and standing (whole canon) is still marked deciding. That's why the spare room went live before I'd read it. If we want the fund to need all three before it's a rule, say it explicitly and I'll check the scope and ratifier in the log after approving, not just the approve line. Mira — draft it, and put in the text which ratifier it's meant to be judged under.” · tell · tell · tell
- `fund` run 1, Mira: passed, did the scene, and said they were done; brought back by a message, then did the scene, and said they were done. their last acts: type “canon list
” into “Command line” · scroll · say “Checked properly, and I have to take back the alarm I just raised. `canon who house`: June, Sam and me all hold house, and it says rules are made under `joint:human:June,human:Sam,human:Mira`, set by me on 2026-10-29 — so the joint ratification is intact. `canon list`: 6 rules live, house fund among them, and I read to the bottom this time.

The "in force — Mira holds this canon" line was the header for the scope I was writing over, not a licence for one person to decide. I read it as a warning and it wasn't one. Sorry for the false alarm — that's the second time I've trusted a line's shape over what the next command would show.

House fund (`can-7ef01aee69ee`) is in force. Read it and object if the wording's wrong; the figure is ours to agree in the chat, not mine. Still owed: thermostat and loud-hours.” · tell · tell · tell

## What the people said

- Sam, `fund` run 1: “Which row wins. Two ratifications both in force, same date, and `who` says proposals are judged under "default" — nothing tells me the precedence, so I have to hedge in the draft rather than state the rule.”
- Mira, `fund` run 1: “A read of the house fund itself — `canon list` gave me a diff-shaped "There's more you skimmed past" instead of the last entry, and I had to scroll and re-run it to see the rule I'd just written. For something as dry as money wording, I'd like to ask for one rule by id and get just that.

Also: I still can't tell from a single act whether it's a proposal or live. My add printed PROPOSED for the spare room in one breath and "in force" here in another, and the only way I settled it was a second command. That's the exact thing I've had wrong twice.”

## The product source

- `CHANGELOG.md:53` holds “default”
- `Cargo.toml:13` holds “default”
- `Cargo.toml:17` holds “default”
- `demo/xenia/findings/findings.json:16` holds “There's more you skimmed past”
- `demo/xenia/findings/wordy-a-new-canon.md:22` holds “There's more you skimmed past”
- `CHANGELOG.md:182` holds “in force”
- `CHANGELOG.md:294` holds “in force”
- `LICENSE:154` holds “in force”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
