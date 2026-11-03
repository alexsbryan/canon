# V1 compatibility admission experiment

This fixes the retained `project` / `--basis` interpretation. The implemented
temporal v2 path is described in [README.md](README.md) and the
[mechanism/interpreter boundary](../../docs/INSTITUTION_BOUNDARY.md). V1's
limitations below are intentional compatibility behavior, not limits of the
new mechanical projector or temporal engine.

This is an optional library with a diagnostic `canon-institution` runner, not a change to canon's format,
CLI, public API, or default fold. Its named interpretation is
`shared-assent-v1`. It reuses canon-core's parsing, signature verification,
ratification procedures and projection; it does not read the experiment's case
names, expected results, actor names or prose.

## Boundary

Inputs are an established **basis log**, a full log extending that basis, and
an explicit Unix time. The basis is the caller's explicit activation boundary:
its existing institutional state is taken as given. New positions are evidence;
new removals, reversals, grants, withdrawals and jurisdiction changes are
requests. The source log is never rewritten.

Each request is normalized into the scopes whose institutional state it would
change. Repeal and undo of an adopted commitment name its existing scope;
withdrawal names the scopes of the grants it removes; placement names both the
old and new scopes. Reversals inspect their targets, not their author's name.
The author may cancel their own still-pending commitment, and a holder may
resign their own seat. These are explicit choices of this optional profile.

One admission procedure asks canon-core's existing ratifier for assent under
every affected scope's existing procedure. It reads the basis plus positions,
never the requested changes to authority. Missing assent leaves the request
pending; an objection refuses it. Only admitted requests reach the projected
log. Every omitted request remains named in the output with its requirements.

## Deliberate limits

This is one admission round over a fixed basis, not a general chronological
institution interpreter. The electorate and procedures are fixed by that basis.
It does not claim historical effect times for changes approved later, or
resolve interdependent changes inside a round. Its projection is a current
snapshot, explicitly labelled with this limitation. A new request must not
predate the basis; future acts are not used as evidence.

New founding, rule-writing, constitutional changes, key bindings and other
unsupported request kinds produce an error, not a guessed interpretation.
Unknown annotations remain carried. The executable must demonstrate that an
authorized request can actually take effect through the existing fold;
unsupported fold interactions are reported rather than called applied.

Evidence that changes the basis's constitution/electorate or combines an
existing rule's adoption with structural mutation refuses the round. Reversing
a resignation is not the same effect as resigning. Reversing placement needs
the pre-act jurisdiction and is unsupported here.

Acceptance: all ten original cases, renamed actors/scopes/text, affirmative
assent to protected changes, both sides of jurisdiction transfer, no
self-authorizing membership change, no future approvals, and an unchanged raw
record. No new primitive or institutional opinion is added to canon-core.
