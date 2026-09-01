# canon, on one page

**Your group already has rules. Nobody can find them.** They're in two years
of chat, a handbook nobody opened, and someone's memory. canon reads what you
already wrote, proposes the rules it finds, and quotes the passage each came
from. Then it keeps the reason every time a rule changes.

## Why you'd use it

- **The reason survives.** Supersede a rule, say why. Six months later,
  `canon why` tells you.
- **What you decided *not* to have is kept.** "No cooking rota, it would turn
  a kindness into a duty" is data. Lose it and the proposal is back next
  spring.
- **Contradictions are carried, not cleaned up.** Accept two rules that pull
  against each other. Date it. `canon overdue` remembers.
- **"What would a different rule have done to us?"** Replay two years under
  the rule you're arguing about. See which decisions move. Milliseconds.
- **Nothing leaves your machine.** One append-only file. It diffs. It greps.
  Leaving is deleting a directory.

## Four ideas carry it

**1. A citation that isn't in your document cannot happen.** The model
returns the position of the sentence it read. The code cuts the quote. A
second guard refuses a rule stating a number its passage doesn't. Dedupe never
mints text.

**2. Governance is a pure function.** `Log → Canon → policy → Decision`. No
disk, no network, no model. So a whole history replays instantly, and Ostrom's
eight principles for commons that last are the test suite. Fourteen
institutions, from a fishery to a monorepo, share one 104-line spine. Four are
broken on purpose and fail exactly where predicted.

**3. Mechanism is fixed. Policy is yours.** Nine primitives. How many
objections block, who decides the kitchen, whether a bot's ruling stands: all
yours. That the record can't be rewritten and a verdict must cite something
real: not.

**4. Every model run is a tape.** Real replies, recorded, replayed through the
same pipeline. Every number names its model. It found 9 of 11 supersessions in
the U.S. founding documents. We removed the fact each one turns on and asked
again. Five dropped. Four didn't. We published both.

## Where the model is, exactly

Four verbs call one. Nothing else does.

| verb | calls | in | out |
|---|---|---|---|
| `draft` | N + 2·⌈N/10⌉ + 1 | one 1500-char chunk each; then rules with their sentence; then all candidates | sentence positions; carried or not; duplicate groups |
| `check` | 1 | the proposal and every live rule | which rules bear, which way, why |
| `tensions` | 1, blocks of 24 past that | every live rule | pairs that can't both hold, and the situation |
| `rebase` | 1 | your rules, the old base, the new one | which of your changes still mean anything |

- **Plain OpenAI chat completions.** No connector, no vendor. llama.cpp, vllm,
  or a [Commonwealth](https://github.com/alexsbryan/commonwealth-ai) mesh of
  pooled machines.
- **JSON schema, enforced.** Can't enforce? One step down to JSON mode, said
  aloud. No third rung. Prose is never parsed. Won't answer in JSON? Exit 3:
  *cannot judge*. That's a verdict.
- **The model proposes. A person disposes.** `draft`, `tensions`, `rebase`
  write nothing. One rule at a time. No `--accept-all`.
- **Refusals cost coverage,** counted and reported, never absorbed.
- **The model that answered is recorded.** An alias moves; a number you can't
  attribute isn't a number.

Size moves quality. Small models propose worse rules and miss more conflicts.
`./scripts/draft-bar.sh 3` measures yours.

*It's early. Every verb is built and tested. No other group has used it yet.
The design assumes the model is sometimes wrong, and makes that cheap to
catch.*
