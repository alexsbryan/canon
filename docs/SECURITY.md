# Security

canon's promise is narrow and worth stating exactly: **there is no account,
no server, and nothing leaves your machine.** The record is a file in a
directory you own, leaving is deleting that directory, and the only network
call the tool can make is to a model endpoint you configured. The one thing
that listens is `canon ui`, on `127.0.0.1` only, for as long as you leave it
running.

A security issue here is anything that breaks that promise, or that puts a
record, a key, or a sealed lot at risk. Reports are taken seriously,
including from people new to the project.

## Reporting a vulnerability

**Please don't open a public issue.** Report privately, either way:

- **GitHub (preferred)** — open a draft advisory from the repository's
  [Security tab](https://github.com/alexsbryan/canon/security/advisories/new)
  (Security → Report a vulnerability). It opens a private thread with the
  maintainer and nothing is public until a fix is ready.
- **Email** — svrnmesh@proton.me.

Include enough to reproduce: what you ran, what happened, and
`git rev-parse --short HEAD`. If a model was involved, name the endpoint
and the model — the tool prints both on every call. A proof of concept
helps; a clear description is enough to start.

## What we care about most

Everything matters, but these map directly onto the promises the tool
makes, and a break in any of them is the sharpest kind of bug this project
can have.

**A passage reaching an endpoint that shouldn't have it.** `draft`,
`check`, `tensions` and `adopt --rebase` send passages from your own documents to a
model. A call is refused unless the endpoint resolves to this machine,
unless the person running it passes `--allow-remote`, and every call prints
the endpoint it used. Any path that sends a passage without that gate, or
that misjudges an endpoint as local, is the report to send first. The
host-matching is deliberately strict — `dev.localhost`,
`localhost.example.com` and `127.0.0.1.example.com` are all covered by
tests, because each is a way to look local without being local.

**Ingest reading more than it was pointed at.** `canon draft --from <path>`
walks a directory and reads anything textual under it. It honours
`.gitignore` unless you pass `--include-ignored`, it does not follow a
symlink whose target leaves the root it was given, and a path that is not a
regular file is refused rather than read. A traversal that escapes the given
root anyway, or a credential file that ends up quoted verbatim in a
proposal — and therefore in `acts.jsonl`, and therefore in your git
history — is a real hazard, since every proposal carries its source passage
by design.

**The agent surface.** `canon mcp` serves tools to an agent over stdio.
**Every tool on it is a read.** Amending the canon requires the CLI, run by
a person. A write reachable from that surface would let an agent mint a
rule, which is the one thing the design says it must never do — an agent
may propose and object, and cannot mint.

**A sealed lot that can be steered.** `canon draw` seals a secret before a
boundary and opens it after, so a panel nobody chose can be recomputed from
the log. Secrets live in `.canon/secrets/`, which writes its own
`.gitignore` of `*` on first use. Anything that lets a secret be predicted,
read early, or committed — or that lets `fresh_secret` fall back to
something guessable rather than refusing — breaks the property the draw
exists for.

**The local page reaching past its door.** `canon ui` serves a page that
writes as the person who started it: it approves and objects, proposes,
changes and retracts rules, records and lifts what is left unwritten, and
identifies — the last only with a key it found in `~/.ssh` or the
ssh-agent and offered by fingerprint, never a path from a form. It listens on
`127.0.0.1` on a port the OS picks; the address it prints carries a random
token, traded on first visit for a `SameSite=Strict`, `HttpOnly` cookie; it
answers only to a `Host` naming that address, which stops DNS rebinding; every
write must be a `POST` from the page itself and carry the token again, and
a decision must concern something waiting on that person; and the page loads nothing and
runs no script. Another website driving it, another machine reaching it,
a rule's text running as markup, or a decision about something the person
has no say in, is the report to send. A program already running as you is
not, since it could run `canon approve` as you anyway; `canon identify`
with a key that needs a touch is the answer to that.

**An act counting under a name that has bound a key, without that key.**
Once someone runs `canon identify`, an act under their name counts only if
one of their keys signed it or their binding vouched for it. A line that reaches the
derived state under a bound name any other way — a parsing gap, a
verification bug, a way to make `refs` match text that changed — is a break
of the one promise signing makes. So is a write path that appends an act
under a bound name unsigned instead of refusing. canon never holds a private
key; `ssh-keygen` signs.

**The record itself.** `acts.jsonl` is append-only and everything is
revertible, including a revert. A path that rewrites or drops a line rather
than appending is a correctness bug with the shape of a security one:
whether it deletes on purpose or by accident, the reason is gone.

## What is not a vulnerability

- **A model proposing a rule that isn't in the document.** That's the known
  failure the design assumes. It's why review is one at a time, why there
  is no `--accept-all`, and why every candidate carries its source passage.
  Report it as a bug, or better, as a fixture.
- **`--allow-remote` sending your text to a remote endpoint.** That's the
  flag doing what it says. A remote endpoint reached *without* it is the
  bug.
- **Anything reachable only by someone who already has write access to your
  `.canon/` directory**, with one exception. They can edit the file, and the
  record's integrity story is git history and review, not a lock. The
  exception is a name that has bound a key: an act under it that its key
  did not sign must not count, write access or not.
- **A program running as you signing as you.** If your key is unencrypted
  or loaded in an agent that doesn't ask, anything you run can use it. A
  FIDO key that needs a touch, a passphrase that isn't cached, or
  `ssh-add -c` is what stops that; canon says so when you bind a key that
  needs no touch.
- **Someone binding your name before you do.** The first key bound to a name
  wins. Bind before your first act, and compare fingerprints with your
  group some other way than the file.

## Supported versions

Pre-1.0: fixes land on `main`. Please test against `main` before reporting —
it may already be fixed. Because this is software you run yourself, there is
no service to patch on your behalf; the honest answer to "am I covered" is
"are you on a recent commit".
