// SPDX-License-Identifier: AGPL-3.0-or-later
//! Do these rules govern the same thing?
//!
//! The fold's second structural guard, and the one that was missing.
//! [`crate::quantify`] stops two rules being folded when they put DIFFERENT
//! numbers on the same thing. Nothing stopped two rules being folded when
//! they put the SAME number on different things — and on a municipal noise
//! code that is the common case, because a permit schedule restates one
//! sentence per permit type:
//!
//! > A type "B" permit may be used for sound equipment emitting music or
//! > human speech registering not more than 65 dBAs …
//! > A type "C" permit may be issued for sound equipment emitting music or
//! > human speech registering not more than 65 dBAs …
//!
//! Identical limits, identical wording, different permits. The reduce step
//! proposed them as duplicates, the quantity guard had no grounds to refuse,
//! and the type "C" commitment was deleted. Every run of the Des Moines bar
//! measured 10 of 11 planted tensions reachable with extraction missing none.
//!
//! Like `quantify` this is a RESOLVER, not a lexicon. Nothing here knows what
//! a permit is. The model reads, the code decides, and no vocabulary is
//! maintained anywhere.
//!
//! **The model partitions; it does not name.** The first cut of this asked
//! for each rule's subject as a string and compared the strings. That fails
//! on the ordinary case it must not break: Maple House states one smoking
//! rule twice, and the readings came back `Smoking in house areas` and
//! `Smoking in interior of house` — two strings, one thing, fold refused.
//! The prompt was asking for the subject "in the rule's own words" AND for
//! the same words across rules, which are contradictory instructions and the
//! kind small models handle worst. Asking instead which rules govern the same
//! thing keeps the reading inside one call, where a model can actually
//! compare, and leaves code holding integers rather than prose.

use serde::Deserialize;
use serde_json::{json, Value};

use crate::model::{Client, ModelError};
use crate::resolver::{self, Offered, Resolver};

const SYSTEM: &str = "\
You decide which rules govern the same thing.

The thing a rule governs is who or what it applies to, and where. Two rules \
govern the same thing even when they are worded differently or one is more \
detailed. Two rules govern different things when they apply to a different \
permit, licence, class, zone, room, party or period — however alike they read.

For each rule return `same_as`: the SMALLEST number of any rule governing the \
same thing, or its own number when no earlier rule does.";

#[derive(Debug, Deserialize)]
struct Partitioned {
    #[serde(default)]
    rules: Vec<RuleSame>,
}

#[derive(Debug, Deserialize)]
struct RuleSame {
    /// 1-based position in the group, so a dropped or reordered answer is
    /// detectable rather than silently misattributed.
    n: usize,
    same_as: usize,
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
                        "same_as": { "type": "integer" },
                    },
                    "required": ["n", "same_as"],
                    "additionalProperties": false,
                },
            },
        },
        "required": ["rules"],
        "additionalProperties": false,
    })
}

/// Partition each proposed group by what its rules govern.
///
/// Returns, per group, one representative POSITION per member: members
/// sharing a representative govern the same thing. A member the model did not
/// answer for, or answered out of range, represents only itself — which
/// refuses its fold rather than allowing one.
///
/// **One call per group, never a shared one.** `same_as` is a position within
/// the call, so packing two groups into one call invites a member of one to
/// name a member of the other. Groups are the reduce step's own proposals and
/// are small; this is cheap, and it is the shape that was measured.
pub fn same_thing(client: &Client, groups: &[Vec<&str>]) -> Result<Vec<Vec<usize>>, ModelError> {
    let mut out = Vec::with_capacity(groups.len());
    for g in groups {
        out.push(partition(client, g)?);
    }
    Ok(out)
}

/// The reading contract, named. See [`crate::resolver`].
pub struct SameThing;

impl Resolver for SameThing {
    /// The index of the member that represents this one.
    type Reading = usize;

    fn name(&self) -> &'static str {
        "same"
    }

    fn system(&self) -> &'static str {
        SYSTEM
    }

    fn schema(&self) -> Value {
        schema()
    }

    /// Represents itself, which refuses the fold. The refusing default this
    /// reader needs: anything the model leaves unsaid must not merge.
    fn unread(&self, index: usize) -> usize {
        index
    }
}

fn partition(client: &Client, group: &[&str]) -> Result<Vec<usize>, ModelError> {
    let offered = Offered::new(group.iter().map(|r| (*r).to_string()).collect(), "rule");
    let mut rep = SameThing.blank(&offered);
    if group.len() < 2 {
        return Ok(rep);
    }
    let got: Partitioned = resolver::ask(
        client,
        &SameThing,
        &offered,
        "Rules:",
        "For each rule, which is the smallest-numbered rule governing the same thing?",
    )?;
    for r in got.rules {
        let Some(at) = offered.at(r.n) else { continue };
        // A representative past the end, or past this member, names nothing
        // the model was shown. Left as itself, which refuses the fold.
        if let Some(to) = r.same_as.checked_sub(1).filter(|i| *i <= at) {
            rep[at] = to;
        }
    }
    Ok(rep)
}

#[cfg(test)]
mod tests;
