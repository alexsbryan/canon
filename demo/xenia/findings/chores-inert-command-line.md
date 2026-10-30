# A tap changed nothing: /, “Command line”

- status: open, not seen since the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T16-08-41-736Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `34ea068`, the product `a5f82d1`
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T16-08-41-736Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `34ea068`, the product `a5f82d1`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T16-08-41-736Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `34ea068`, the product `a5f82d1`
- kind: hard (the instrument's): `inert`; never both

## Where

/, the element “Command line”. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / pool <scope> [--at <date>] whose turn it is. `who` is authority, / `pool` is appropriation

## Who, and how often

- `chores`: 1 of 3 accounts (Mira run 1)

## What happened

- `chores` run 1, Mira: failed, ran out of patience. inert {"tap":"Command line"}; their last acts: type “ssh-keygen -t ed25519 -f ~/.ssh/mira -N "" -C "mira"
” into “Command line” · look · wait · type “y
” into “Command line” · type “ls ~/.ssh
” into “Command line” · tap “Command line”
- `chores` run 1, Mira: failed, ran out of patience. inert {"tap":"Command line"}; their last acts: wait · type “y
” into “Command line” · type “ls ~/.ssh
” into “Command line” · tap “Command line” · scroll · tap “Command line”
- `chores` run 1, Mira: failed, ran out of patience. inert {"tap":"Command line"}; their last acts: say “Mira here. My key question is answered: `human:Mira` is now bound to key fingerprint SHA256:gQblUx/uYTAP4o5kWHp3soIl1V9OiFUVW5sBtX4lB4M (read it back to me in person, not from a file). Still no grant for me.

Sam — please `canon grant human:Mira house` (or a narrower scope: noise). June asked to agree first; reading `canon who`: house is held by June and Sam under `standing` — a holder's word is a rule at once. If you want that grant to wait for both of you, say so and I'll wait.

Noise: I'll have a draft rule for you to read before anything goes in.” · say “June — I just did it, so here are the exact lines, in order:

1. `mkdir -p ~/.ssh && ssh-keygen -t ed25519 -f ~/.ssh/mira -N "" -C mira` (mine was `mira`; use `june`)
2. It prints a fingerprint like SHA256:... — keep that.
3. `canon identify ~/.ssh/june.pub` — it replies "human:June is bound to SHA256:..." Check the name it says is exactly `human:June`.
4. Then read that fingerprint out in the chat so the rest of us can compare it to our own terminals. `canon identify` warned me: anyone who can edit the key file could bind the name, so comparing is the only way to know it's you.

Two caveats: `identify` has no undo — a name binds once, so if the name comes back wrong, stop and ask before running more. And a plain key can sign without asking you; `ssh-keygen -t ed25519-sk` (a hardware key) requires your touch, which is stricter than mine.

Bathroom: yes, I'll draft a rota as `canon add` for you both to read. I don't hold house, so it needs a holder's approval anyway.” · type “canon add "Bathroom cleaning goes by rota: each of us takes a week in turn (June, Sam, Mira), Monday to Sunday; the person whose week it is says so in the house chat by Sunday evening if they cannot do it and we swap. A week nobody covers is the next person's." --scope house -m "drafted by Mira at June's request; three weeks uncleaned; to be read by June and Sam before it counts"
” into “Command line” · look · type “canon who house; echo ---; canon log | tail -5; echo ---; canon why can-acf07050abf3
” into “Command line” · tap “Command line”

## What the people said

- Mira, `chores` run 1: “reached for: Any help text that tells me in plain words, before I press Enter, whether an `add` counts now or waits for a read. Nothing in the help said a holder's own word makes it a rule at once — I only learned that from `canon who` afterwards.”
- Mira, `chores` run 1: “reached for: Any help text that tells me in plain words, before I press Enter, whether an `add` counts now or waits for a read. Nothing in the help said a holder's own word makes it a rule at once — I only learned that from `canon who` afterwards.”
- Mira, `chores` run 1: “reached for: Any help text that tells me in plain words, before I press Enter, whether an `add` counts now or waits for a read. Nothing in the help said a holder's own word makes it a rule at once — I only learned that from `canon who` afterwards.”

## The product source

- `demo/xenia/README.md:146` holds “Command line”
- `demo/xenia/README.md:243` holds “Command line”
- `demo/xenia/README.md:249` holds “Command line”
- `crates/canon-cli/src/sources/tests.rs:441` holds “Ctrl-C”
- `crates/canon-cli/src/ui.rs:145` holds “Ctrl-C”
- `demo/xenia/README.md:227` holds “Ctrl-C”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
