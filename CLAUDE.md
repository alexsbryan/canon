# Working in canon

`./scripts/pre-push.sh` is the gate: rustfmt, clippy, the suite, the contract
check, docs links and the ledger. Run it before you say a change is done.

## The contract

canon promises other implementations and every existing record that it keeps
the laws in [docs/CONTRACT.md](docs/CONTRACT.md). Every change is judged
against one adopted commit, the standard in `.github/contract-baseline`.

- Anything under `crates/canon-core/` or `crates/canon-cli/`, the manifests,
  `docs/CONTRACT.md` and `docs/SPEC.md` is judged. Run
  `python3 scripts/contract-check.py` before committing such a change, not
  after CI is red. About 40 seconds warm.
- Only `preserves_checked_contract` and `extends_checked_contract` pass. A new
  public enum variant, a changed field, a new or reworded law, a SPEC.md edit,
  or different minted record bytes is a break or not established, even when
  the feature asked for it.
- When the check does not pass, stop and tell the user what it found. Do not
  move `.github/contract-baseline`, set `CANON_CONTRACT_BASELINE`, or reshape
  the change to dodge the check. Adopting a new standard is the holder's
  decision, made in the order [docs/CONTRACT_ADOPTIONS.md](docs/CONTRACT_ADOPTIONS.md)
  describes.

Hooks in `.claude/settings.json` hold these lines. They are there because
0.5.0 was adopted backwards; CONTRACT_ADOPTIONS.md has the account.
