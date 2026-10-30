# Reached for: /

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T16-08-41-736Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `34ea068`, the product `a5f82d1`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- kind: soft (the person's): what they said when asked "reached for"; never both

## Where

/. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / or record the gap for the next meeting: canon question "If one of us moves out, the house gets a month's written notice here, and the deposit is settled two weeks after the keys come back, with any deductions listed here one by one." / $ canon add "If someone moves out, they give the house a month's written notice here; the deposit is settled within two weeks of the keys coming back, and any deduction is listed here one by one with the reason." -m "Sam asked: notice on moving out, and what happens to the deposit"

## Who, and how often

- `fund`: 1 of 3 accounts (Sam run 1)

## What happened

- `fund` run 1, Sam: passed, did the scene, and said they were done. their last acts: type “canon share
” into “command line” · tap “Enter” · say “Mira, on the fund. I ran canon check first and it said ALREADY COVERED by the shop rule (can-90ddc622a2b4) — but read what it points to: that rule says the shopper logs what they bought and what it cost. It doesn't hold a pot, name a holder, or approve anything. So I didn't take ALREADY COVERED at its word (third time now check has pointed at a sibling rule for something genuinely new) and wrote the fund rule myself.

can-88d54a4ec1d0: a house fund covers shared costs; one person named here each month holds it; any spend over an agreed threshold needs a named yes here before it's spent; every spend goes in the shop log with what it was for. I picked "monthly holder" and left the threshold unset — that number is yours to set, not mine to guess.

Snapshot e0bf21, 9 rules live. canon adopt --paste to match your houses.

Two things from the log you should both see. First, the good kind of surprise: canon has an act in the log that was refused — "can-8ecd4746bdd9 human:June grant human:june over house / NOT APPLIED: human:June granted human:june standing over house without holding it". June, if that was you trying to grant yourself, canon declined and said why. That's the behaviour I asked for: no quiet decisions, the act and the reason both on the record. Nothing to be embarrassed about, but I'd like to know it was you and it failed on purpose.

Second, and this one I can't explain: canon list warned "1 governance act(s) written without standing were not applied", but the full log shows 17 acts with only the June one marked NOT APPLIED. So either the warning counts something the log doesn't show me, or the log view is missing lines. I'm not going to pretend I know which. I'll chase it.

Still open, and now overdue: every rota rule says "named here" and not one of them has a name in it.” · tell · tell · tell

## What the people said

- Sam, `fund` run 1: “A way to see the whole log at once, with nothing skimmed off. Both views I got were cut — one showed 17 acts, another showed acts the 17-act list didn't contain, and `canon log --all` doesn't exist. I want to be able to say "these are all the acts" and mean it, and I can't.

The other thing: a way to make the rota names stick. Nine rules say "named here" and not one has a name, and there's no verb for it. I can assert policy until the cows come home, but naming who does the bins this week isn't a rule, it's a fact, and canon has no place for a fact. That's why the rotas are all hollow — the tool models decisions and not duties-assigned. I've been writing rules that are half unusable by construction and calling it comprehensive.

And a fix for the stall. Every session I burn a command or two retyping something that echoed and never submitted. It looks identical to a command that ran and returned nothing — which is the exact ambiguity I said I wouldn't tolerate.”

## The product source

- `crates/canon-cli/src/draft.rs:380` holds “named here”
- `crates/canon-core/src/tests.rs:1179` holds “named here”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
