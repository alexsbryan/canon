# Reached for: /

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T05-58-40-351Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `70fce4b`, its findings written again by xenia `221a107` with no one run, the product `df4b66f`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- kind: soft (the person's): what they said when asked "reached for"; never both

## Where

/. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / {"id":"can-987d7fccbdc5","v":4,"ts_unix":1793323464,"actor":"human:Sam","op":"ratification","text":"A holder's word is a rule at once. A change of policy waits like any other change, and a person's own scope is theirs.","rul$ checking on http://localhost:9741/v1 (local) / note: `primary` answered by deepseek-flash

## Who, and how often

- `repairs`: 3 of 3 accounts (Sam run 1, Mira run 1, June run 1)

## What happened

- `repairs` run 1, Sam: passed, did the scene, and said they were done. their last acts: type “canon add "Repairs: whoever notices a broken thing in the house reports it here the same day, names who is phoning the landlord, and writes the date they phoned. If the landlord hasn't arranged a fix a week after that date, it gets reported here again with the date, until it is fixed." -m "back window broken; so a repair is owned and dated, not just remembered"
” into “Command line” · type “canon share
” into “Command line” · say “June: broken back window — I checked first and canon said THIS NEEDS A NEW RULE (deepseek-flash, nothing bears on it), so it's written: can-36af7c219cf6. Whoever notices reports it here the same day, names who is phoning the landlord, and writes the date they phoned; if nothing's arranged a week after that date, it gets reported here again with the date, until fixed. Same mechanism as bins/bathroom/shop deliberately — a name and a date, not "someone will ring them".

Snapshot 078500, 7 live: canon adopt --paste if you want your houses to match.

And the rule is only half done: nobody has put a name to the window yet. Landlord number and who's ringing them — say it here and it's covered. If someone would rather I rang, say so and I will.” · tell · tell · tell
- `repairs` run 1, Mira: passed, did the scene, and said they were done. their last acts: type “canon question "Who pays for shared shopping, and how is it recorded so nobody has to remember?"
” into “Command line” · type “canon question "Who calls the landlord when the back window is broken, and what do we do if they don't come?"
” into “Command line” · say “Mira here. June — the back window: that's the same shape as bins and the rota. Name who calls the landlord, and if that person can't, they hand it to someone by name in here — not "someone will". If the landlord doesn't come, we need a date, not a hope: named person chases, and a second date by when we call again.

I didn't write that as a rule yet. I did record it as a gap so it's waiting for us properly: can-dad262c7d169. And I finally recorded the shopping gap the right way too — can-5bc256c7d04f — because we've mentioned it three times now and mentioning isn't recording.

Two things still open: my canon is in the temp folder with Sam's three rules in it, and I still don't have the exact init line for a permanent folder. That's my only blocker — give me the line and I'll move it myself and write the window rule there.” · tell · tell · tell
- `repairs` run 1, June: passed, did the scene, and said they were done. their last acts: type “canon list; canon who” into “Command line” · tap “Enter” · scroll · tell · tell · tell

## What the people said

- Sam, `repairs` run 1: “A way to see what a rule does before it's in force. `canon check` runs on the proposal, but the only other preview is doing the add and reading the id back — so the moment of truth is the write itself. I'd like "show me this rule as it will read in the list, and which rules it sits beside" without committing.

Second, a pointer rather than a paraphrase. check tells me deepseek-flash judged it and gives me a reason, but the reason is prose I have to trust. What I reach for is a list of the rule ids it considered and rejected — that's a lookup I can audit, and it would settle whether the check that said "nothing bears on it" for the bikes rule had actually read the rota rules.

Third, a signed date. The rule I wrote depends on "a week after that date", and the dates live in prose in the chat, not in canon. There's no field I can stamp a repair with — opened, phoned, fixed — so the timeline is my memory again, which is exactly what the rule was supposed to replace.”
- Mira, `repairs` run 1: “A way to say where the house should live. There's init, and there's CANON_DIR, but nothing that just moves a canon to a permanent folder — and I won't hand-roll a copy-and-reinit for rules that are real now. I also wanted a plain "where am I" verb and got pwd and ls instead, which is fine but tells me nothing about whether this canon is the house's canon or a stray copy.”
- June, `repairs` run 1: “A way to see the questions and the answers next to each other. Mira's questions are questions, some of my concerns ended up as rules, and the log's last twenty-five lines show them shuffled together — I couldn't tell at a glance which of the 14 situations still have no answer. `canon voice` claims to show "what someone raised, and what came of it," but I didn't run it, so I may have walked past the thing I wanted. Also: a plain "where does this canon live, and is that path temporary?" — pwd told me, but canon never said it itself.”

## The product source

- `crates/canon-core/src/policy/tests.rs:156` holds “nothing bears on it”
- `crates/canon-core/src/policy/tests.rs:242` holds “nothing bears on it”
- `crates/canon-core/src/policy.rs:461` holds “nothing bears on it”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
