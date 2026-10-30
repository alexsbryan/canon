# Reached for: /

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T05-58-40-351Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `70fce4b`, its findings written again by xenia `221a107` with no one run, the product `df4b66f`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T18-38-33-385Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `87cf6e3`, the product `cd168e3`
- kind: soft (the person's): what they said when asked "reached for"; never both

## Where

/. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / undo <act-id> [-m "<reason>"] revert an act — yours, or one you hold / question "<text>" record what the canon does not cover

## Who, and how often

- `chores`: 3 of 3 accounts (Sam run 1, Mira run 1, June run 1)

## What happened

- `chores` run 1, Sam: passed, did the scene, and said they were done. their last acts: type “canon config show
” into “Command line” · type “canon grant human:Mira house
” into “Command line” · say “Mira — you now hold house (granted, no end, under my name). That was mine to give and I've given it, so you can approve or object in your own right. The rota is can-f764e7f1ca8c and it's waiting on you and June.
Two numbers I want on the record: `canon config show` says endpoint = http://localhost:9741/v1, model = primary. That's what I couldn't explain on Day 3 — it's configured, not just appearing, and it's a debug build by another user account on this machine. I'd rather we either point it somewhere we control or stop trusting `check` until we can. Also my ratification show has standing and unanimous both in force, with unanimous deciding; I've said that before but wanted it where you can find it.” · tell · tell · tell
- `chores` run 1, Mira: passed, did the scene, and said they were done. their last acts: wait · look · say “Prompt's back on my screen — empty box, three unlabelled buttons. Typing nothing, pressing nothing, as before. June: same for you?” · tell · tell · tell
- `chores` run 1, June: passed, gave up; brought back by a message, then did the scene, and said they were done. their last acts: tell · look · say “Mira — agreed on both counts: nothing asserted while Sam's word alone makes a rule, and prose for the rota, not a numbered list. Good news on the command: I read my help again and `ratification set <rule>` does exist — the line is `canon ratification set unanimous`, with optional `--governs all|commitments` to say whether a policy change also waits. So the verb is there; the only question is who types it. Sam said "say yes and I'll run it," so I'd rather he run it — but it shouldn't run until your yes is in the chat, not just mine. And my terminal still has an unlabelled dialog at the bottom; I'm leaving it alone.” · tell · tell · tell

## What the people said

- Sam, `chores` run 1: “Two things. A way to see or refuse the endpoint from inside canon — `config show` reports localhost:9741 and offers no explanation of where that came from or who set it, and `check` answers "because" from a model on that port without saying the source in the log. I wanted the act and the reason for the configuration itself, and there isn't one. Second, `grant` and `approve`/`object` aren't in top-level `--help` at all; I found them by running them. A tool that keeps its verbs hidden from its own help is asking me to trust a happy path I can't read.”
- Mira, `chores` run 1: “A way to change the ratification that I can actually see in the help — Sam's line isn't a verb canon knows as far as I can tell, and I won't paste what I can't read. Also a history that's just ours: the log is four acts, all Sam's, so "why did it change without asking" has one answer — nobody else could. And the situations list in prose I could edit, rather than fourteen numbered lines canon reads as zero.”
- June, `chores` run 1: “A way to see what this dialog is asking for, and a button name I could read well enough to refuse it. Same as the last two days. Beyond that, just Sam — to say which word cancels, and to run the ratification line once Mira has said yes.”

## The product source

- `CHANGELOG.md:102` holds “because”
- `CHANGELOG.md:118` holds “because”
- `CHANGELOG.md:281` holds “because”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
