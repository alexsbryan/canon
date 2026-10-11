# Before signing

This repository's own record, `.canon/acts.jsonl`, exactly as it stood before
format v3 added signed acts (at commit `1197918`). It holds 49 acts written
by builds from v1 onwards, six of them still `v: 1`.

It is frozen here because the live record will change: once its holder runs
`canon identify`, it carries signed lines, and it stops being evidence of
how an unsigned record reads. `reinterpretation_bar.rs` reads this copy and
checks that every act reads, and remints, exactly as it did before signing.

Do not edit it. A record that predates signing cannot be written again.
