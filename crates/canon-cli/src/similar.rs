// SPDX-License-Identifier: AGPL-3.0-or-later
//! "This looks like something already here" — a warning at `add`, never a
//! decision.
//!
//! The group's two guest rules were the same decision written twice: one
//! proposed, one already proposed, each one approval short. Nothing said so.
//! A model could judge sameness, but `add` must work with no endpoint, so
//! this is a deterministic floor: content words, a trailing `s`, and
//! containment. It is deliberately conservative — three shared words and
//! four-fifths of the shorter text — because a false warning costs a line
//! and a missed one costs a duplicate rule.

use std::collections::BTreeSet;

use canon_core::{Canon, Status};

/// A commitment the new text looks like.
pub struct Like<'a> {
    pub id: &'a str,
    pub text: &'a str,
    pub status: &'static str,
}

/// The closest live or proposed commitment, when the new text contains most
/// of it (or it contains most of the new text).
pub fn nearest<'a>(canon: &'a Canon, text: &str) -> Option<Like<'a>> {
    let new = content(text);
    if new.len() < 3 {
        return None;
    }
    let mut best: Option<(usize, Like<'a>)> = None;
    for c in canon.commitments() {
        let status = match c.status() {
            Status::Active => "live",
            Status::Proposed { .. } => "proposed",
            _ => continue,
        };
        let old = content(c.text());
        if old.len() < 3 {
            continue;
        }
        let shared = new.intersection(&old).count();
        let smaller = new.len().min(old.len());
        if shared < 3 || shared * 5 < smaller * 4 {
            continue;
        }
        if best.as_ref().is_none_or(|(most, _)| shared > *most) {
            best = Some((
                shared,
                Like {
                    id: c.id().as_str(),
                    text: c.text(),
                    status,
                },
            ));
        }
    }
    best.map(|(_, like)| like)
}

/// The words a rule is about: lowercased, a trailing `s` dropped, and the
/// words that only hold a sentence together removed. `guests`/`guest` and
/// `stays`/`stay` are one word here; `no` and `not` are not, because they
/// turn a duplicate into its opposite.
fn content(text: &str) -> BTreeSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| {
            let w = w.to_lowercase();
            if w.len() > 3 && w.ends_with('s') && !w.ends_with("ss") {
                w[..w.len() - 1].to_string()
            } else {
                w
            }
        })
        .filter(|w| !STOP.contains(&w.as_str()))
        .collect()
}

const STOP: &[&str] = &[
    "a", "an", "the", "and", "or", "but", "if", "then", "than", "that", "this", "these", "those",
    "is", "are", "was", "were", "be", "been", "being", "of", "to", "in", "on", "at", "by", "for",
    "with", "from", "as", "it", "its", "we", "us", "our", "you", "your", "they", "them", "their",
    "he", "she", "his", "her", "i", "my", "me", "do", "does", "did", "done", "have", "has", "had",
    "can", "could", "may", "might", "must", "shall", "should", "will", "would", "about", "after",
    "before", "over", "under", "between", "into", "out", "up", "down", "off", "again", "once",
    "here", "there", "when", "where", "why", "how", "all", "any", "both", "each", "few", "more",
    "other", "some", "such", "only", "own", "same", "so", "too", "very", "just",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_guest_rules_are_the_same_words() {
        let a = content(
            "Guests stay at most three nights, unless the house agrees otherwise in advance.",
        );
        let b = content("A guest stays three nights at most.");
        assert!(b.is_subset(&a), "{b:?} is not within {a:?}");
    }

    #[test]
    fn an_opposite_rule_is_not_a_duplicate() {
        let a = content("No pets in the shared kitchen.");
        let b = content("Pets are allowed in the shared kitchen.");
        assert!(!b.is_subset(&a));
    }

    #[test]
    fn a_different_room_is_not_a_duplicate() {
        let a = content("Guests stay at most three nights.");
        let b = content("No smoking indoors.");
        assert!(a.intersection(&b).count() < 3);
    }
}
