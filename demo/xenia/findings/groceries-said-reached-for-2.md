# Reached for: /

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T16-08-41-736Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `34ea068`, the product `a5f82d1`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T18-38-33-385Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `87cf6e3`, the product `cd168e3`
- kind: soft (the person's): what they said when asked "reached for"; never both

## Where

/. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / undo <act-id> [-m "<reason>"] revert an act — yours, or one you hold / question "<text>" record what the canon does not cover

## Who, and how often

- `groceries`: 3 of 3 accounts (Sam run 1, Mira run 1, June run 1)

## What happened

- `groceries` run 1, Sam: passed, did the scene, and said they were done. their last acts: type “grep -n -i 'pmset' ~/dev/canon/demo/xenia/houses/coliving.mjs | head -20” into “Command line” · look · say “Found it. There's a node process running from ~/dev/canon/demo/xenia/houses (coliving.mjs) under a harness called "xenia", started 11:38 this morning. Alongside it are a heap of `canon mcp` and `euphemia mcp` servers on separate terminals, a year's worth. That's the thing putting a box under our command lines — it's a demo harness driving the house as a scenario, and it asks for pmset because the scenario wants to see who touches the password prompt.

So: the prompt is not canon asking for a favour, it's a test asking us to misbehave. Same answer. Mira: the exact line is `canon ratification set unanimous` — that's what I ran, it's in the log as can-204b6a8b7d3b, and I'm not running anything else until you and June both say. The rota is still just a proposal; approving it is `canon approve can-f764e7f1ca8c`. I'm not killing the harness process without you two saying so, but that's my recommendation.” · tell · tell · tell
- `groceries` run 1, Mira: failed, gave up. their last acts: say “Sam, June: there's an unlabelled prompt directly under my command line now — a text field and three buttons. I'm typing nothing into it and pressing nothing; I'll use the command line above it. Still waiting on the exact ratification command from you, Sam, and no rule goes in before that.” · look · give_up · tell · tell · tell
- `groceries` run 1, June: passed, did the scene, and said they were done. their last acts: look · say “Sam — the shelf and the shop are real, but I'd rather we settle the policy question first so whatever we write is the house's, not one person's. In the meantime: two questions the canon may not cover, if nobody objects, are on the shelf. Also my terminal still shows that unlabelled dialog at the bottom, and now I can see its buttons: Cancel, Allow, Don't Ask. I'm not pressing any of them. Sam, your "Cancel or close it" still stands?” · say “Mira — you asked for the exact line yesterday: `canon ratification set unanimous`, and the verb is there in my help. If your yes is now in, say so plainly and Sam can type it. I'll hold, same as before, until all three of us have spoken.” · tell · tell · tell

## What the people said

- Sam, `groceries` run 1: “Two things. A way to see why `check` is answering at all — it says "nothing the house has decided bears on it" with no sign that a debug build under another account is doing the answering. And a way to see what a pending prompt is for before it's in front of you; the dialog never named the process, so we three spent days guessing at something `ps` settled in a minute.”
- Mira, `groceries` run 1: “A way to change the ratification. Sam says set it to unanimous; the help has no such verb and nobody has given me the line, so the one policy that governs everything else is the one we can't touch. I also reached for a shared purchase record for the shelf and shop, and for a noise rule — neither is there, and both wait on the policy. And a label on that prompt.”
- June, `groceries` run 1: “A way to refuse a permission prompt that doesn't say what it's asking for. "Cancel" by name got "nothing here says that" on my screen, and the buttons were too small to read for days — so the one thing I most needed, a labelled way to say no to an unlabelled dialog, wasn't there. Nothing canon could do about it, but it's the gap I kept hitting.”

## The product source

- `crates/canon-cli/src/check.rs:283` holds “nothing the house has decided bears on it”
- `demo/xenia/findings/findings.json:1731` holds “nothing the house has decided bears on it”
- `demo/xenia/findings/leaving-said-reached-for.md:19` holds “nothing the house has decided bears on it”
- `demo/xenia/findings/bikes-said-reached-for.md:26` holds “Cancel”
- `demo/xenia/findings/bikes-said-reached-for.md:47` holds “Cancel”
- `demo/xenia/findings/findings.json:1391` holds “Cancel”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
