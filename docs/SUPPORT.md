# Getting help

Roughly fastest-answer-first.

## Something isn't working

1. **`canon --help`** is seven verbs, and **`canon help all`** is all of
   them, grouped by whether they need a model. Most "how do I" questions
   are one of those lines.
2. **[Getting started](./GETTING_STARTED.md)** walks a house through its
   first hour, and the **[Cookbook](./COOKBOOK.md)** is organised by the
   question you're actually asking rather than by verb.
3. **If a model is involved**, check the endpoint first. Every call prints
   the one it used, `canon config show` says what's configured, and a call
   to anything not on this machine is refused unless you pass
   `--allow-remote`. A small local model proposes worse rules and misses
   more conflicts than anything measured in this repository —
   `./scripts/draft-bar.sh 3` tells you where yours lands.
4. Still stuck: **open a bug report.** The template asks for your version,
   platform, and — if a model was involved — the endpoint and model, because
   those answer most of the follow-up questions before they're asked.

## A question, not a bug

Use **[Discussions](https://github.com/alexsbryan/canon/discussions)**.
How-to questions, "is it supposed to work like that", and what-are-you-
governing all belong there. Blank issues are off deliberately — an issue is
for something actionable, and a question that turns out to be a bug is easy
to promote.

## You pointed it at your notes and it did something wrong

That's the most useful report this project can get, and it's a bug report
rather than a discussion. `canon draft --from <path> --dry-run` writes
nothing to the canon and still leaves the run artifact behind, so the
finding travels without you having to accept anything.

Both directions count: a rule it invented that isn't in your document, and
a real rule it walked straight past. If you can share the passage, a
**fixture** is the strongest possible version of that report — see
[CONTRIBUTING.md](./CONTRIBUTING.md#the-fastest-way-in).

## Something you want to build or change

Every path in this repository is open to pull requests. See
[CONTRIBUTING.md](./CONTRIBUTING.md). The project's own rules are in
`.canon/acts.jsonl`; `canon open` prints what nobody has decided yet, which
is where to push if you want to steer something rather than fix something.

## Security or privacy

**Do not open a public issue** — including for any way something could
leave a machine unexpectedly. See [SECURITY.md](./SECURITY.md); there's a
private advisory form and an email address.

## What to expect

One steward, so response times vary with the week. Security reports get an
acknowledgement first. Bug reports with a reproduction get looked at
soonest, because they're the ones that can be acted on without a round
trip. [GOVERNANCE.md](./GOVERNANCE.md) is honest about what that means
today.
