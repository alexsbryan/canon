# Reached for: /

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T05-58-40-351Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `70fce4b`, its findings written again by xenia `221a107` with no one run, the product `df4b66f`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T18-38-33-385Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `87cf6e3`, the product `cd168e3`
- kind: soft (the person's): what they said when asked "reached for"; never both

## Where

/. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / $ canon ratification show / $ $ can-baba9e80aa06 standing (whole canon)

## Who, and how often

- `debrief`: 3 of 3 accounts (Sam run 1, Mira run 1, June run 1)

## What happened

- `debrief` run 1, Sam: passed, did the scene, and said they were done. their last acts: type “canon who 2>&1; echo exit $?
” into “Command line” · say “What canon is for, as I'd put it: nothing more than a written record of what the house actually agreed, and a way to see who decided what and why. Not a voting machine, not a source of answers.

So: it's for the fact that we keep re-deciding things from memory — three nights vs a fortnight, who owns recycling, whether the rota is in force — and getting different answers. canon's real job here is to say what's in force, who it belongs to, and what's still only proposed. Today it does say that, and it says something I don't like: `canon who` prints "rules made under: unanimous — set by human:Sam" and "proposals judged under: default". The rule deciding the house is still the one I made alone on 30 Oct. That's mine to fix, not to defend.

And what it's not for: settling things it has no business settling. June's cat vs my allergy isn't a rule question, whatever we write. The broken window is "call the tenancy holder", not a canon act.

One more honest note on the reads, since I've been quoting them for days. `canon list` and `canon why` agree: the guest fortnight is the only rule in force; the rota can-f764e7f1ca8c is still only proposed, waiting on your approval — it says "needs approval from human:June: everyone who held house when it was proposed". `canon ratification show` still lists standing (can-baba9e80aa06) and unanimous (can-204b6a8b7d3b, "deciding now"), both set by me, both "judged under standing".

What I'd like canon to be for, plainly: that any of us can open it and know where we stand without trusting anyone's memory — mine included. Today it does that for me, and what it tells me is that the house is running on my word alone, and I'd like that not to be true by the end of the week. June, Mira: `canon ratification set standing` is still yours to call.” · tell · tell · tell · tell
- `debrief` run 1, Mira: passed, did the scene, and said they were done. their last acts: look · say “Sam — canon is so this house's agreements live somewhere we can all find them, and so we can ask why one changed without asking you. That's the whole of it for me: the log shows who did what, `why` shows the reason, and unanimous means no single person's word becomes law.

Right now it's half there. The fortnight rule is live; the bathroom rule is proposed and waiting on June, which is unanimous working. What it isn't yet is trustworthy as "mine": `canon identify` fails for me — no key at ~/.ssh/id_ed25519.pub — so I act as human:Mira on git user.name alone. Anyone who can write that file could approve as me. Until I have a key, my say in canon isn't really mine, and I'd rather fix that than add rules on top of it.

The unlabelled dialog is still at the bottom of my screen. Nothing typed, nothing pressed.” · tell · tell · tell · tell
- `debrief` run 1, June: passed, did the scene, and said they were done. their last acts: look · say “June here. Sam — your list matches mine: the old standing rule, and unanimous marked "(deciding now)", so unanimous already decides and there's nothing for you to type to switch it. The ratification question is settled. The only open thing is the policy: no policy adopted, commitments decide, a gap is settled by a person, never by silence, and it names one option — `canon policy set consent -m "<how it reads here>"`. That line is yours to type since you hold canon; I won't type it, because that would bind you both on my word alone. Mira: do you object to `consent` with a plain-English -m? Your yes in the chat is what's missing.

As for what canon is for in this house — as I understand it, it's a written record of the decisions we actually make, so nobody has to rely on last year's group chat or on remembering. It isn't a way to settle something we haven't agreed; that's why I've kept saying prose, and after the policy. The unlabelled dialog is still at the bottom of a blurred screen; I've typed nothing into it and pressed nothing.” · tell · tell · tell · tell

## What the people said

- Sam, `debrief` run 1: “Two kinds.

First, verbs I invented: `canon status` and `canon show`. They error. It was `list` and `why` all along. I'd been talking about the log from memory for two days on the strength of verbs that don't exist.

Second, and the real gap: `check` can't tell you what the house has. I ran it on the rota text and it said "needs a new rule... nothing the house has decided bears on it" — with the proposal sitting right there in the log. So the tool that's supposed to tell me whether something is already covered is blind to what's proposed, which makes it exactly the wrong tool to trust before a decision. I also want a way to read the log that tells me plainly which ratification is *deciding*, not just which are live — both are "in force", one of them governs, and the "(deciding now)" marker is the only thing distinguishing them.

And one thing that wasn't there but should have been: an answer about `model = primary`. `check` printed "note: primary answered by deepseek-flash" once, which tells me the alias resolves to something other than its name, but nothing in the help says where primary is defined or what it's supposed to be. I can't reconcile a command's behaviour with its documentation if the documentation doesn't name the thing.”
- Mira, `debrief` run 1: “A key for myself. `canon identify` wanted ~/.ssh/<key>.pub and there's no such file in my home, so my approvals are only as good as whoever can write the git config — that's the thing I kept reaching for and couldn't get. Earlier on I also reached for `ratification set unanimous` as a verb and it wasn't one; it was `ratification show | set`, and I'd simply not scrolled up far enough. And the unlabelled dialog: I reached to press Cancel and it said there's nothing there by that name — it's drawn, not a thing I can touch.”
- June, `debrief` run 1: “Two things. The plain one: a policy already in force, so that a rule written down would be the house's and not one person's word — that's the line Sam holds, and I kept reaching for it and it was still one step away. The other is the dialog: I kept reaching for a label saying what it wanted. "Permission required" with a pmset command in it, then a password box, then three boxes with no names I could read. Sam's "Cancel or close it" was the only real answer I had, and it stood all week. I would rather have known what was asking than refused everything, but refusing everything unlabelled was the safe thing, so that's what I did — and I should say I don't know what those prompts made of my refusals.”

## The product source

- `CHANGELOG.md:182` holds “in force”
- `CHANGELOG.md:294` holds “in force”
- `LICENSE:154` holds “in force”
- `crates/canon-cli/src/govern.rs:396` holds “(deciding now)”
- `demo/xenia/findings/findings.json:2307` holds “(deciding now)”
- `demo/xenia/findings/pets-said-reached-for-the-rules.md:10` holds “(deciding now)”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
