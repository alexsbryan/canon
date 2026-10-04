# canon

canon helps your group find its rules in old notes and chat, and keep the
reason when they change.

Whether you share a house or work on a project, your agreements can be
scattered across documents and people's memories. canon proposes what it
finds, quoting the source. You review each proposal and keep the ones that
reflect what you agreed.

It's early. The only group using it so far is ours.

## Why this exists

We think the people living with a rule should have a say in making it, and
changing your mind should leave a reason, not erase the past. Your group
chooses who decides what and how proposals become rules. The tool keeps
that record; it cannot make a group fair.

Open questions and things you deliberately leave unwritten belong in the
record too.

## Get started

Install on macOS or Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/alexsbryan/canon/main/install.sh | sh
```

The [installer](./install.sh) puts one binary in `~/.local/bin`. On Windows,
download the zip from
[releases](https://github.com/alexsbryan/canon/releases/latest).

In the folder where you want to keep your group's record:

```sh
canon init --profile house
canon add "Quiet hours are 10pm to 7am."
canon list
```

Use one of your own agreements. These commands need no model; the `house`
profile just calls agreements “rules”.

To read existing notes or chat exports, first run a local
OpenAI-compatible model server. Set its address and model name, then point
canon at your folder of text:

```sh
canon config set endpoint http://localhost:8080/v1
canon config set model YOUR_MODEL_NAME
canon draft --from ~/group-notes
```

Replace the address, model name, and folder with yours. Review each proposal
against its source passage. Models miss agreements and misread them;
smaller models tend to do worse.

[Getting started](./docs/GETTING_STARTED.md) covers model setup, changing a
rule with a reason, and sharing the record.

## How it works

canon has two halves. The record is `.canon/acts.jsonl`, a text file.
Changes and undos add entries rather than rewriting history. You can keep
it in git, share it, and read it without canon. No account is needed.

Code reads that history to work out what's in force and who may decide,
under your group's chosen rules. This needs no model. You can replay past
decisions under a different policy to see what would change.

Only `draft`, `check`, `tensions`, and `rebase` call a language model, to
help read documents or compare rules. A model offers evidence and
proposals; it cannot make a rule. canon refuses remote endpoints unless
you pass `--allow-remote`.

The record trusts whoever can write the file. Protect shared copies with
permissions and review; [the security guide](./docs/SECURITY.md) explains
the boundary.

## Go further

- [Everyday questions](./docs/COOKBOOK.md) — who decides, how to object,
  and what to do when the rules don't cover something.
- [The guide](./docs/GUIDE.md) — the ideas and commands, in plain words.
- [The design](./docs/PRIMITIVES.md) — what the tool fixes and what it
  leaves to your group, informed by Elinor Ostrom's work on shared resources.
- [The evidence](./docs/DEMO_PLAN.md) — experiments, measurements, and
  their limits.

To build from source or contribute, start with
[Contributing](./docs/CONTRIBUTING.md). This project's own rules live in
`.canon/` too; [Governance](./docs/GOVERNANCE.md) says who decides here.

The tool is AGPL-3.0-or-later. The [record format](./docs/SPEC.md) is CC0;
you can implement it independently.
