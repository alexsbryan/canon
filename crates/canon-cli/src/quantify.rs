// SPDX-License-Identifier: AGPL-3.0-or-later
//! What quantities does a rule state?
//!
//! **A resolver, not a lexicon.** The question "do these two rules state the
//! same limit" is a reading task, and [`crate::measure`] answered it with a
//! hand-kept list of units, number words and clock formats. That list is
//! fitted to the documents it was written against, and it fails the moment a
//! real one arrives: measured on a municipal noise code, `85 dBA` and
//! `85 dBC` both parsed as stating NO measure, the fold guard had no grounds
//! to refuse, and five planted supersessions were deleted at the reduce step.
//! Adding `dba` to the list would fix that corpus and nothing else — the next
//! document says degrees, or lux, or lumens, or acre-feet.
//!
//! So the model reads and the code decides. One narrow question — *list the
//! quantities in this sentence* — with a typed answer, which is the kind of
//! task a 4B model does as well as a 27B. Everything downstream compares
//! STRUCTURE, and structure needs no vocabulary.
//!
//! Rules are quantified in batches because each rule's quantities are
//! independent of every other rule's. That is the opposite of comparison,
//! where the whole difficulty is that N² pairs compete for one window.

use serde::Deserialize;
use serde_json::{json, Value};

use crate::model::{Client, ModelError};

/// One quantity a rule states.
///
/// `of` is what the number counts or limits, in the rule's own words. It is
/// what stops "no more than 2 guests" and "no more than 2 nights" from
/// comparing equal, without anyone maintaining a list of the nouns a canon
/// might be about.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Quantity {
    /// The number as written: `85`, `10:00 PM`, `twenty-five`.
    pub value: String,
    /// The unit as written: `dBC`, `feet`, `dollars`, `nights`. Empty when
    /// the rule states a bare count.
    #[serde(default)]
    pub unit: String,
    /// What it measures: `sound level`, `guest stay`, `late fee`.
    #[serde(default)]
    pub of: String,
    /// The same quantity in a standard form, so two spellings of one limit
    /// compare equal.
    ///
    /// `11 PM` and `eleven at night` are one instant and must not read as a
    /// disagreement. Normalising is itself a reading task — the alternative
    /// is a table of number words and clock formats, which is the thing this
    /// module exists to delete.
    #[serde(default)]
    pub canonical: String,
}

impl Quantity {
    /// Comparable form. Case and spacing are noise; the words are not.
    ///
    /// Falls back to the as-written value when no canonical form came back,
    /// so a missing field can only ever make two quantities look DIFFERENT —
    /// which keeps a rule out of a fold rather than silently into one.
    fn key(&self) -> (String, String) {
        let flat = |s: &str| {
            s.split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase()
        };
        let c = if self.canonical.trim().is_empty() {
            format!("{} {}", self.value, self.unit)
        } else {
            self.canonical.clone()
        };
        (flat(&c), flat(&self.of))
    }
}

const SYSTEM: &str = "\
You list the quantities a rule states.

A quantity is any number, time, or limit the rule puts on something: a sound \
level, a clock time, a count, a duration, a distance, a fee, a temperature.

For each one return:
- value: the number exactly as the rule writes it
- unit: the unit exactly as the rule writes it, or \"\" if it states none
- of: what the quantity measures, in three words or fewer
- canonical: the same quantity in a standard form — a 24-hour clock time \
like 23:00, digits rather than number words, a singular unit

Rules:
- Two rules stating the SAME limit must produce the same canonical string. \
\"11 PM\" and \"eleven at night\" are both 23:00.
- Copy the value and unit as written. Do not convert, round, or normalise.
- A weighting or qualifier attached to a unit is part of the unit: dBA and \
dBC are different units.
- A rule stating no quantity returns an empty list.";

#[derive(Debug, Deserialize)]
struct Quantified {
    #[serde(default)]
    rules: Vec<RuleQuantities>,
}

#[derive(Debug, Deserialize)]
struct RuleQuantities {
    /// 1-based position in the batch, so a dropped or reordered answer is
    /// detectable rather than silently misattributed.
    n: usize,
    #[serde(default)]
    quantities: Vec<Quantity>,
}

fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "rules": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "n": { "type": "integer" },
                        "quantities": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "value": { "type": "string" },
                                    "unit": { "type": "string" },
                                    "of": { "type": "string" },
                                    "canonical": { "type": "string" },
                                },
                                "required": ["value", "unit", "of"],
                                "additionalProperties": false,
                            },
                        },
                    },
                    "required": ["n", "quantities"],
                    "additionalProperties": false,
                },
            },
        },
        "required": ["rules"],
        "additionalProperties": false,
    })
}

/// How many independent rules go into one reading pass.
///
/// Independent work batches; comparative work does not. Each rule's
/// quantities depend on nothing but that rule, so this is a throughput knob
/// and not the attention cliff `tensions::BATCH` is guarding.
const BATCH: usize = 10;

/// The quantities each rule states, in the order given.
///
/// A rule the model does not answer for comes back empty rather than
/// missing, so a caller can never silently line up the wrong answers.
pub fn quantify(client: &Client, rules: &[&str]) -> Result<Vec<Vec<Quantity>>, ModelError> {
    let mut out: Vec<Vec<Quantity>> = vec![Vec::new(); rules.len()];
    for (b, block) in rules.chunks(BATCH).enumerate() {
        let mut user = String::from("Rules:\n");
        for (i, r) in block.iter().enumerate() {
            user.push_str(&format!("{}. {}\n", i + 1, r));
        }
        user.push_str("\nList the quantities each rule states.");
        let got: Quantified = client.complete_json(SYSTEM, &user, "quantities", &schema())?;
        for r in got.rules {
            if r.n >= 1 && r.n <= block.len() {
                out[b * BATCH + r.n - 1] = r.quantities;
            } else {
                eprintln!(
                    "\nwarning: dropped quantities for rule {} — only 1..{} were offered",
                    r.n,
                    block.len()
                );
            }
        }
    }
    Ok(out)
}

/// Do these two rules state different quantities for the same thing?
///
/// Pure code over structure the model produced. Two rules disagree when they
/// measure the same thing and put different numbers on it — which is a
/// contradiction, never a duplicate, and is the fold that
/// `unmarked supersession` depends on surviving.
///
/// Only fires when BOTH state quantities. A rule with no numbers may still
/// be a duplicate of one that has them.
pub fn differs_by_quantity(a: &[Quantity], b: &[Quantity]) -> bool {
    if a.is_empty() || b.is_empty() {
        return false;
    }
    for qa in a {
        for qb in b {
            let (ca, oa) = qa.key();
            let (cb, ob) = qb.key();
            if oa == ob && ca != cb {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests;
