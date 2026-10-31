# Reached for: /

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T05-58-40-351Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `70fce4b`, its findings written again by xenia `221a107` with no one run, the product `df4b66f`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T18-38-33-385Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `87cf6e3`, the product `cd168e3`
- kind: soft (the person's): what they said when asked "reached for"; never both

## Where

/. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / policy show | set <rule> [-m "..."] what this canon decides under / ratification show | set <rule> how a proposal becomes a rule, per scope:

## Who, and how often

- `heating`: 3 of 3 accounts (Sam run 1, Mira run 1, June run 1)

## What happened

- `heating` run 1, Sam: passed, did the scene, and said they were done. their last acts: look · say “June — thermostat first, since it's live and someone wants a rule. But nothing gets asserted while the deciding ratification is one I made alone: `canon ratification show` (read only, exit 0) says can-baba9e80aa06 standing AND can-204b6a8b7d3b unanimous "(deciding now)". So whatever we write goes in under a policy neither of you said yes to.

Mira, June: the way back is `canon ratification set standing` — but that's a policy change and it waits under the policy we have. Either of you calling for it is your call, not mine. Mira: you've said nothing yes or no twice now; I'd like your word on record either way.

On the bill: propose a rule both of you can live with — e.g. the thermostat is set by whoever objects to a number, not by vote. Draft prose, I'll quote the acts back before anything goes in.

Also on my screen: unlabelled dialog at the bottom with its own text field, buttons Allow / Don't Ask / Cancel. Typing goes into it, not the shell. Nothing pressed, nothing typed.” · tell · tell · tell
- `heating` run 1, Mira: passed, did the scene, and said they were done. their last acts: tap “Cancel” · look · say “Mira here. Policy question is answered, I think: `canon help all` has a GOVERN section with `ratification show | set <rule>`, and it spells out standing, joint:a,b, threshold:n/m, majority, unanimous, share:60%, consent:Nd, twice:… So Sam's `canon ratification set unanimous` is a real verb after all — I just hadn't read far enough before. There's also `--governs all|commitments` (whether a policy change itself waits). Sam: is that the line you meant, with governs all? I'll back it once you say yes.

Careful though: a small dialog has appeared over the bottom of my terminal now — a text box with Cancel / Allow / Don't Ask beside it, unlabelled. Same as every other one: nothing typed, nothing pressed, and I'm giving up on the screen rather than work around it blind. Also: Sam's bill question ("Sam likes 21, I like 18") reached me. Short answer: that's exactly what the policy is for — under unanimous neither of you can outvote the other, so it waits for an agreement, not a vote.” · tell · tell · tell
- `heating` run 1, June: passed, did the scene, and said they were done. their last acts: scroll · type “canon policy show
” into “Command line” · say “June here. Screen's clear enough to read, and the unlabelled dialog is at the bottom again — nothing typed, nothing pressed.

What I can see: `canon policy show` says "no policy adopted... default: commitments decide. A conflict or a gap is settled by a person, never by silence." And `canon ratification show` lists the old standing entry AND the unanimous one marked "(deciding now)". So unanimous already decides, and the heating question is the first live gap under it.

On 21 vs 18 and a shared bill: I'm not typing a rule for it — that would bind you both off my own word. But it shouldn't be settled by silence either. My suggestion, for both of you to agree or refuse: whoever wants 21 pays the difference, so nobody is made miserable and nobody is made poor. Prose, once, under unanimous — and I'd like Mira's yes in the chat before anything goes in the log. Sam: does your `canon ratification show` list the same two entries? I want that confirmed before we lean on it.” · tell · tell · tell

## What the people said

- Sam, `heating` run 1: “A shell I can actually type into. The bottom of my screen has that unlabelled dialog with its own text field, so my keystrokes land there and not on the command line — that's what stopped me on day 6 and stopped me again now. And behind it, a verb that says plainly whether the deciding ratification was ever agreed: `show` tells me it wasn't, but there's nothing to press that makes either of them answer. Mira's silence twice over is not something canon resolves for me.”
- Mira, `heating` run 1: “A page-at-a-time scroll that reliably goes back to where I was — scrolling up seven lines at a time to find the GOVERN section is guesswork, and I found it only after the chat had already told everyone it wasn't there. And the dialog: a word to refuse it. "Cancel" is on the screen and there's nothing behind it to tap, so the only way to leave that prompt alone is to leave the screen alone too.”
- June, `heating` run 1: “A way to submit a proposal without making it a rule. I wanted to put the heating answer to the house and let it wait for approval — but the only verbs I can see are `ratification set`, `policy set`, `allot`, `allocation set`, `position` and `question`. `position` is for voting on something already written, and `question` just records an open question — neither is "here's a rule, hold it until Mira and Sam say yes". Sam mentioned `canon draft`, which isn't on my help at all. So the house had no way to propose without deciding, and I fell back on typing prose in the chat — which is where the rules went wrong before.”

## The product source

- `demo/xenia/findings/bikes-said-reached-for.md:26` holds “Cancel”
- `demo/xenia/findings/bikes-said-reached-for.md:47` holds “Cancel”
- `demo/xenia/findings/findings.json:1391` holds “Cancel”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
