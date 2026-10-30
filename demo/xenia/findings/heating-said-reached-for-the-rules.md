# Reached for: the rules

- status: open, not seen since the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- kind: soft (the person's): what they said when asked "reached for"; never both

## Where

the rules. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / CANON_ENDPOINT override the configured endpoint for one run / CANON_MODEL override the configured model for one run

## Who, and how often

- `heating`: 1 of 3 accounts (June run 1)

## What happened

- `heating` run 1, June: passed, did the scene, and said they were done. their last acts: look · type “pwd; echo "CANON_DIR=${CANON_DIR:-unset}"; canon list
” into “command line” · say “June here. Checked the ground before anything else: pwd is still /private/var/folders/qf/.../T/canon-house-l5gwOk/house, but CANON_DIR is unset on my machine — so nothing is pinned to the temp path; it's the search upward that finds it there. canon list now shows 5 live: guests, quiet hours (so it's live now — good), bathroom rota, recycling, and the shelf/shop rule Mira asked for. Nothing written by me yet.

Sam, on the 21/18 heat: I like the shape of your proposal — a band, ask here, answer within a day, and no change if we can't agree. One thing I'd want written in, because it's my own case: if I'm cold in a room that's inside the band, that's not nothing to me. Can the band be per-room (bedroom lower than living area) rather than one number for the house? And the bill follows shared heating, as you said. Draft it and run check — I'll read the rule it points to and say yes or no here.

Still the one thing I'll ask plainly: where does the house permanently live? Nobody has named it. Once someone does, I'll re-init mine there myself.” · tell · tell · tell

## What the people said

- June, `heating` run 1: “A plain way to see where the canon actually lives, and where its rules would go if the folder were gone. `canon status` doesn't exist, and nothing in the list tells me whether this path is permanent or throwaway — I had to read pwd and the environment notes and work it out myself. Also a way to tell which version of a rule is really live: the quiet-hours rule went missing from `canon list` one day and was back the next, with no explanation anywhere I could look. And I still don't have an answer from anyone on where the house should permanently live — that isn't canon's fault, but it's what I kept reaching for.”

## The product source

None found by its exact text.

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
