# The eleven-principles project

A codebase canon on the `code` profile, forked from a shared engineering canon.

**This is the transfer test.** Fernwood Commons shows the eight design
principles working in a house. If the result were about coliving rather than
about the primitives, it would not survive being moved to an institution with
different membership, different stakes and a compiler. It does, and the
translation is one column wide:

| # | In a house | In a codebase |
|---|---|---|
| 1 | who holds the kitchen | who owns the module |
| 2 | forked from a coliving canon | forked from an engineering canon |
| 3 | the cooks change the kitchen's rule | the parser's owner changes the parser's rule |
| 4 | the agent's grant expires | the bot's commit rights expire |
| 5 | first ask, then the house, then no | first a reviewer, then the group, then no |
| 6 | a conflict carried knowingly | a conflict carried knowingly |
| 7 | the house keeps what it wrote | the fork keeps what it wrote |
| 8 | kitchen inside house | module inside crate |

Same primitives, same strength marks, no new mechanism. Run it:

```sh
canon replay fixtures/eleven-principles
```

## What this one carries that the house does not

- **Entrenchment.** Some of these commitments are ranked `principle` and some
  `convention`. Amending a convention is `act`; amending a principle is
  `ask-panel`. That is one policy wrapping another — `cautious/entrenched/
  consent` — and the rank is an annotation the policy reads, so the library
  never learns what a principle is.
- **The reversible attribute.** "Delete the v1 reader and the migration path
  with it" is refused; the same change behind a flag is not. The refusal comes
  from an attribute the person typing the command supplied, not from the
  wording — which is why both spellings of the same sentence are in the
  scenario.
- **An agent's authority as a scope grant with a horizon.** `agent:depbot`
  holds `engine.deps` until the new year. There is no agent feature: it is a
  grant, it expires, and `overdue` says so. Its citation of the wire rule is
  fine — reading is what it is for. Its *adjudication* lands in `unattended`
  with its name on it.

## Where the eight are, in the scenario

Steps carry `principle` and `strength` fields, and `canon replay` prints them.
Two of the eight are `affordance` in both fixtures and that is the honest mark:
the tool permits congruence and the right to organize, and gets out of the way;
it does not provide them.
