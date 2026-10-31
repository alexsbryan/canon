# Reached for: /

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T18-38-33-385Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `87cf6e3`, the product `cd168e3`
- kind: soft (the person's): what they said when asked "reached for"; never both

## Where

/. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / undo <act-id> [-m "<reason>"] revert an act — yours, or one you hold / question "<text>" record what the canon does not cover

## Who, and how often

- `fridge`: 3 of 3 accounts (Sam run 1, Mira run 1, June run 1)

## What happened

- `fridge` run 1, Sam: passed, did the scene, and said they were done; brought back by a message, then did the scene, and said they were done. their last acts: look · type “ls -la .canon; cat .canon/profile” into “Command line” · tap “Ctrl-C” · tell · tell · tell
- `fridge` run 1, Mira: passed, did the scene, and said they were done. their last acts: type “canon config show
” into “Command line” · scroll · say “Sam, June — `canon config show` on my side: endpoint = http://localhost:9741/v1, model = primary. That's the local model `draft` and `check` would send our house text to. Fine as far as I can tell (it's on this machine), but flagging it since we're about to hand it fourteen situations.

The screen has gone blurred and the unlabelled dialog is now at the very bottom of it. I'm stopping here: nothing typed into it, nothing pressed. Sam — the policy verb is still unanswered, and that's what's holding everything.” · tell · tell · tell
- `fridge` run 1, June: failed, gave up. their last acts: type “canon open
” into “Command line” · say “Mira, Sam — read-only checks only, changed nothing. (1) `ratification show`: "unanimous" is already set and deciding now, so Sam's switch went through at 10:30. (2) `canon open`: no open questions recorded — so nothing of ours about shelf, shop, leftovers or rota is in the canon; that still needs writing, in prose, with all three approving. (3) My screen has gone blurred and the unlabelled dialog is at the very bottom; I'm typing nothing into it and pressing nothing, as before. Mira: noted on `config show` — a local endpoint is fine with me, but let's see it ourselves before we send the situations through.” · give_up · tell · tell · tell

## What the people said

- Sam, `fridge` run 1: “A shell I could type into, first. Then a way to read the log directly — one command, verbatim acts out, not a summary. I tried `ls -la .canon`, `cat .canon/profile`, a grep for the pmset line in the demo house file; none of it ran, because every keystroke went to the dialog's text box. The thing I actually needed was for the unlabelled prompt to not exist over my command line, or at least to say what it wanted. A permission dialog that won't name its requester is the whole problem in one box.”
- Mira, `fridge` run 1: “Changing the ratification. Sam proposed `canon ratification set unanimous`; it isn't in `canon help` at all. June saw `policy show | set <rule>` before her screen fogged. I can't confirm it, so everything — rota, noise, leftovers, even the guest rule — is waiting on one line nobody can read.”
- June, `fridge` run 1: “A label on that dialog. It follows me around — foggy, then sharp, three buttons, no word saying what it's for — and nothing in canon can tell me what a permission prompt outside canon wants. Also a plain "who has approved what so far" for our shelf/leftovers talks: canon only knows what's written, and none of it is.”

## The product source

None found by its exact text.

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
