// SPDX-License-Identifier: AGPL-3.0-or-later
//! Where in a passage a rule is stated.
//!
//! **Ask the model for a position, never for a copy.** The map step used to
//! ask for the rule AND the words it came from, then check the words were
//! really there. Reproduction is a task a model can fail while reading
//! perfectly well: it reflows, tidies a hyphen, drops a subsection letter,
//! and a correct extraction is thrown away for a transcription slip. Measured
//! on a municipal noise code, 12.8% of candidates died that way — against
//! 1.6% on a hand-written house charter, which is the tell that the failure
//! tracks how awkward the source is to retype, not how hard it is to read.
//!
//! So the passage is cut into sentences by code, the model is shown them
//! numbered, and it answers with an INDEX. The citation is then a byte slice
//! of the source, and "the quote is not in the passage" stops being a thing
//! that can happen — not a rule the model is asked to keep (§7.6), and not a
//! check that can fire. What remains checkable is whether the index points at
//! sentences the passage actually has, which is code's to decide and is
//! decided here.
//!
//! **On the segmenter.** Cutting text into sentences is a heuristic, and this
//! one is deliberately small. It is not a decider: it fixes a coordinate
//! system, and every verdict downstream is taken over structure someone else
//! produced. Its worst failure is a citation wider or narrower than a reader
//! would have drawn — never one that is not in the source. Where it is unsure
//! it merges rather than splits, because a citation carrying a neighbouring
//! sentence still evidences the rule and a fragment does not.

use std::fmt;
use std::ops::Range;

/// A citation shorter than this cannot be evidence of anything.
pub const QUOTE_MIN: usize = 20;

/// The most sentences one rule may cite.
///
/// A rule and its qualifier can straddle a sentence break; a rule pointing at
/// half a page is pointing at the passage, which is what the chunk id already
/// records.
pub const SPAN_MAX: usize = 3;

/// A citation that does not point at sentences this passage has.
///
/// Every variant names what was asked for and what was on offer, because
/// these are counted: "the extractor cited past the end nine times" is a
/// measurement about the reading pass, not noise to swallow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Miscited {
    /// An index outside `1..=have`.
    OutOfRange {
        first: usize,
        last: usize,
        have: usize,
    },
    /// `last` before `first`.
    Backwards { first: usize, last: usize },
    /// More than [`SPAN_MAX`] sentences.
    TooWide { n: usize },
    /// A real span, too small to evidence anything.
    TooShort { chars: usize },
}

impl fmt::Display for Miscited {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfRange { first, last, have } if first == last => {
                write!(f, "cited sentence {first}, but the passage has {have}")
            }
            Self::OutOfRange { first, last, have } => {
                write!(
                    f,
                    "cited sentences {first}-{last}, but the passage has {have}"
                )
            }
            Self::Backwards { first, last } => {
                write!(f, "cited sentences {first} to {last}, which runs backwards")
            }
            Self::TooWide { n } => write!(
                f,
                "cited {n} sentences; a citation that wide evidences the passage, not the rule"
            ),
            Self::TooShort { chars } => {
                write!(
                    f,
                    "cited span is {chars} characters — too short to be evidence"
                )
            }
        }
    }
}

/// The passage's sentences, as byte ranges into it, in order.
///
/// Ranges are trimmed, non-overlapping and ascending, so `text[span]` is
/// always a slice of the original — that is what makes a citation built from
/// one verbatim by construction rather than by inspection.
///
/// A cut is made where the document itself marks a new unit — a list marker,
/// a table row, a blank line — and where prose ends a sentence. Prose ends
/// when `.`, `!` or `?` is followed by whitespace and then something that
/// starts a sentence. A period closing a one-letter token does not end
/// anything, which is what keeps `9:00 a.m.` and `I.C. ch. 321G` whole.
pub fn sentences(text: &str) -> Vec<Range<usize>> {
    let mut cuts: Vec<usize> = vec![0, text.len()];

    // Structure the document already marks.
    let mut at = 0usize;
    for line in text.split_inclusive('\n') {
        if at > 0 && starts_a_unit(line) {
            cuts.push(at);
        }
        at += line.len();
    }

    // Sentence ends inside a run of prose.
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    for i in 0..chars.len() {
        if !matches!(chars[i].1, '.' | '!' | '?') {
            continue;
        }
        if chars[i].1 == '.' && closes_a_label(&chars, i) {
            continue;
        }
        // Closing punctuation belongs to the sentence that just ended.
        let mut j = i + 1;
        while matches!(
            chars.get(j).map(|c| c.1),
            Some('"' | '\'' | ')' | ']' | '\u{201d}' | '\u{2019}')
        ) {
            j += 1;
        }
        // A dot inside a token — `42.5`, `sec.42` — ends nothing.
        if !chars.get(j).is_some_and(|c| c.1.is_whitespace()) {
            continue;
        }
        while chars.get(j).is_some_and(|c| c.1.is_whitespace()) {
            j += 1;
        }
        let Some(&(idx, next)) = chars.get(j) else {
            continue;
        };
        if next.is_uppercase() || matches!(next, '(' | '"' | '\u{a7}' | '\u{201c}') {
            cuts.push(idx);
        }
    }

    cuts.sort_unstable();
    cuts.dedup();
    cuts.windows(2)
        .filter_map(|w| trimmed(text, w[0]..w[1]))
        .collect()
}

/// The passage as the model sees it, one numbered sentence per line.
///
/// Positions are marked `[n]`, which municipal drafting does not use, so the
/// coordinate system cannot be confused with the document's own `(1)`, `a.`
/// and `1.` numbering. Each sentence is flattened onto one line for the
/// prompt; the copy still comes from the source, so a sentence that wrapped
/// in the document keeps its line breaks in the citation.
pub fn numbered(text: &str, spans: &[Range<usize>]) -> String {
    let mut out = String::new();
    for (i, s) in spans.iter().enumerate() {
        let flat = text[s.clone()]
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        out.push_str(&format!("[{}] {flat}\n", i + 1));
    }
    out
}

/// Copy the cited sentences out of the passage, `first` and `last` 1-based
/// and inclusive.
///
/// The span runs from the start of `first` to the end of `last`, so whatever
/// separated them in the document — a newline, a list marker, a table pipe —
/// is carried along. A citation is a contiguous slice of its passage or it is
/// an error; there is no third outcome and nothing is repaired quietly.
pub fn cite(
    text: &str,
    spans: &[Range<usize>],
    first: usize,
    last: usize,
) -> Result<String, Miscited> {
    let have = spans.len();
    if first < 1 || last < 1 || first > have || last > have {
        return Err(Miscited::OutOfRange { first, last, have });
    }
    if last < first {
        return Err(Miscited::Backwards { first, last });
    }
    let n = last - first + 1;
    if n > SPAN_MAX {
        return Err(Miscited::TooWide { n });
    }
    let quoted = text[spans[first - 1].start..spans[last - 1].end].trim();
    let chars = quoted.chars().count();
    if chars < QUOTE_MIN {
        return Err(Miscited::TooShort { chars });
    }
    Ok(quoted.to_string())
}

/// Does this line open a unit the document itself marked?
///
/// A blank line separates whatever is on either side of it. Otherwise it is a
/// table row, a block quote, a heading, a bullet, or a short alphanumeric
/// token closed by `)` or `. ` — `(1)`, `(a)`, `1.`, `iv.`, `Sec.`.
fn starts_a_unit(line: &str) -> bool {
    let t = line.trim_start();
    let Some(first) = t.chars().next() else {
        return true;
    };
    match first {
        '|' | '#' | '>' => true,
        '-' | '*' | '+' => t[first.len_utf8()..].starts_with(' '),
        '(' => marker(&t[1..], ")"),
        c if c.is_alphanumeric() => marker(t, ". "),
        _ => false,
    }
}

/// `1.`, `(a)`, `(iv)` — an enumerator, and not a short word that a wrapped
/// line happened to start with.
///
/// This took any short alphanumeric token until a house charter wrapped
/// "…moved to the owner's bedroom / door. Perishable food…" and `door.` read
/// as a marker, stranding the verb of one rule as a position of its own.
/// Enumerators are digits, one letter, or roman numerals; English words are
/// none of those.
fn marker(t: &str, close: &str) -> bool {
    let tok: String = t.chars().take_while(|c| c.is_alphanumeric()).collect();
    let n = tok.chars().count();
    if n == 0 || n > 4 || !t[tok.len()..].starts_with(close) {
        return false;
    }
    n == 1
        || tok.chars().all(|c| c.is_ascii_digit())
        || tok
            .chars()
            .all(|c| matches!(c, 'i' | 'v' | 'x' | 'l' | 'c' | 'd' | 'm'))
}

/// How far into its line a numeric token may sit and still read as a label
/// rather than as the end of a sentence.
///
/// `Table 1.` and `Sec. 42-1.` open their lines; `…shown in table 3.` does
/// not, and is a sentence that happens to end in a number. Nothing but
/// position separates the two, so position is what decides.
const LABEL_COL: usize = 12;

/// Is the `.` at `i` closing a label rather than a sentence?
///
/// Two shapes. A one-letter token is an initial or an abbreviation —
/// `I.C. ch. 321G`, `9:00 a.m.` A numeric token opening its line is an
/// enumerator — `1. Electrical power tools.`, `Table 1. Sound Levels`,
/// `Sec. 42-1. Noise.` — and cutting there strands the marker as a position
/// of its own, with the rule it labels at the next one.
///
/// Merging is the safe direction both times: two sentences cited as one still
/// evidence the rule, while `1.` cited alone evidences nothing.
fn closes_a_label(chars: &[(usize, char)], i: usize) -> bool {
    let tok = token_before(chars, i);
    if tok.chars().count() == 1 && tok.chars().all(char::is_alphabetic) {
        return true;
    }
    !tok.is_empty()
        && tok.chars().all(|c| c.is_ascii_digit() || c == '-')
        && column(chars, i) <= LABEL_COL
}

/// The word or number ending immediately before `i`.
fn token_before(chars: &[(usize, char)], i: usize) -> String {
    let mut j = i;
    while j > 0 && (chars[j - 1].1.is_alphanumeric() || chars[j - 1].1 == '-') {
        j -= 1;
    }
    chars[j..i].iter().map(|c| c.1).collect()
}

/// Characters between the start of `i`'s line and `i`.
fn column(chars: &[(usize, char)], i: usize) -> usize {
    let mut j = i;
    while j > 0 && chars[j - 1].1 != '\n' {
        j -= 1;
    }
    i - j
}

fn trimmed(text: &str, r: Range<usize>) -> Option<Range<usize>> {
    let slice = &text[r.clone()];
    let start = r.start + (slice.len() - slice.trim_start().len());
    let end = r.start + slice.trim_end().len();
    (start < end).then_some(start..end)
}

#[cfg(test)]
mod tests;
