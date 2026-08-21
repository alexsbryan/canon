// SPDX-License-Identifier: AGPL-3.0-or-later
//! Lineage tests. The merge driver gets a real two-checkout exercise,
//! because "both sides fold to the same canon" is the property the whole
//! git story rests on and it is not obvious from reading the code.

use canon_core::{Act, ActKind, Log};

use super::*;

fn asserted(text: &str, ts: i64, actor: &str) -> Act {
    Act::new(
        ActKind::Assert {
            text: text.into(),
            from: None,
            source: None,
        },
        ts,
        actor,
    )
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("canon-lineage-{name}"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

use std::path::PathBuf;

#[test]
fn a_generation_is_split_off_a_url_without_eating_an_scp_style_host() {
    assert_eq!(
        split_generation("https://example.com/house.git@v3"),
        ("https://example.com/house.git", Some("v3"))
    );
    assert_eq!(
        split_generation("https://example.com/house.git"),
        ("https://example.com/house.git", None)
    );
    // `git@host:path` is a URL, not a generation.
    assert_eq!(
        split_generation("git@example.com:dana/house.git"),
        ("git@example.com:dana/house.git", None)
    );
}

#[test]
fn the_merge_driver_unions_two_divergent_appends() {
    // Two people append on two machines. Git sees a textual conflict where
    // semantically there is none.
    let dir = scratch("merge");
    let base = Log::from_acts(vec![asserted("quiet hours at 11", 100, "human:dana")]);
    let mut ours = base.clone();
    ours.push(asserted("guests two nights", 200, "human:dana"));
    let mut theirs = base.clone();
    theirs.push(asserted("kitchen cleaned by the cook", 200, "human:sam"));

    let (o, a, b) = (dir.join("O"), dir.join("A"), dir.join("B"));
    std::fs::write(&o, base.render()).unwrap();
    std::fs::write(&a, ours.render()).unwrap();
    std::fs::write(&b, theirs.render()).unwrap();

    let args: Vec<String> = [&o, &a, &b]
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    assert_eq!(merge_driver(&args), 0);

    let merged = Log::parse(&std::fs::read_to_string(&a).unwrap()).unwrap();
    assert_eq!(merged.len(), 3, "both appends survive");
    assert_eq!(merged.derive().active().count(), 3);
}

#[test]
fn the_merged_file_is_byte_identical_whichever_side_ran_the_merge() {
    // The property that makes the driver usable at all: two machines that
    // merge the same pair produce the same bytes, so the next diff is empty.
    let dir = scratch("merge-symmetric");
    let base = Log::from_acts(vec![asserted("quiet hours at 11", 100, "human:dana")]);
    let mut ours = base.clone();
    ours.push(asserted("guests two nights", 200, "human:dana"));
    let mut theirs = base.clone();
    theirs.push(asserted("kitchen cleaned by the cook", 200, "human:sam"));

    let run = |name: &str, a: &Log, b: &Log| -> String {
        let (op, ap, bp) = (
            dir.join(format!("{name}-O")),
            dir.join(format!("{name}-A")),
            dir.join(format!("{name}-B")),
        );
        std::fs::write(&op, base.render()).unwrap();
        std::fs::write(&ap, a.render()).unwrap();
        std::fs::write(&bp, b.render()).unwrap();
        let args: Vec<String> = [&op, &ap, &bp]
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        assert_eq!(merge_driver(&args), 0);
        std::fs::read_to_string(&ap).unwrap()
    };
    assert_eq!(run("x", &ours, &theirs), run("y", &theirs, &ours));
}

#[test]
fn the_driver_survives_the_empty_base_git_passes_for_a_file_added_on_both_sides() {
    let dir = scratch("merge-empty-base");
    let (o, a, b) = (dir.join("O"), dir.join("A"), dir.join("B"));
    std::fs::write(&o, "").unwrap();
    std::fs::write(
        &a,
        Log::from_acts(vec![asserted("a", 100, "human:dana")]).render(),
    )
    .unwrap();
    std::fs::write(
        &b,
        Log::from_acts(vec![asserted("b", 100, "human:sam")]).render(),
    )
    .unwrap();
    let args: Vec<String> = [&o, &a, &b]
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    assert_eq!(merge_driver(&args), 0);
    assert_eq!(
        Log::parse(&std::fs::read_to_string(&a).unwrap())
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn the_driver_refuses_a_corrupt_side_rather_than_silently_dropping_it() {
    // Exit 1 leaves git's conflict markers in place and tells the person.
    // Writing a "merged" file missing half the acts would lose law silently.
    let dir = scratch("merge-corrupt");
    let (o, a, b) = (dir.join("O"), dir.join("A"), dir.join("B"));
    std::fs::write(&o, "").unwrap();
    std::fs::write(
        &a,
        Log::from_acts(vec![asserted("a", 100, "human:dana")]).render(),
    )
    .unwrap();
    std::fs::write(&b, "<<<<<<< HEAD\nnot json at all\n").unwrap();
    let args: Vec<String> = [&o, &a, &b]
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    assert_eq!(merge_driver(&args), 1);
}

#[test]
fn merge_driver_with_no_arguments_prints_its_own_setup() {
    // The one place a person meets git in this tool, so it explains itself
    // rather than failing with a usage line.
    assert_eq!(merge_driver(&[]), 0);
}

#[test]
fn a_url_becomes_a_stable_and_unambiguous_cache_directory_name() {
    let s = slug("https://example.com/dana/house.git");
    assert!(s.starts_with("https---example-com-dana-house-git-"), "{s}");
    assert_eq!(s, slug("https://example.com/dana/house.git"), "stable");
    assert_ne!(slug("https://a.com/house"), slug("https://b.com/house"));
    // The readable half is lossy; the digest is what keeps two lineages out
    // of one cache directory.
    assert_ne!(slug("https://x.com/a.b"), slug("https://x.com/a-b"));
}
