# Reached for: the rules

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T16-08-41-736Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `34ea068`, the product `a5f82d1`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- kind: soft (the person's): what they said when asked "reached for"; never both

## Where

the rules. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / CANON_API_KEY sent as a bearer token, for an endpoint that needs one / ON THE PERSONAL PROFILE

## Who, and how often

- `fridge`: 1 of 3 accounts (June run 1)

## What happened

- `fridge` run 1, June: passed, did the scene, and said they were done. their last acts: scroll · type “canon list
” into “Command” · say “June here. About the leftovers — I don't want to write a rule with no home. pwd is still /private/var/folders/.../T/canon-house-l5gwOk/house, the throwaway folder, and the help says CANON_DIR pins canon to "this canon instead of searching upward" — so re-init alone may not move it; whoever runs it needs to unset CANON_DIR first.

Meanwhile I'd like a plain answer before we write anything: is there a shared shelf and what's on it, or is everything labelled? Once that's said, I'll put it in words — I'd suggest: anything unlabelled on the shared shelf is fair for anyone, anything with a name on it is that person's. I'll write it the moment our rules sit somewhere permanent. Sam, still waiting on you for the permanent path.” · tell · tell · tell

## What the people said

- June, `fridge` run 1: “A field to type commands into. The terminal shows "Command line / Enter / Ctrl-C / Ctrl-D" at the top but I couldn't find a place to type — I tried the label, "Enter", even "Command", and nothing took. I only got `canon who` and `canon list` through by typing into "Command" — and both were guesses at the right name. Also: no clear "where does this canon actually live / can I move it" command. pwd told me the folder was wrong but not how to fix it, and the leftovers question had nowhere to go but talk.”

## The product source

- `demo/xenia/findings/bikes-inert-command-line.md:10` holds “Command line / Enter / Ctrl-C / Ctrl-D”
- `demo/xenia/findings/bikes-said-reached-for-the-rules.md:10` holds “Command line / Enter / Ctrl-C / Ctrl-D”
- `demo/xenia/findings/bikes-said-reached-for.md:10` holds “Command line / Enter / Ctrl-C / Ctrl-D”
- `CLAUDE.md:18` holds “, even ”
- `crates/canon-cli/src/draft.rs:1258` holds “, even ”
- `crates/canon-core/src/ratify.rs:958` holds “, even ”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
