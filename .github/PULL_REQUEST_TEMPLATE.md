<!--
Keep this light. CI runs the checks for you — rustfmt, clippy, the whole
suite, and a link check over the docs — so there is nothing to prove here.
This is just to help a reviewer follow the change.

Running ./scripts/pre-push.sh first gets you the same answer in about ten
seconds. ./scripts/install-git-hooks.sh wires it to `git push`.
-->

## What this changes

<!-- A sentence or two. Link an issue with "Closes #123" if there is one. -->

## How you checked it

<!-- However you convinced yourself it works — a test you added, a command you ran, output you pasted. Rough is fine. -->

## Notes for reviewers

<!-- Optional: trade-offs, things you're unsure about, follow-ups you're leaving. -->

<!--
A few things only apply to some changes, and none of them are a checklist to clear:

- Adding an op to the format? `adequacy_bar` will fail until the census names
  the primitive it serves and the composition that did not reach it. That
  third column is the actual work.
- Adding a fixture? A PROVENANCE.md naming the upstream and the sha256 of
  every file copied is what makes it re-checkable later.
- Quoting an accuracy number? Name the model and the endpoint that produced it.
- Changing how the project itself is governed? That's a line appended to
  .canon/acts.jsonl with `canon`, not an edit to a markdown file.
-->
