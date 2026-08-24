// SPDX-License-Identifier: AGPL-3.0-or-later
//! The resolver contract: **text in, typed evidence out, never a verdict.**
//!
//! A resolver reads open text and returns structure. Code compares the
//! structure and decides. `locate` returns a POSITION and code cuts the quote;
//! `quantify` returns quantities and code compares canonical forms; `subject`
//! returns a PARTITION and code compares integers. Three modules arrived at
//! that independently, each after a failure that came from letting a model
//! hold the decision — so it is written down here once, and the fourth module
//! inherits it instead of paying for it again.
//!
//! **The failure it prevents:** a model asked to *guarantee* something will
//! sometimes fail to, plausibly and without saying so. Structure it produces
//! can be checked; a verdict it produces cannot. Every measured regression in
//! this tool's extraction path traces to the same shape — asking for a
//! promise instead of a reading (§7.6).
//!
//! ## The numbering discipline
//!
//! Every resolver here shows the model a numbered list and takes numbers back.
//! That coordinate system is what makes an answer checkable at all: "is 7 one
//! of the things I offered?" is a question code can settle, and "is this quote
//! really in the passage?" is not.
//!
//! [`Offered`] owns both halves — rendering the list, and turning a number
//! back into an index. **It never clamps.** An out-of-range answer is dropped
//! and counted, because clamping attributes a reading to whichever item
//! happened to be last, which produces a confident citation of the wrong rule
//! (§18.3). Four places wrote that check by hand before this existed, and one
//! of them wrote it slightly differently.

use std::cell::Cell;

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::model::{Client, ModelError};

/// How the list is marked in the prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    /// `1. text` — the plain form, for lists of rules.
    Dot,
    /// `[1] text` — for prose whose own numbering (`(1)`, `a.`, `1.`) would
    /// otherwise be confusable with the coordinate system laid over it.
    Bracket,
}

/// The items a resolver showed the model, and the only answers it will take.
pub struct Offered {
    items: Vec<String>,
    what: &'static str,
    mark: Mark,
    flatten: bool,
    refused: Cell<usize>,
}

impl Offered {
    /// `what` names the items in the warning a refusal prints — "rule",
    /// "commitment", "sentence". It appears in output a person reads.
    pub fn new(items: Vec<String>, what: &'static str) -> Self {
        Self {
            items,
            what,
            mark: Mark::Dot,
            flatten: false,
            refused: Cell::new(0),
        }
    }

    /// Mark with `[n]` instead of `n.`.
    pub fn marked(mut self, mark: Mark) -> Self {
        self.mark = mark;
        self
    }

    /// Collapse each item onto one line for the prompt.
    ///
    /// The prompt only; whatever the item is cut from keeps its own line
    /// breaks, because the citation is taken from the source and not from
    /// what the model was shown.
    pub fn flattened(mut self) -> Self {
        self.flatten = true;
        self
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The numbered list, one item per line.
    pub fn numbered(&self) -> String {
        let mut out = String::new();
        for (i, item) in self.items.iter().enumerate() {
            let body = if self.flatten {
                item.split_whitespace().collect::<Vec<_>>().join(" ")
            } else {
                item.clone()
            };
            match self.mark {
                Mark::Dot => out.push_str(&format!("{}. {body}\n", i + 1)),
                Mark::Bracket => out.push_str(&format!("[{}] {body}\n", i + 1)),
            }
        }
        out
    }

    /// A 1-based number from the model, as an index into what was offered.
    ///
    /// **Never clamps, and says so out loud when it refuses.** Clamping would
    /// attribute the reading to whichever item happened to be last, which is
    /// how a confident citation of the wrong rule gets printed.
    pub fn at(&self, n: usize) -> Option<usize> {
        let ok = n.checked_sub(1).filter(|i| *i < self.items.len());
        if ok.is_none() {
            self.refused.set(self.refused.get() + 1);
            eprintln!(
                "warning: dropped a reading naming {} {n} — only 1..{} were offered",
                self.what,
                self.items.len()
            );
        }
        ok
    }

    /// How many answers named something that was not on offer.
    ///
    /// Counted rather than merely warned about: "the extractor cited past the
    /// end nine times" is a measurement about the reading pass, not noise to
    /// swallow.
    pub fn refused(&self) -> usize {
        self.refused.get()
    }
}

/// Text in, typed evidence out, never a verdict.
///
/// Implementors describe the reading; they do not perform the decision. The
/// contract has three parts and each one has been paid for:
///
/// 1. **The model answers in a schema**, so a malformed answer is a parse
///    error rather than a plausible sentence.
/// 2. **Answers are keyed by number**, so code can check that what came back
///    is something that was offered.
/// 3. **Anything unsaid keeps a refusing default** ([`Resolver::unread`]).
///    Absence is reported, never defaulted — a resolver that filled in a
///    permissive answer for what the model skipped would be inventing evidence
///    (§18.3).
pub trait Resolver {
    /// The typed evidence produced for one item.
    type Reading;

    /// The key the answer object is returned under.
    fn name(&self) -> &'static str;

    /// What the model is told it is doing. Succinct and non-contradictory:
    /// asking for two things at once is what small models handle worst, and
    /// it is the documented cause of the one prompt regression here.
    fn system(&self) -> &'static str;

    /// The shape the answer must take.
    fn schema(&self) -> Value;

    /// The reading for an item the model said nothing about.
    ///
    /// **Must refuse, never approve.** For `subject` that is "represents
    /// itself", which refuses the fold; for `quantify` it is "no quantities",
    /// which cannot claim a disagreement.
    fn unread(&self, index: usize) -> Self::Reading;

    /// One reading per item offered, starting from the refusing default.
    fn blank(&self, offered: &Offered) -> Vec<Self::Reading> {
        (0..offered.len()).map(|i| self.unread(i)).collect()
    }
}

/// Ask a resolver's question about a numbered list.
///
/// The one place a resolver's prompt is assembled, so the numbering the model
/// sees and the numbering [`Offered::at`] checks against cannot drift apart.
pub fn ask<R: Resolver, T: DeserializeOwned>(
    client: &Client,
    resolver: &R,
    offered: &Offered,
    heading: &str,
    question: &str,
) -> Result<T, ModelError> {
    let user = format!("{heading}\n{}\n{question}\n", offered.numbered());
    client.complete_json(
        resolver.system(),
        &user,
        resolver.name(),
        &resolver.schema(),
    )
}

#[cfg(test)]
mod tests;
