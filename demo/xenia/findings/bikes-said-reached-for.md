# Reached for: /

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T05-58-40-351Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `70fce4b`, its findings written again by xenia `221a107` with no one run, the product `df4b66f`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- kind: soft (the person's): what they said when asked "reached for"; never both

## Where

/. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / {"id":"can-987d7fccbdc5","v":4,"ts_unix":1793323464,"actor":"human:Sam","op":"ratification","text":"A holder's word is a rule at once. A change of policy waits like any other change, and a person's own scope is theirs.","rul$ checking on http://localhost:9741/v1 (local) / note: `primary` answered by deepseek-flash

## Who, and how often

- `bikes`: 2 of 3 accounts (Sam run 1, June run 1)

## What happened

- `bikes` run 1, Sam: passed, did the scene, and said they were done. their last acts: type “canon add "Bikes live on the rack in the back garden and are not stored in the hall. The drive is kept clear of the second car except by agreement recorded here, naming who and for how long." -m "hall and drive blocked this week; same shape as the other rotas - named here, not the group chat"
” into “Command line” · type “canon share
” into “Command line” · say “Sam, Mira: bikes and the drive are written. can-2374e36e6037 — bikes live on the back-garden rack, not the hall; the drive stays clear of the second car except by agreement recorded here, naming who and for how long. Checked first: THIS NEEDS A NEW RULE, nothing already decided bore on it (deepseek-flash, localhost:9741). Snapshot 5b0930, 6 live — canon adopt --paste to match.

Mira, on the init line: I'm not going to give you one, because there isn't a permanent folder to point it at. Your situations.md and your canon are both in /private/var/folders/.../T/canon-house-.../house, and that folder *is* the house — mine, yours and June's are the same sandbox, with each of our homes inside it. There is no move verb in canon; relocation is lineage only (adopt/upgrade/rebase --onto/share). Re-initing elsewhere would give you a house the rest of us can't reach. The protection is copies: run canon share and keep the adopt line.

And you're right that anything written in /var/folders could be swept — so the answer is that we each hold a snapshot, not that we move.

Two things from my side, honestly: my terminal stalled twice today — commands echoed and didn't run, and once I read a row of buttons at the bottom of the screen as Save/Cancel and nearly pressed it. Neither tap found anything, so nothing happened. What I've learned: the words are the screen; if a command doesn't come back with a $ line and output, it didn't run — retype it and end with Enter. Nothing was lost: all 6 rules are live and 5b0930 has them.” · tell · tell · tell
- `bikes` run 1, June: passed, did the scene, and said they were done. their last acts: look · type “pwd; echo "CANON_DIR=${CANON_DIR-<unset>}"; ls
” into “Command line” · say “June here. Ground check first: pwd on my machine is still /private/var/folders/qf/.../T/canon-house-l5gwOk/house, CANON_DIR is unset (so a re-init elsewhere would work — we're there by upward search, not pinned), and situations.md is the only other thing in the folder. Five rules live, 10 acts in the log: Sam's grants, his standing rule, and the five rules he asserted.

I've been the careful one for ten days and it's left the house with only Sam's rules. So here's my correction: I'm not holding my say any longer waiting for a folder. But I still won't write policy into a folder macOS sweeps — so the two things are separate, and I'm doing the one I can.

Mira: to move it yourself, from the folder where situations.md permanently lives, run:
  unset CANON_DIR   (only if it's set — it isn't on mine)
  canon init --profile house
  canon grant human:june house
  canon grant human:mira house
Then Sam re-grants and you re-write the rules there, or adopt snapshot 978521 with canon adopt --paste if the paths still match. I'd rather Sam confirm that sequence than have me guess at his machine.

Sam: two of yours evaporated. The heating band (19–21, ask here, answer in a day) and the quiet-hours-adjacent Friday-party rule were both proposed and never written — the log proves it. And the bathroom rota still has no name for week 1.

On the hall: bikes and the second car need named places, not hope. My proposal, same shape as bins: each bike slot and the drive belong to a named person each week; if it's not yours, hand it over by name here. That's a proposal, not a rule — I'll wait for Sam's check on it and say yes or no, and I'm not writing it into a temp folder.” · tell · tell · tell

## What the people said

- Sam, `bikes` run 1: “A reliable indication that a command had actually run. The whole day turned on this: I typed a check, the text appeared, and nothing happened — no `$` line, no output, no error. I had no way to tell "canon is down" from "canon is waiting" from "my Enter never landed". canon tells me who decided a rule, which model, at what endpoint, whether it's superseded — more provenance than I get from most tools — but it says nothing about whether the thing I just did happened. In a tool whose promise is "you'll see the act and the reason", a silent no-op is the one failure that undercuts it.

I also tried to tap Save and Cancel on a dialog I thought was in front of me. Neither existed. I'd been reading the thumbnail as words for three days.”
- June, `bikes` run 1: “A "where am I" verb. `canon status` isn't a command — I tried it early and got nothing. I had to use pwd and the shell to find out where our rules actually lived, and that's how we learned they were in /var/folders at all. A tool about house policy ought to be able to tell you plainly which house it's holding.

Also missing: something that shows a proposal that was never written. Sam's heating band and the party rule just vanished. The log can prove an absence if you go looking, but nothing nudged anyone that a discussed rule had gone unwritten. And no easy answer to "move the whole canon somewhere permanent" — it took reading the help to find CANON_DIR, and it turned out unset, so re-init would have been fine all along.

Mostly, though, what wasn't there was a way to disagree without it becoming law. Under "a holder's word is a rule at once," the moment I said what I wanted, it was house policy. That's a frightening thing to give someone who'd rather ask than guess.”

## The product source

- `demo/xenia/findings/findings.json:2232` holds “a holder's word is a rule at once,”
- `demo/xenia/findings/heating-said-reached-for-the-record.md:18` holds “a holder's word is a rule at once,”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
