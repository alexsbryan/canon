# Design: so nobody can ratify as you by setting an env var

*Internal. Written before it was implemented and revised as it was built: it describes what was built, and says where the first draft was wrong. It says what changes, what was considered and turned down, what it still cannot stop, and what pins it. Euphemia raised it; nothing here is euphemia-specific.*

## The problem

`store::actor()` takes `CANON_ACTOR`, else git's `user.name`, else `$USER`, and the fold believes the string. So `CANON_ACTOR=human:alex canon approve …` is a ratification by alex. SECURITY.md says so honestly: anyone who can append to `acts.jsonl` can be anyone. Euphemia's README repeats it: "canon trusts the label it is given."

In euphemia that boundary is too wide. Agents run as the person's own Unix user, in checkouts that write the commons' canon and push it to a bare git remote. Nothing there checks a commit signature: there is no CI and no branch protection. The thing that separates "an agent proposed" from "a person ratified" is a string any of those processes can set.

What we want: a person can opt in so that acts under their name count only if their key signed them. People who don't opt in see no change. Canon gains no account, no server and no key store.

## This reverses a recorded decision

Two acts in this repository's own canon say not to do this:

- **`can-ef55ca2a8f9c` (silence, "identity and time inside the format").** "A signing key still needs somebody outside canon to bind it to a person. A timestamp the fold could verify would need a clock the fold does not have. Both stay the substrate's: signed commits and branch protection where the ledger lives in git, mutual visibility where it lives on a shared folder."
- **`can-50996b2caed2` (assert, `canon.core`).** "The fold takes ts_unix and actor as given."

Both have to be superseded, with a reason, by their holder before anything below lands. The argument for doing so:

1. **The substrate euphemia uses has no defender.** The silence names two substrates: git with branch protection, and a shared folder with mutual visibility. Euphemia is a third: git with no gate, written by agents running as the person. `canon guard git` can only reject at a remote that runs it.
2. **The binding objection still holds, and the design keeps it.** Canon does not decide that a key is alex's. The person binds their own key, and the group confirms the fingerprint the way it confirms anything else, out of band. That is the same thing a `grant` does when it ties standing to a name string.
3. **The time objection was the one that nearly sank this, and the design does not rely on time.** See "One binding, decided by content" below. The fold still has no clock and still trusts `ts_unix` for what it already uses it for.

If you'd rather keep the silence, the substrate alternative is written up under "Considered and turned down". It is smaller, but it does not cover euphemia.

## The shape of it

A person runs **`canon identify <key.pub>`** once. That writes a `key` act binding one or more SSH public keys to their actor name, signed by one of those keys. From then on, an act under that name counts only if one of two things is true:

- a bound key **signed** it, or
- the binding **vouched** for it by digest. That is how their history from before binding keeps counting.

Any other act under that name is **carried and not applied**: kept in the log, listed in a new `Canon::unverified`, and shown wherever `unattended` is shown today. Names that never bind behave exactly as now. Run again, `canon identify` says who you are and what doesn't count. That is the whole verb.

People sign with `ssh-keygen -Y sign`, run as a separate program, so canon never touches a private key. Canon verifies in-process. The pieces:

- one optional envelope field, `sig`;
- one optional envelope field, `refs`;
- one new annotation, `key`;
- one pre-pass at the top of `fold::derive_at`;
- one small no_std dependency in core;
- a signing step in `cmds::write`, and the `identify` verb.

## 1. What gets signed

**Canonical bytes.** The act's JSON object as it appears on its line, minus `sig`, re-rendered compactly:

- keys sorted bytewise;
- strings escaped minimally: `"`, `\` and U+0000–U+001F only, with `\b \f \n \r \t` and lowercase `\u00xx`;
- integers in shortest decimal.

That covers `id`, `v`, `ts_unix`, `actor`, `op`, the body and `refs`. Canon writes no floats; the only `f64` in core is display text in `allot.rs`. These rules match what `serde_json::Value` renders without `preserve_order`, which the workspace already forbids. SPEC states the rules rather than naming serde, and a test vector pins them.

*Corrected in implementation.* This section first said to verify from the raw line, so a build that drops a field it doesn't know couldn't break a valid signature. But `canon merge-driver` re-renders every line through `Act`, so raw bytes don't survive a merge anyway, and a log would verify differently depending on whether it had been merged. The canonical bytes are therefore rendered from the parsed act. A line with a field this build doesn't know loses it and stops verifying; that fails closed, since the act simply doesn't count. `every_act_kind_survives_the_wire_with_its_id_intact` already pins render∘parse for every known op, and `a_signed_act_survives_render_parse_and_a_merge` pins it for signatures.

**Not the id.** An id is 12 hex characters, 48 bits (`id.rs:27`), and nothing checks it against the content (`Log::parse` trusts the stored id). Two proposals with the same id take about 2²⁴ hashes, which is seconds. A signature over an id would vouch for whichever text sits behind it.

**`refs`: referents pinned at full strength.** A signed approval points at an agent's proposal by that same 48-bit id. So the signer also signs `refs`, a map from every act id the body mentions to the full SHA-256 of that act's canonical bytes. The signer scans the body for any string equal to an id in the log, so this needs no per-op knowledge. On verification, each listed referent that is present must match its digest; a missing referent is allowed. Swap a colliding proposal in behind the id and the approval no longer counts.

`refs` always includes the canon's earliest act, and a signed act must name at least one act that is present. That makes a signed act pasted into an unrelated canon fail. A fork shares its parent's history, so acts can still move between parent and fork; that is accepted.

A `key` act is the exception: it need not name anything present, because `identify` may be the first thing written in a new canon. *Corrected in implementation:* the first version allowed an unanchored binding only as the canon's earliest act. In the founding test, `identify` and the grant landed in the same second and the grant's id sorted first, so the binding stopped holding and the next write went out unsigned. Clock skew between machines would do the same thing, retroactively. No ordering rule survives that, so none is used. The cost is a residual named below: a binding pasted in from another canon.

**`sig`.** The SSHSIG blob from `ssh-keygen -Y sign -n canon`, base64, without the armour lines. The `canon` namespace means a git commit signature can't be replayed as an act signature, and the reverse. Anyone can check an act without canon: re-wrap the blob, write an `allowed_signers` line from the `key` act, and run `ssh-keygen -Y verify -n canon`. SPEC shows how, in keeping with "read it without canon".

**Key types.** Two:

- `ssh-ed25519`;
- `sk-ssh-ed25519@openssh.com` (FIDO). For FIDO keys, verification requires the user-presence flag, as OpenSSH does by default. So "a person touched the key" can be checked by any reader, not only the signer's machine.

RSA and ECDSA keys are refused when you bind, with a message. SSHSIG's `sha512` and `sha256` both come from `sha2`, which core already has.

## 2. One binding, decided by content

The first draft said a binding protects acts dated after it. That fails against the attacker this is for: the forger dates the forged act a second before the binding, and it counts. A rule based on `ts_unix` can't tell an honest unsigned act made before binding from a forged one backdated, and the silence was right that the fold has no clock to settle it.

So validity is decided by content, not time:

```
{"op":"key","keys":["sk-ssh-ed25519@openssh.com AAAA…","ssh-ed25519 AAAA…"],
 "prior":["<sha256>", …]}
```

- **`keys`**: the keys that sign for this name. More than one is allowed: a laptop and a hardware key, or an offline backup.
- **`prior`**: full digests of everything the person wrote under the name before binding. A large history means a long line, about 66 bytes per act; this repository's own canon would need about 3 KB.

The act is signed by one of its own keys, which proves possession. The actor is the act's own `actor`, never a body field, following the rule `position` already keeps. An act under the name counts if its digest is in `prior` or a bound key signed it. Nothing else does:

- **Old history** survives because the binding vouches for it.
- **A forgery backdated before the binding** isn't vouched for, so it doesn't count.

**A name binds once.** A second valid `key` act for the same name doesn't replace the first; it contests the name. Neither wins, because "the earlier one" would be decided by a timestamp the loser can write. Only digests in every binding's `prior` still count, and nothing signed does. **No unbinding and no revert:** a `revert` that targets a `key` act has no effect. Losing every key means the name is retired: holders `withdraw` its grants and `grant` the person's new name. Both acts exist already. Name a backup key when binding if that would hurt.

**Rotation and vouching were built and then removed.** The first implementation chained key acts (`prev`, each link signed by the previous one's key) and had `canon key rotate` and `canon key vouch`. That added a chain walk, fork handling and three more verbs, all to avoid "a lost key means a new name", which was already the v1 decision. One verb and one binding is enough for now. Chains can come back as a format extension, and they fail closed: a v3 reader would see a second key act as a contest, never as a takeover.

**Two machines, one person.** Acts written unsigned under your name on a machine that hadn't seen your binding yet arrive after the merge without being vouched for, so they don't count, and `list` says so. Re-add them.

## 3. Where it bites

There is one choke point: a pre-pass at the top of `fold::derive_at`, which has two direct callers (`log.rs:143`, `log.rs:151`). The pre-pass:

1. finds each actor's binding;
2. moves every act that fails into `Canon::unverified`;
3. hands the rest to the fold unchanged. `governance_before` gets the same filtered slice.

The pre-pass depends only on `key` acts, digests and signatures, never on standing. So none of the scattered `is_human` sites change (`fold.rs:956`, and `ratify.rs:471`, `513`, `534`, `594`, `632`, `864`).

**Turned down: treating an unverified act as non-human.** That reuses `unattended`, but `Canon::seat` and `gate()` don't check humanity. A forged `grant` or `ratification` under a holder's name would still apply, only flagged. "Carried, not applied" is safe for every op.

## 4. Writing

`cmds::write` gives every live write one place to hook in. The one exception is `leave -m`, which appends as `anonymous` and stays unsigned. After building the act, `cmds::write`:

1. loads the log;
2. computes `refs`;
3. signs only if the actor is bound. Unbound actors keep writing v2 lines, so their canons stay readable by older builds.

**Nothing to configure.** The log already says which keys are bound to the name, so canon finds the one to sign with the way SSH would: a matching public key in `~/.ssh`, else a matching key the ssh-agent lists (`ssh-add -L`). `CANON_SIGNING_KEY` names a key kept anywhere else. *Corrected in implementation, twice:* the key path first lived in git config, for a bad reason (the frozen baseline tests). Then it lived in `.canon/config`, written by `identify`, until a test showed that a file belonging to a directory isn't a person's. On a shared folder, the key one person named there became the default for the next person to run `identify`, who was then silently bound to someone else's key. A key belongs to the person, and their SSH setup already says which keys they hold.

**Signing fails, so nothing is written.** If the actor is bound and no bound key is in `~/.ssh` or the agent, or `ssh-keygen` fails, or the person cancels the touch, the write is refused with the bound fingerprints. An act that would be carried and not applied is never appended. That matches euphemia's rule that an approval which wouldn't count isn't written.

**The verb.** `canon identify [<key.pub> [<backup.pub>…]]`:

- On an unbound name, it binds, vouches for the name's history, and prints the fingerprints for the group to check. Without a key named, it lists what `~/.ssh` holds rather than choosing.
- On a bound name, it says who you are, how many acts under the name don't count, and which key would sign here. It exits 2 if none would.

`list`, `log` and the MCP `canon_list` report acts under a bound name that don't count. MCP stays read-only.

## 5. Format, dependency, contract

**Format v3.** A line that carries `sig` or `refs`, or is a `key` act, is written as `v: 3`. Older readers refuse it, which is the version rule doing its job: a reader that applied forgeries silently would be worse than one that declines to read. This is the same argument SPEC makes for v2. Canons with no bindings keep writing v2 and remain readable everywhere. SPEC gains one section; the census test (`can-96d951ae3378`) needs `key` entered with its primitive and the composition that failed:

- **Primitive.** Boundaries (Primitive 6), because this defines who a name is.
- **Failed composition.** `grant`, which is standing-gated rather than self-gated and has no signature semantics.

**Verification in core.** "What is in force" must have one answer, and it must not depend on which front end asked, so verification belongs in core. The cost is one dependency: `ed25519-compact` 2.6 with `default-features = false`. It is no_std, has no dependencies, and reads no file or socket, so it is consistent with `can-043cacac3c26`. Base64 decoding is about 25 lines and is written by hand. `core-boundary.sh`'s direct-dependency set and `APPROVED` list each gain one entry, with the justification the script asks for.

`ed25519-dalek` pulls in about six crates. Calling `ssh-keygen -Y verify` costs about 3.6 ms per act measured here; at about three folds per command, a 200-act canon spends about 2 s per command, and every reader would need `ssh-keygen`, Windows included. An injected verifier trait keeps core's dependencies unchanged but makes `Log::derive` without a verifier fail closed. That is a trap for the next consumer.

**Contract.** *Corrected in implementation:* this change classifies as `breaks_checked_contract`, not merely `not_established`, and it can't be written any other way. Two things break, both inherent:

- The baseline's `a_version_beyond_this_build_is_still_refused` asserts that a `v: 3` line is refused, and v3 is now read. The candidate's copy of that test moves to v4.
- The generated old consumer matches `ActKind` exhaustively, and `key` is a new variant.

The generated consumer only reads `Act`'s fields and never builds one, so `refs` and `sig` on `Act` are a surface change, not a break. Record bytes for unbound canons don't change: `Act::new` still writes `v: 2` and the new fields are skipped when empty. Checked by running the gate's own `observations` and `read_old_records` against the baseline binary and this one. All six observations are identical: decisions, minted bytes with ids, and old-record readings, for both fixtures. CONTRACT.md allows adopting a breaking change as a governance decision, provided old records are shown to read the same, and that comparison is the evidence. The new baseline is adopted through `CANON_CONTRACT_BASELINE`.

## What it stops, and what it can't

**Stops:**

- Writing an act under a bound name without that person's key: an env var, a hand-edited line, or a bad merge.
- Moving a signed approval onto different text through an id collision.
- Replaying a signed act into an unrelated canon.
- A backdated forgery under a bound name.
- A second binding for a name already bound: it contests the name and wins nothing.

**Can't stop, and says so:**

1. **A process that can use your unlocked key is you.** An agent running as your user can sign through an unencrypted key file or an `ssh-agent` that doesn't ask. For the euphemia threat this is the main residual, not a footnote. The protection is real only with a FIDO key that needs a touch (and the verifier checks the touch flag), a passphrase that isn't cached, or `ssh-add -c`. `canon identify` says this when the key isn't a FIDO key.
2. **Squatting a name that hasn't bound yet.** A squatter's binding controls what it vouches for. If you then bind too, the name is contested and nobody signs for it. The defence is to identify before your first act: `canon init`, then `canon identify`, and in euphemia the first `review`. A squat shows up as a refusal on your next write. Recovery is a new name, as with a lost key.
3. **Deleting or withholding lines.** A signature proves a line is genuine; it can't make a line exist. That remains git's job, with `canon guard` / `witness`, unchanged.
4. **A key holder backdating their own acts.** `ts_unix` still drives "first word wins" and the rest. Signing makes a backdate attributable, not impossible.
5. **References made before binding.** Acts vouched for in `prior` are pinned, but what they point at is only as firm as a 48-bit id.
6. **A binding pasted in from another canon.** It carries only its owner's real key, so nobody can sign as them with it. But their history here isn't in its `prior`, so it stops counting, and if they had bound here too, the name is contested. Someone with write access can do as much by deleting their lines; this way is at least louder.
7. **A bug in ed25519 edge cases that differs between implementations.** It only matters for keys the attacker owns. Pin strict RFC 8032 behaviour with test vectors.

SECURITY.md changes accordingly. "Anything reachable only by someone with write access is not a vulnerability" gains an exception for bound names, and "an act counting under a bound name without its key" joins the list of what we care about most.

## Considered and turned down

- **Leave it to the substrate.** Keep the silence: extend `canon guard` so that every `human:` actor added in a commit must match the commit's verified signer, using git's own `allowedSignersFile`. It is smaller: no format change, no core change, git does the cryptography. It fails euphemia on four counts. The commons remote has nothing that runs the gate. A commit is coarser than an act, and euphemia commits mix agent and person acts. Squash and rebase merges re-sign or strip signatures. A pasted file carries nothing. It is still worth doing for git-hosted canons, but separately.
- **Holder approval of each binding**, the way a non-holder's retract waits for a holder. It would block squatting of members. But approval needs `seat`, `seat` needs grants, and a grant's validity needs bindings. Breaking that loop means verification goes back inside the multi-pass fold, together with reverts. Deferred; binding first covers most of the gap.
- **Bindings that take effect from their timestamp.** Backdating defeats them; see §2.
- **Signing the id.** 48 bits; see §1.
- **A local verification cache.** Not needed at in-process speeds, and a cache another same-user process can write is something else to defend.
- **GPG, X.509, or a key format of our own.** People already have SSH keys, and git uses the same signature format.

## Footprint

| Area | Changes |
|---|---|
| Core | About 350 lines: canonical bytes, SSHSIG parse and verify, base64, the binding/`prior`/`refs` pre-pass, `Canon::unverified` |
| CLI | About 300 lines: signing and refusing in `cmds::write`, `canon identify`, finding the key in `~/.ssh` or the agent, reporting in `list`, `log` and MCP |
| Docs | A SPEC section; SECURITY.md; a GUIDE paragraph in the user's register ("make it so only you can sign as you") |
| Dependencies | One crate, no transitive dependencies |
| Euphemia | `canon identify` in the first `review`; accept v3 and the new canon version in the adapter; agents stay unbound labels |

## Pinned by

**Compatibility:**

- Every existing fixture folds identically, and the bytes minted by `replay --out` don't change. No bindings means no change.
- render∘parse keeps the canonical bytes for every known op, and the merge driver keeps `sig` and `refs`.

**Verification:**

- Signatures from real `ssh-keygen` (ed25519 and FIDO) verify in core.
- Flipping any byte of a signed line makes it unverified.
- A FIDO signature with the presence flag cleared is unverified.
- A canonical-bytes test vector covers control characters and non-ASCII text.

**Attacks:**

- `CANON_ACTOR=human:alex` against a bound alex: the act is carried, not applied, and listed. That includes a forged `grant` and a forged `ratification`.
- A forgery backdated before the binding is unverified, and history the binding vouched for still counts.
- A second binding, even one dated earlier, contests the name and nothing signed under it counts.
- A binding written in the same second as later acts still holds, whatever the ids' order.
- A hand-written colliding id behind a signed approval stops the approval counting.
- A signed act pasted into another canon fails, and a pasted binding never lets anyone sign as its owner.

**CLI:**

- A bound actor with no key, or the wrong key, is refused and nothing is appended.
- `init` then `identify` founds a canon whose every act is signed, with nothing configured.
- On a machine without the key, writes are refused until the key is in `~/.ssh`.

## Decisions

Taken: lost every key means a new name; verification lives in core with one dependency; one verb, `canon identify`, and one binding per name.

Still open, and for the holder of `canon` rather than for a PR:

1. **Supersede `can-ef55ca2a8f9c` and `can-50996b2caed2`** in `.canon/`, with the reasoning in "This reverses a recorded decision" or your own. An agent writing those acts under the holder's name would be the impersonation this design closes.
2. **Adopt the new contract baseline** through `CANON_CONTRACT_BASELINE` once the change is reviewed (see "Contract" above).

## Known costs found while building

- **One touch per act.** `canon add "…" --scope house` writes two acts, so with a FIDO key it asks for two touches. Euphemia's `review`, approving several proposals, asks for one touch per approval. Signing a batch under one touch would need one signature to cover several lines, which this format doesn't have.
- **`ssh-keygen` is needed to sign, not to read.** Every reader verifies in-process. Windows ships OpenSSH's `ssh-keygen`; the release builds for Windows, but CI never runs Windows, so signing there is untested.
