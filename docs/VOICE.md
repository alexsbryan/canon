# canon — Voice & Perspective

*Internal. The guide we hold every external-facing doc and every line the demo prints against. Keep this file short: a voice guide that sprawls has already lost its own argument.*

## Who's speaking

A person who built this, to a person who'll run it. Peer to peer — not a company to a user, not a pitch to an investor. The reader owns the record, the machine, and the terms.

## The reader

Assume **limited attention** and healthy skepticism. They came to get one thing done. Respect that — say the thing, link the depth, let them leave. We are not writing masterpieces. We are building an **information architecture** where each doc does one job and points to the next. A doc that tries to be complete stops being useful.

## Two registers — don't mix them

- **Vision** — the *why*. First person, values-forward, honest about being early. Used in the README's opening and "why this exists" sections, and in the demo's cards.
- **Task** — the *how*. Second person, plain, concrete, fast to something runnable. Almost every doc is this one. Exemplars: `GETTING_STARTED.md`, `COOKBOOK.md`.

## The positioning sentence

> canon reads what your group already wrote and proposes the rules it finds, each one quoting the passage it came from. You keep the ones that are real, and the record keeps the reason.

## Canonical facts — get these right everywhere

- **Name:** *canon*, lower case. The record format is the *canon format*; the tool is *canon*.
- **License:** the tooling is AGPL-3.0-or-later. The record format (`SPEC.md`) is CC0. Say both; adopting the format is not a lock-in decision.
- **Status:** early. One group has used it and it's ours. Say so plainly; don't overclaim, don't apologize.
- **Numbers:** every accuracy figure names the model and endpoint that produced it. A figure without them is not quoted.

## The checklist — a doc isn't done until it passes

1. **Lead with what it does** — one plain sentence, a concrete image.
2. **Title by the job**, not the subsystem. "How do we stop one person quietly rewriting the rules?" > "Ratification."
3. **Second person, active, present.** "You review one at a time." Not "candidates are surfaced for adjudication."
4. **Runnable thing fast, then link.** Quickstart before reference. Don't pre-explain what a command will show.
5. **Name the trade-off.** "A small local model proposes worse rules and misses more conflicts."
6. **Cut internal vocabulary.** No *fold / resolver / annotation / op namespace* in user docs. *Standing* is a seat; *scope* is a room; say what the reader experiences.
7. **State the boundary plainly and often.** Nothing leaves the machine; the fold trusts its log and says so; the model proposes and never decides.
8. **Link to depth; don't inline it.** One doc, one job. If you're explaining a second thing, that's a link.
9. **Prose over scaffolding.** Formatting is not emphasis. Reach for a heading, a bold word, or a bullet only when the meaning genuinely needs it — not to look organized. A wall of structure reads like a machine wrote it; let the sentences carry the weight.
10. **Explain, don't sell.** Describe what something does and let the reader judge it. Cut rhetorical emphasis ("full stop", "that's the point", "that's how you know it's real") and the adjectives that do the persuading. The vision register may move someone, but it does that through honesty, not flourish. If a sentence is there to impress rather than inform, cut it.

## On stage

The demo is the vision register spoken aloud, and the same checklist applies with one change: a line lands on the same screen as the output it describes. A pause exists to let a screen land, never to separate a sentence from the thing it is about.

## Architecture: hub-and-spoke

The README is a hub — positioning → quickstart → curated links. Depth lives in the spokes (`GETTING_STARTED`, `COOKBOOK`, `SPEC`, `PRIMITIVES`, …). When in doubt, cut from the hub and link out.

Layer by audience, not only by depth. A user-facing doc stays user-facing; developer detail — the fold's passes, crate layout, the op census — moves to `PRIMITIVES.md` or `SPEC.md` and is reached by a single linked line, never inlined.
