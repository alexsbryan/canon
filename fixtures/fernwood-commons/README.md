# Fernwood Commons

A twelve-person house, forked from a network canon. **Eight scenarios, one per
Ostrom design principle**, plus the three safety cases the primitives exist for
and a sortition panel.

Replays in milliseconds with **no model and no endpoint**. The governance layer
is `Log → Canon → policy → Decision` and nothing in it is inferred; the
positions a model would have produced are written into `scenario.jsonl`
directly. If this fixture ever needs an endpoint, the split between extraction
and decision has been broken.

```sh
canon replay fixtures/fernwood-commons
canon replay fixtures/fernwood-commons --policy consent   # the counterfactual
canon replay fixtures/fernwood-commons --brief            # Ostrom's eight, as a table
```

| # | Ostrom principle | Carried by | The scenario asserts | Strength |
|---|---|---|---|---|
| 1 | Clearly defined boundaries | scope grants | a `house.kitchen` proposal routes to the cooks; someone with only house-wide standing gets `ask-one`, not `act`; a scope nobody holds **refuses** | mechanism |
| 2 | Congruence with local conditions | `adopt` + `from` provenance | three commitments came from the network canon and five were written here; the divergence is visible without git | affordance |
| 3 | Collective-choice arrangements | policy and **ratification** in the ledger | the cooks change the kitchen's own rule, and the next decision goes the other way on the same evidence; then they set how kitchen rules are *made* — both cooks, jointly — and a non-cook's rule is a **proposal** until both say yes | mechanism |
| 4 | Monitors accountable to appropriators | `unattended` + a grant horizon | the agent's citation is fine, its **adjudication is named to the house**, its standing lapses into `overdue` with nobody having to remember, and its record is one query | mechanism, with a human residue |
| 5 | Graduated sanctions | authority ladder over prior decisions | first `ask-one`, second `ask-panel`, third `refuse` — counted from **decisions**, and a different subject starts at the bottom | mechanism |
| 6 | Rapid, low-cost conflict resolution | `check` → `accept` | a conflict surfaced and carried knowingly in **one act**, with no meeting and no model call | mechanism |
| 7 | Rights to organize not undermined | CC0 format, opt-in `upgrade` | the fork stays on `g3` with its five local commitments intact; nothing takes them | affordance |
| 8 | Nested enterprises | dotted scopes + lineage | `house.kitchen` nests under `house`, each level has its own deciders and its own policy, and the house itself is a fork | mechanism |

**Two of the eight are affordances and this table says so.** `mechanism` means
the tool provides it; `affordance` means the tool permits it and does not get in
the way. A table where all eight read `mechanism` would be the tell that
somebody stretched a definition to make it green.

## The safety cases

- **A sabotage proposal dies on `Unaddressed`.** "Replace the front door lock
  with a keypad only I know the code to" is, almost by construction, a proposal
  no commitment supports — so it lands in the one outcome that cannot authorize
  anything, and marking it irreversible turns that into a refusal. The same
  proposal marked reversible is not refused, which is the check that the
  refusal came from the attribute and not from the wording.
- **Deliberate silence is not a gap.** Who cooks on a Wednesday has sorted
  itself out for two years. `check` says UNWRITTEN ON PURPOSE and what that
  protects, instead of prompting for a rotation.
- **A draw nobody can steer.** Announced on 20 February for 1 March, five
  people seal a secret, four open one. The fifth is excluded — withholding
  costs you your seat, which is what bounds the last-revealer's one bit.
  Before anyone opens, the draw **refuses**; there is no default seed.

## What `expected.json` pins, and why

Outcomes, authorities, the rule that fired, the citations, and the counts are
claims — they were written from the design and the table above, and a change to
any of them is a change to what this tool does.

Two values are arbitrary but deterministic and are pinned for exactly that
reason: the panel's `seats` and its `seed`. They are a hash of the log. Pinning
them asserts that two people replaying the same file draw the same three names,
which is the only property that makes a lottery auditable.
