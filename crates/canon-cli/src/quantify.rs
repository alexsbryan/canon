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
use crate::resolver::{self, Offered, Resolver};

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
        (self.measure(), flat(&self.of))
    }

    /// Does this state a number at all?
    ///
    /// A reading pass sometimes answers with an entry whose value and unit
    /// are both blank — an object where the model had nothing to put. That
    /// states nothing, and every consumer here compares measures, so letting
    /// it through means "no number" gets compared as though it were one. It
    /// cost two of three candidates on a live smoke run: both Type "F"
    /// permits were refused for stating `` that their citation did not.
    fn states_a_number(&self) -> bool {
        !self.measure().is_empty()
    }

    /// Does this state an actual NUMBER — the guard's question, which is not
    /// the read-time question [`Quantity::states_a_number`] asks.
    ///
    /// [`unsupported`] exists to catch a rule that MISSTATES a number. A
    /// reading that names no number misstates nothing, and refusing it costs
    /// a real rule for a mismatch that was never numeric.
    ///
    /// **Measured 2026-08-24 on the maple-house bar.** The model returned
    /// `any number` as a quantity of "Overnight guests are not permitted in
    /// the house at any time for any number of nights", the citation's own
    /// reading produced no matching measure, and the guard refused the rule —
    /// costing T1's anchor. "Any" is a universal quantifier: it is the
    /// ABSENCE of a numeric limit, and reading it as a limit inverts what the
    /// rule says.
    ///
    /// **The test is a digit in the canonical form, and it is not a word
    /// list.** `SYSTEM` contracts canonical as "digits rather than number
    /// words", so a real quantity carries a digit there by construction. That
    /// matters: units are an OPEN set — dBA, dBC, fortnights, therms — which
    /// is exactly why the hand-kept unit list in `measure.rs` failed and was
    /// deleted on 2026-08-22. Digits are a closed set of ten (§2).
    ///
    /// An EMPTY canonical is not judged. `measure()` then falls back to the
    /// as-written value, which legitimately carries no digit ("twenty
    /// gallons"), so the guard abstains rather than refuse on a reading it
    /// cannot assess — absence reported, never defaulted (§18.3).
    fn is_numeric(&self) -> bool {
        let c = self.canonical.trim();
        c.is_empty() || c.chars().any(|ch| ch.is_ascii_digit())
    }

    /// The quantity as the rule wrote it: `two thirds`, `ten Days (Sundays
    /// excepted)`. Falls back to the canonical measure for a reading whose
    /// value and unit are both blank, which has nothing as-written to show.
    ///
    /// Named because [`unsupported`] both REPORTS this form and now looks for
    /// it in the text; two spellings of one form would let the guard refuse a
    /// rule while naming a string it never searched for.
    fn written(&self) -> String {
        let w = format!("{} {}", self.value, self.unit);
        if w.trim().is_empty() {
            self.measure()
        } else {
            w.trim().to_string()
        }
    }

    /// The measure alone, without what it measures.
    ///
    /// Falls back to the as-written value when no canonical form came back,
    /// so a missing field can only ever make two quantities look DIFFERENT —
    /// which keeps a rule out of a fold rather than silently into one.
    fn measure(&self) -> String {
        flat(&if self.canonical.trim().is_empty() {
            format!("{} {}", self.value, self.unit)
        } else {
            self.canonical.clone()
        })
    }
}

/// Case and spacing are noise; the words are not.
fn flat(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
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
    n: crate::model::Pos,
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

/// A rule's quantities and its citation's, read in one call.
pub type PairReading = (Vec<Quantity>, Vec<Quantity>);

/// Read two texts TOGETHER, so they are canonicalised against each other.
///
/// **Canonical form is only agreed within one call.** The prompt asks that
/// two texts stating the same limit produce the same canonical string, and
/// the model can only honour that for texts it can see at once. Reading a
/// rule in one call and its citation in another and then comparing the
/// results assumes a stability the design never offered — and it does not
/// hold: a live smoke run refused a permit for stating `9:00 a.m.` that its
/// own citation stated too, because one pass wrote `9:00 a.m.` and the other
/// `09:00`. Both readings were correct; only the comparison was wrong.
///
/// Pairs are independent of one another, so they still batch. The pairing is
/// structural rather than a parity trick on [`BATCH`]: each call carries
/// whole pairs because they are chunked as pairs.
pub fn quantify_pairs(
    client: &Client,
    pairs: &[(&str, &str)],
) -> Result<Vec<PairReading>, ModelError> {
    let mut out = Vec::with_capacity(pairs.len());
    for block in pairs.chunks(BATCH / 2) {
        let mut flat: Vec<&str> = Vec::with_capacity(block.len() * 2);
        for (a, b) in block {
            flat.push(a);
            flat.push(b);
        }
        let read = read_block(client, &flat)?;
        let mut it = read.into_iter();
        while let (Some(a), Some(b)) = (it.next(), it.next()) {
            out.push((a, b));
        }
    }
    Ok(out)
}

/// One reading call over at most [`BATCH`] texts, answers in the order given.
///
/// A text the model does not answer for comes back empty rather than missing,
/// so a caller can never silently line up the wrong answers.
/// The reading contract, named. See [`crate::resolver`].
pub struct Quantities;

impl Resolver for Quantities {
    /// No quantities. Cannot claim a disagreement, which is the refusing
    /// default this reader needs.
    type Reading = Vec<Quantity>;

    fn name(&self) -> &'static str {
        "quantities"
    }

    fn system(&self) -> &'static str {
        SYSTEM
    }

    fn schema(&self) -> Value {
        schema()
    }

    fn unread(&self, _index: usize) -> Vec<Quantity> {
        Vec::new()
    }
}

fn read_block(client: &Client, block: &[&str]) -> Result<Vec<Vec<Quantity>>, ModelError> {
    let offered = Offered::new(block.iter().map(|r| (*r).to_string()).collect(), "rule");
    let mut out = Quantities.blank(&offered);
    let got: Quantified = resolver::ask(
        client,
        &Quantities,
        &offered,
        "Rules:",
        "List the quantities each rule states.",
    )?;
    for mut r in got.rules {
        let Some(at) = r.n.get().and_then(|n| offered.at(n)) else {
            continue;
        };
        // Filtered here, at the one place a model's answer becomes data,
        // rather than defended against by each consumer.
        r.quantities.retain(Quantity::states_a_number);
        out[at] = r.quantities;
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

/// A quantity the rule states that its own citation does not.
///
/// The citation proves the WORDS are the passage's; it does not prove the
/// RULE matches them. Observed against a live endpoint: a candidate read "at
/// least three hours in advance" while its verbatim quote said "three days
/// ahead". A commitment that misstates a number is worse than a missing one —
/// it contradicts the sentence printed beneath it, and the citation makes it
/// look checked.
///
/// Compared on the MEASURE alone, never on what it measures. A rule may
/// reword what a number counts — "within any seven-day period" as "per week"
/// — and that is a paraphrase, not a different rule. A number the citation
/// never states is not a paraphrase of anything.
///
/// This was a hand-kept list of units and number words in `measure.rs` until
/// 2026-08-22. On a municipal noise code that list read `85 dBA` and `85 dBC`
/// as stating no measure at all, so the guard could not fire in either
/// direction. Both sides are read by the model now and compared here as
/// structure — the same reading the fold guard uses, so there is one answer
/// to "what does this state" (§10.6).
pub fn unsupported(
    rule: &[Quantity],
    cited: &[Quantity],
    rule_text: &str,
    cited_text: &str,
) -> Option<String> {
    let have: Vec<String> = cited.iter().map(Quantity::measure).collect();
    let rule_words = flat(rule_text);
    let cited_words = flat(cited_text);
    rule.iter()
        .filter(|q| q.is_numeric())
        // What the READING says: the citation carries no matching measure.
        .filter(|q| !have.contains(&q.measure()))
        // What the TEXT says. A reading is one model call and it can be wrong
        // in both directions; these two facts are not, and a guard that
        // refuses a rule against a citation the reader misread is refusing on
        // no evidence at all (§18.3, §11).
        //
        // **Measured on the founding corpus, 2026-08-26.** All eight refusals
        // in the 2026-08-26 sweep were false. Five named a quantity present
        // VERBATIM in the citation the reader had just been shown alongside
        // the rule — `annually`, `two thirds`, `two`, `three`, `ten Days
        // (Sundays excepted)`. One named `1` for "Electors shall meet in
        // their respective states and vote by ballot", a rule that states no
        // number at all: the reading invented it, and the rule was refused
        // for a claim it never made. Two of the eight carried the anchors for
        // planted supersessions S2 and S8.
        .find(|q| {
            let w = flat(&q.written());
            // A rule cannot misstate a number it does not visibly state, and
            // a citation that carries the words carries the number.
            !w.is_empty() && rule_words.contains(&w) && !cited_words.contains(&w)
        })
        // Name what was COMPARED. Reporting `value unit` printed an empty
        // string for a reading whose value and unit were blank but whose
        // canonical form was not — so the refusal said the rule stated ``,
        // and the mismatch it was actually about was invisible.
        .map(Quantity::written)
}

#[cfg(test)]
mod tests;
