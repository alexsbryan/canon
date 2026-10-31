// SPDX-License-Identifier: AGPL-3.0-or-later
//! What the fixtures decide and the bytes they mint, held to contract/records/.
//!
//! A record is append-only and other implementations read it, so the meaning
//! of one that exists cannot move unnoticed: a change to a decision or to a
//! minted act (its id included) is a diff in contract/records/, made with
//! `CANON_BLESS=1 cargo test`.

use std::path::Path;
use std::process::Command;

fn canon(args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_canon"))
        .args(args)
        .output()
        .expect("canon runs");
    assert!(
        out.status.success(),
        "canon {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn the_fixtures_decide_and_mint_what_is_checked_in() {
    for name in ["fernwood-commons", "eleven-principles"] {
        let fixture = canon_contract::root().join("fixtures").join(name);
        let fixture = fixture.to_str().unwrap();

        // The decisions, without the footer that times the run.
        let said = canon(&["replay", fixture, "--json"]);
        let decisions: serde_json::Value = serde_json::Deserializer::from_str(&said)
            .into_iter()
            .next()
            .expect("a JSON reading")
            .expect("it parses");
        canon_contract::golden(
            &format!("records/{name}.decisions.json"),
            &(serde_json::to_string_pretty(&decisions).unwrap() + "\n"),
        );

        let out =
            std::env::temp_dir().join(format!("canon-contract-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&out);
        canon(&["replay", fixture, "--out", out.to_str().unwrap()]);
        let acts = std::fs::read_to_string(Path::new(&out).join("acts.jsonl")).unwrap();
        canon_contract::golden(&format!("records/{name}.minted.jsonl"), &acts);
        let _ = std::fs::remove_dir_all(&out);
    }
}
