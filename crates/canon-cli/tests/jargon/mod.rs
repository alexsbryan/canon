// SPDX-License-Identifier: AGPL-3.0-or-later
//! The words a high school government student has not met, kept out of what
//! canon prints. One list and one reading of it, shared by the help test in
//! `main.rs` and the scripted session in `founding_bar.rs`.
//!
//! **What a person types is not prose.** A `--flag`, a span in backticks
//! (a command, a rule spelled the way it is set) and `ratification` as the
//! verb's own name are spellings, like the op names in the record, and stay
//! as they are; every other word on the line is read.

/// Stems, so `commitments`, `ratified`, `policies` and `retraction` count.
pub const JARGON: &[&str] = &[
    "commitment",
    "scope",
    "standing",
    "holder",
    "superse",
    "retract",
    "adjudicat",
    "ratif",
    "polic",
    "horizon",
    "tension",
    "lineage",
    "provenance",
    "subsidiarity",
    "entrench",
    "graduated",
];

/// Each jargon word in `text`, with the line it is on.
pub fn found(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        // Outside the backticks: the even pieces of a split on them.
        let read: String = line.split('`').step_by(2).collect::<Vec<_>>().join(" ");
        let mut before = "";
        for (n, token) in read.split_whitespace().enumerate() {
            let flag = token.trim_start_matches(['[', '(']).starts_with("--");
            let verb = token == "ratification" && (n == 0 || ["canon", "help"].contains(&before));
            before = token;
            if flag || verb {
                continue;
            }
            for word in token.split(|c: char| !c.is_alphanumeric()) {
                let word = word.to_lowercase();
                if let Some(stem) = JARGON.iter().find(|s| word.starts_with(*s)) {
                    out.push(format!("`{stem}` in: {}", line.trim()));
                }
            }
        }
    }
    out
}
