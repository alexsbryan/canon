// SPDX-License-Identifier: AGPL-3.0-or-later
//! Format v3 reads every record written before it exactly as before.
//!
//! CONTRACT.md: "A breaking adoption needs old-record migration or
//! reinterpretation tests in the new standard." Signed acts break the
//! checked contract — the format reads `v: 3`, and `ActKind` gained `Key` —
//! so this is that test. There is no migration, because there is nothing to
//! migrate: a record with no `key` act is not reinterpreted at all.
//!
//! Two bodies of old records. This repository's own `.canon/acts.jsonl` as
//! it stood before signing, frozen in `fixtures/pre-signing/` because the
//! live one will carry signed lines once its holder identifies. And every
//! replayable fixture, minted by this build, which pins the other half:
//! somebody who never identifies writes the bytes an older canon wrote.

use std::path::{Path, PathBuf};
use std::process::Command;

use canon_core::sign::{self, Verified};
use canon_core::{Act, Log, UNSIGNED_VERSION};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every fixture `canon replay` can mint a record from.
fn fixtures() -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut dirs = vec![repo().join("fixtures")];
    while let Some(d) = dirs.pop() {
        for e in std::fs::read_dir(&d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                if p.join("scenario.jsonl").is_file() {
                    found.push(p.clone());
                }
                dirs.push(p);
            }
        }
    }
    found.sort();
    found
}

/// What reinterpretation would look like, checked line by line: nothing
/// set aside, nothing that would be signed, and each act reminted by this
/// build to the bytes it was read from.
fn reads_as_before(name: &str, text: &str) {
    let log = Log::parse(text).unwrap_or_else(|e| panic!("{name}: {e}"));
    assert!(!log.is_empty(), "{name}: an empty record proves nothing");
    assert_eq!(
        sign::verify(log.acts()),
        Verified::default(),
        "{name}: a record with no key act has nothing set aside"
    );
    assert!(log.derive().unverified().is_empty(), "{name}");
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let act: Act = serde_json::from_str(line).unwrap();
        assert!(act.v <= UNSIGNED_VERSION, "{name}: {line}");
        assert!(act.sig.is_none() && act.refs.is_empty(), "{name}: {line}");
        // At the line's own version: this repository still holds v1 lines,
        // which v2 already read without rewriting.
        let mut again = Act::new(act.kind.clone(), act.ts_unix, act.actor.clone());
        again.v = act.v;
        assert_eq!(
            serde_json::to_string(&again).unwrap(),
            line,
            "{name}: reminted differently"
        );
    }
    assert_eq!(
        Log::parse(&log.render()).unwrap().render(),
        log.render(),
        "{name}"
    );
}

#[test]
fn this_repositorys_own_record_reads_as_it_did_before_signing() {
    let text = std::fs::read_to_string(repo().join("fixtures/pre-signing/acts.jsonl")).unwrap();
    reads_as_before("fixtures/pre-signing/acts.jsonl", &text);
}

#[test]
fn every_fixture_mints_and_reads_as_it_did_before_signing() {
    let out = std::env::temp_dir().join(format!("canon-reinterpretation-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    let all = fixtures();
    assert!(all.len() >= 17, "only {} fixtures found", all.len());
    for fixture in all {
        let name = fixture
            .strip_prefix(repo())
            .unwrap_or(&fixture)
            .display()
            .to_string();
        let record = out.join(name.replace('/', "_"));
        let minted = Command::new(env!("CARGO_BIN_EXE_canon"))
            .arg("replay")
            .arg(&fixture)
            .arg("--out")
            .arg(&record)
            .env_remove("CANON_DIR")
            .env_remove("CANON_ACTOR")
            .output()
            .unwrap();
        assert!(
            minted.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&minted.stderr)
        );
        let text = std::fs::read_to_string(record.join("acts.jsonl")).unwrap();
        reads_as_before(&name, &text);
    }
}
