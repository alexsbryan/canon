# Too much: the terminal

- status: open, not seen since the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T18-55-28-602Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `fc36366`, the product `963e1cd`
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T16-03-37-837Z`, 1 run(s) by claude-sonnet-5-5, as claude -p, xenia `d7600a5`, the product `0f15ca8`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T18-55-28-602Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `fc36366`, the product `963e1cd`
- kind: soft (the person's): what they said when asked "too much"; never both

## Where

the terminal. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / $ / $

## Who, and how often

- `june-first-hour`: 1 of 1 accounts (June run 1)
- `june-join`: 1 of 1 accounts (June run 1)
- `mira-join`: 1 of 1 accounts (Mira run 1)
- `sam-join`: 1 of 1 accounts (Sam run 1)

## What happened

- `june-first-hour` run 1, June: could-not-judge, they spoke as someone acting for the person, not as the person: If you want me to go ahead with `init --profile house` and `add "Quiet hours are 10pm to 7am."`, just say so, and say whether that rule is settled. (“if you want me”). their last acts: wait · tell · tell · tell · tell · tell
- `june-join` run 1, June: passed, reached their goal. their last acts: tap “Enter” · tell · tell · tell · tell · tell
- `mira-join` run 1, Mira: passed, reached their goal. their last acts: say “Done, Sam. I started my own copy in ~/house (house profile), then adopted your block with `canon adopt --paste < ~/share.txt`. It reports: adopted share-src@690c38 — 1 commitment, carrying where it came from.

So in my copy, live now:
  can-c710f090d63b  Guests may stay up to three nights.  (from can-00a3dc1f0da8, via the adopt act can-6f9aa49a5f48)

Two things I noticed and want to flag:
1. My copy only has the one guest rule as a live rule. Your ratification ("A holder's word is a rule at once...") came across only as my own init act (can-2d711ca741cc), not as something adopted from you — so if the ratification matters to the house, say so and I'll look at how it should travel.
2. The adopt act records where the rule came from but no reason why. For "why did this change without asking Sam", the tool for it is `canon why <id>` — I didn't get to run it before my terminal session ended, so I can't yet show you what the trail looks like. I'll do that next time.” · tell · tell · tell · tell · tell
- `sam-join` run 1, Sam: passed, reached their goal. their last acts: say “Done, Mira. I started my own canon here (profile: house) and read the block in from ~/share.txt with `canon adopt --paste`.

Live now: 1 rule — "Guests may stay up to three nights." (can-15c44d7a13ff)

And because I don't like quiet decisions, here's the trail behind it, so you can show the others:
- canon log (oldest first): adopt share-src@684205 → assert "Guests may stay up to three nights." → ratify standing
- canon why can-15c44d7a13ff: written 2026-10-28 by Sam, inherited from upstream can-87ab46c85dfd, in force.

Two things I noticed, in the interest of you being able to trust the happy path:
1. `canon adopt` isn't listed in the top-level `canon --help`; I had to ask for `canon adopt --help` directly. The share file told me to run it, but the help didn't.
2. `adopt` has no `--dry-run`, so there's no preview — it applies as it arrives. I passed `-m` to record why ("adopting the house canon as shared in the house chat by Mira; single live rule, nothing retracted"). Worth knowing that the rationale didn't appear in `canon log` or `canon why` output — I can dig into whether it's stored but not displayed.

The share header says snapshot 2026-10-28, generation 684205, and canon says a newer `canon share` block can be piped into `canon upgrade --paste` later. So when the house canon moves on, I can pull it in and you'll see each act.” · tell · tell · tell · tell · tell

## What the people said

- June, `june-first-hour` run 1: “Not too much, but two points gave me pause. Piping a downloaded script straight into a shell unsettled me — I read it first. And `canon add "Quiet hours are 10pm to 7am."` meant writing a rule into the house's record; I wasn't willing to do that without knowing it was actually decided and that this was the right folder.”
- June, `june-join` run 1: “Only one small thing was more than I'd take on by myself: setting up a model endpoint for `draft`. That's a change to the house's setup and I didn't know which server to point at, so I left it for Sam rather than guess. Everything else stayed within starting my own copy and adding the one rule.”
- Mira, `mira-join` run 1: “Two small things were more than needed: the Canon block's closing hint pointed to `adopt --paste` but the top-level help didn't list `adopt` at all, so I had to probe for it. And in this shell, typed commands sit at the prompt until I tap Enter — easy to pile up a queue of commands without meaning to.”
- Sam, `sam-join` run 1: “The help-all was confusing to me — I read its closing line ("who decides what, how you decide, what has gone stale...") as a table of contents expecting separate sections, and instead got configuration/flags/environment/personal-profile content. That mismatch of expectation is the only thing that was too much; the rest was clear.”

## The product source

- `README.md:39` holds “Quiet hours are 10pm to 7am.”
- `crates/canon-cli/src/review/tests.rs:45` holds “Quiet hours are 10pm to 7am.”
- `crates/canon-cli/src/review/tests.rs:107` holds “Quiet hours are 10pm to 7am.”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/scenes --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon --debug
```
