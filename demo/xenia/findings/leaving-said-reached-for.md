# Reached for: /

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T05-58-40-351Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `70fce4b`, its findings written again by xenia `221a107` with no one run, the product `df4b66f`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- kind: soft (the person's): what they said when asked "reached for"; never both

## Where

/. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / can-987d7fccbdc5 2026-10-30 human:Sam ratify standing — A holder's word is a rule at once. A change of policy waits like any other change, and a person's own scope is theirs. / can-05d97cde50ed 2026-10-30 human:Sam grant human:june over house

## Who, and how often

- `leaving`: 3 of 3 accounts (Sam run 1, Mira run 1, June run 1)

## What happened

- `leaving` run 1, Sam: passed, did the scene, and said they were done. their last acts: type “canon share
” into “Command line” · scroll · say “Moving-out rule is in: can-17c37cfea276 — a month's written notice here; deposit settled within two weeks of the keys coming back; any deduction listed here one by one with the reason. Checked first: THIS NEEDS A NEW RULE, decider deepseek-flash on localhost:9741, "nothing the house has decided bears on it" — so read the rule above, not my paraphrase of it.

Snapshot 487d79, 9 live. adopt: canon adopt --paste

Two things for the record. My first two `canon check`s echoed and returned nothing at all — the old stall; retyping with an explicit Enter ran it. And I tried tapping a "Save" button that looked like it was at the bottom of the screen: "There's nothing here that says Save." So that row is still not something I understand; I'm not trusting it and not pressing what I can't identify.

Still no names on any of the rotas — bathroom, bins, shop, office days. Nine rules, every one of them waiting for a person. Also: is one month the notice we actually want? I picked it; change it now if it's wrong.” · tell · tell · tell
- `leaving` run 1, Mira: failed, ran out of patience. their last acts: type “canon share
” into “Command line” · look · type “pwd && ls -a
” into “Command line” · tell · tell · tell
- `leaving` run 1, June: passed, did the scene, and said they were done. their last acts: type “q
” into “Command line” · look · say “June here. Sam — the notice/deposit rule is in the log (can-17c37cfea276): a month's written notice here, deposit settled within two weeks of the keys coming back, deductions listed one by one with reasons. That covers your question, and it seems fair to me. One thing I'd add: "written notice here" should mean a date stated, so "I'll be moving out soon" doesn't count as notice. I'll leave that to whoever writes it.

Two corrections and one honest gap.

Correction one: I've been telling you I'm not a holder. I checked canon who today — I am. human:june over house. The failed grant is still in the log as NOT APPLIED, but I hold house now. So I've had standing for a while and didn't know it.

Correction two, for the record: nine rules are live now, and every one of them is yours, Sam. That's not a complaint about your rules — most are sensible — but it's the truth of where we are.

The gap: on day 12 I said I'd phone the landlord about the back window and write the date here. There is no date in the log. I don't know whether I rang and failed to record it or never rang. I won't make up a date. If someone has rung, say when and I'll write it. If not, I'll ring and write it — but I want the date in the log this time, not just my good intentions.

And the old thing, since it's still true: pwd is still /private/var/folders/…/T/canon-house-l5gwOk/house. CANON_DIR is unset, so we're there by upward search — a re-init in a permanent folder would work. If nobody objects, I'll do mine and give you the exact line for yours.” · tell · tell · tell

## What the people said

- Sam, `leaving` run 1: “Something that reliably runs when I type it. No way to move the house somewhere permanent — only lineage verbs. No way to write a name against a rota without writing a rule. And no way for check to notice I'd already written the same mechanism under a different subject: it said "nothing bears on it" for the office, the pets and the thermostat even where bins/bathroom/shop plainly share a shape.”
- Mira, `leaving` run 1: “A way to walk the canon over to a permanent folder without guessing. I wanted the exact init line, and something that tells me where the canon actually sits — config show gives endpoint and model only, no path. Without that I can't write a rule of my own anywhere but /var/folders, which is the one thing I've been careful not to do for twelve days.”
- June, `leaving` run 1: “Two things.

The first is small and practical: a way to move the canon. I wanted something like "init here instead" — a single verb that relocates the canon to a permanent folder and brings the rules with it. What I found instead was that `init` starts a fresh canon, which is a different thing entirely, and the environment note says CANON_DIR pins canon to "this canon" rather than searching upward. So moving means starting over and losing or re-asserting nine rules. Nobody in the house wanted to be the one to risk that, and so we've sat in /var/folders for two weeks with macOS free to sweep it. The thing I reached for was a way to say "same rules, better home" and it isn't there.

The second is bigger and I'm less sure of it: canon can record a decision but it can't start a conversation. Every rule in there is one person's proposal that stuck. When Sam and I disagreed about the heating — 21 versus 18 — there was no place in canon to put "these two haven't settled this yet." Mira has been using `question` acts, and that's the closest thing, but a question isn't a place where a disagreement sits and gets worked out; it's just a note. I reached for something like "this is open, here are the two positions, here's what would settle it" and canon gave me an empty line where a rule goes.

That's why I kept hovering. Writing a rule means ending the conversation, and I didn't want to end conversations I hadn't had.”

## The product source

- `crates/canon-core/src/policy/tests.rs:156` holds “nothing bears on it”
- `crates/canon-core/src/policy/tests.rs:242` holds “nothing bears on it”
- `crates/canon-core/src/policy.rs:461` holds “nothing bears on it”
- `CHANGELOG.md:42` holds “this canon”
- `CHANGELOG.md:44` holds “this canon”
- `CHANGELOG.md:47` holds “this canon”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
