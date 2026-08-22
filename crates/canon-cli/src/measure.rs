// SPDX-License-Identifier: AGPL-3.0-or-later
//! Reading measures out of a sentence — the numbers, units and clock times a
//! rule states.
//!
//! One implementation, two callers, on purpose (§10.6). It began inside
//! `draft`, where it does two jobs: it refuses a candidate stating a quantity
//! its passage never did, and it refuses to fold two rules that state
//! different quantities. `tensions` needs the same reading to notice that two
//! commitments disagree about a number, and a second implementation of "is
//! this the same measure" would be a second answer to that question.
//!
//! Every function here expects text already through [`normalize`].

/// Whitespace-insensitive containment. Models reflow quoted text across line
/// breaks; that is the same words, so it is the same citation.
pub(crate) fn normalize(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Units a rule measures things in. Deliberately short: these are the words
/// that change what a rule MEANS when they are wrong.
const UNITS: &[&str] = &[
    "minute", "hour", "day", "night", "week", "month", "year", "gallon", "dollar",
    // Loudness. A weighting letter is part of the unit, not decoration:
    // 85 dB(A) and 85 dB(C) permit different sound, so a rule restating one
    // as the other states a different limit while looking like a copy.
    "decibel", "db", "dba", "dbc",
];

/// Number words a rule might use where the passage used a digit, or the
/// other way round. `twice a month` and `two times per month` are the same
/// rule and must not be read as different ones.
fn as_number(word: &str) -> Option<u32> {
    const WORDS: &[(&str, u32)] = &[
        ("once", 1),
        ("one", 1),
        ("single", 1),
        ("twice", 2),
        ("two", 2),
        ("three", 3),
        ("four", 4),
        ("five", 5),
        ("six", 6),
        ("seven", 7),
        ("eight", 8),
        ("nine", 9),
        ("ten", 10),
        ("eleven", 11),
        ("twelve", 12),
        ("thirteen", 13),
        ("fourteen", 14),
        ("fifteen", 15),
        ("sixteen", 16),
        ("seventeen", 17),
        ("eighteen", 18),
        ("nineteen", 19),
        ("twenty", 20),
        ("thirty", 30),
        ("forty", 40),
        ("fifty", 50),
        ("sixty", 60),
        ("seventy", 70),
        ("eighty", 80),
        ("ninety", 90),
        ("hundred", 100),
    ];
    let w = word.trim_matches(|c: char| !c.is_ascii_alphanumeric());
    if let Ok(n) = w.parse::<u32>() {
        return Some(n);
    }
    WORDS.iter().find(|(s, _)| *s == w).map(|(_, n)| *n)
}

fn singular(word: &str) -> String {
    let w: String = word
        .trim_matches(|c: char| !c.is_ascii_alphanumeric())
        .to_string();
    w.strip_suffix('s').map(str::to_string).unwrap_or(w)
}

/// One number starting at `words[i]`, and how many words it spans.
///
/// `twenty-five` tokenises to two words and states one number. Read as two,
/// a rule naming a twenty-five dollar fee states neither the fee it names nor
/// the one its passage does, and the faithful rule is dropped.
fn number_at(words: &[String], i: usize) -> Option<(u32, usize)> {
    let n = as_number(&words[i])?;
    if (20..=90).contains(&n) && n % 10 == 0 {
        if let Some(u) = words.get(i + 1).and_then(|w| as_number(w)) {
            if (1..=9).contains(&u) {
                return Some((n + u, 2));
            }
        }
    }
    Some((n, 1))
}

/// The unit a symbol states on its own. `$50` names a currency with no
/// currency word in it, and a guard that reads no measure there folds a $50
/// rule into a $75 one.
const SYMBOL_UNITS: &[(char, &str)] = &[('$', "dollar"), ('%', "percent")];

/// A meridiem at `chars[j]`, punctuated or not: `pm`, `p.m.`, `p.m` all read
/// as `pm`. Returns the half of the day and the position after it.
///
/// The trailing boundary check is load-bearing: without it `at 7 among the
/// hedges` reads as 7am.
fn meridiem_at(b: &[char], j: usize) -> Option<(&'static str, usize)> {
    let half = match b.get(j)? {
        'a' => "am",
        'p' => "pm",
        _ => return None,
    };
    let mut k = j + 1;
    if b.get(k) == Some(&'.') {
        k += 1;
    }
    if b.get(k) != Some(&'m') {
        return None;
    }
    k += 1;
    if b.get(k) == Some(&'.') {
        k += 1;
    }
    match b.get(k) {
        Some(c) if c.is_ascii_alphanumeric() => None,
        _ => Some((half, k)),
    }
}

/// Every clock time in `s`, canonicalised: `11:00 PM`, `11 pm`, `11 p.m.` and
/// `23:00` all become `11pm`. Minutes appear only when they are not zero, so
/// `10 pm` and `10:00 PM` are one time and `10:30 PM` is a different one.
///
/// Expects text already through [`normalize`].
fn clock_times(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let b: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < b.len() {
        if !b[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        let hour: String = b[start..i].iter().collect();
        if hour.len() > 2 {
            continue;
        }
        let mut j = i;
        // Optional `:mm`. Minutes are kept, because 10:30 PM and 10:00 PM are
        // two different rules about the same subject.
        let mut minutes = String::new();
        if j < b.len() && b[j] == ':' {
            j += 1;
            let m = j;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            minutes = b[m..j].iter().collect();
        }
        let after_digits = j;
        while j < b.len() && b[j] == ' ' {
            j += 1;
        }
        let mm = if minutes.trim_start_matches('0').is_empty() {
            String::new()
        } else {
            format!(":{minutes}")
        };
        if let Some((half, end)) = meridiem_at(&b, j) {
            out.push(format!("{hour}{mm}{half}"));
            i = end;
            continue;
        }
        // No meridiem. A 24-hour clock is still a time, but only where it
        // cannot be read as either half of the day: a bare 7:00 is ambiguous
        // and guessing at it would invent a measure the document never
        // stated.
        if minutes.is_empty() {
            continue;
        }
        let Ok(h) = hour.parse::<u32>() else {
            continue;
        };
        let named = match h {
            0 => Some(("12", "am")),
            13..=23 => None,
            _ => continue,
        };
        match named {
            Some((h12, half)) => out.push(format!("{h12}{mm}{half}")),
            None => out.push(format!("{}{mm}pm", h - 12)),
        }
        i = after_digits;
    }
    out
}

/// How far apart a number and its unit may sit and still be one measure.
/// "two consecutive nights" is a measure; a number three sentences from a
/// unit is not.
const MEASURE_GAP: usize = 2;

/// Words that turn a number into a time of day rather than a count.
/// "eleven at night" and "11 PM" are the same instant, and reading the first
/// as a count of nights makes two identical rules look like a contradiction.
const TIME_OF_DAY: &[(&str, &str)] = &[("night", "pm"), ("evening", "pm"), ("morning", "am")];

/// Words, split on whitespace AND hyphens.
///
/// Hyphens matter: "within any seven-day period" states the same measure as
/// "per 7 days", and a tokeniser that only splits on spaces sees no number at
/// all in the first.
fn words_of(s: &str) -> Vec<String> {
    s.split(|c: char| c.is_whitespace() || c == '-')
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

/// Is this unit being used as a time of day — "at night", "in the morning"?
fn is_time_of_day(words: &[String], unit_at: usize) -> bool {
    let unit = singular(&words[unit_at]);
    if !TIME_OF_DAY.iter().any(|(u, _)| *u == unit) {
        return false;
    }
    words[..unit_at]
        .iter()
        .rev()
        .take(2)
        .any(|w| matches!(singular(w).as_str(), "at" | "in" | "the"))
}

/// Number-and-unit pairs in `s`, as `(value, unit)`. Times of day are not
/// counts and are excluded — [`clock_times`] picks those up instead.
fn measures(s: &str) -> Vec<(u32, String)> {
    let words = words_of(s);
    let mut out = Vec::new();
    let mut i = 0;
    while i < words.len() {
        let Some((n, used)) = number_at(&words, i) else {
            i += 1;
            continue;
        };
        // A symbol states its unit inside the same token, so there is no gap
        // to scan.
        if let Some((_, unit)) = SYMBOL_UNITS
            .iter()
            .find(|(c, _)| words[i].contains(*c) || words[i + used - 1].contains(*c))
        {
            out.push((n, (*unit).to_string()));
            i += used;
            continue;
        }
        for j in (i + used)..(i + used + 1 + MEASURE_GAP).min(words.len()) {
            let u = singular(&words[j]);
            if UNITS.contains(&u.as_str()) {
                if !is_time_of_day(&words, j) {
                    out.push((n, u));
                }
                break;
            }
        }
        i += used;
    }
    out
}

/// Times written in words: "eleven at night" -> `11pm`.
fn spelled_times(s: &str) -> Vec<String> {
    let words = words_of(s);
    let mut out = Vec::new();
    // `midnight` and `noon` name an instant with no digit in it, so a rule
    // beginning quiet hours at midnight and one beginning them at 11:00 PM
    // would otherwise not disagree about anything.
    for w in &words {
        match singular(w).as_str() {
            "midnight" => out.push("12am".to_string()),
            "noon" => out.push("12pm".to_string()),
            _ => {}
        }
    }
    let mut i = 0;
    while i < words.len() {
        let Some((n, used)) = number_at(&words, i) else {
            i += 1;
            continue;
        };
        for j in (i + used)..(i + used + 1 + MEASURE_GAP).min(words.len()) {
            let u = singular(&words[j]);
            if let Some((_, half)) = TIME_OF_DAY.iter().find(|(t, _)| *t == u) {
                if is_time_of_day(&words, j) {
                    out.push(format!("{n}{half}"));
                }
                break;
            }
        }
        i += used;
    }
    out
}

/// Does this rule state a measure its passage does not?
///
/// **The citation check proves the QUOTE is real. It does not prove the RULE
/// matches it.** Observed against a live endpoint: a candidate read "at least
/// three hours in advance" while its own verbatim quote said "three days
/// ahead". A rule that misstates a time or a count is worse than a missing
/// rule — it is a house rule contradicting the sentence printed beneath it,
/// and the citation makes it look checked.
///
/// Narrow on purpose. Only NUMBER-AND-UNIT pairs and clock times are
/// compared, so a rule that rewords "within any seven-day period" as "per
/// week" survives — that is a paraphrase, not a different rule. What does not
/// survive is a quantity the passage never states.
pub(crate) fn unstated_measure(text: &str, chunk: &str) -> Option<String> {
    let (t, c) = (normalize(text), normalize(chunk));
    let source: Vec<(u32, String)> = measures(&c);
    for (n, unit) in measures(&t) {
        if !source.iter().any(|(m, u)| *m == n && *u == unit) {
            return Some(format!("{n} {unit}(s)"));
        }
    }
    let mut source_times = clock_times(&c);
    source_times.extend(spelled_times(&c));
    let mut stated = clock_times(&t);
    stated.extend(spelled_times(&t));
    stated.into_iter().find(|time| !source_times.contains(time))
}

/// Every measure a rule states, as comparable strings.
fn measure_set(s: &str) -> std::collections::BTreeSet<String> {
    let n = normalize(s);
    measures(&n)
        .into_iter()
        .map(|(v, u)| format!("{v} {u}"))
        .chain(clock_times(&n))
        .chain(spelled_times(&n))
        .collect()
}

/// Do these two rules state different measures?
///
/// **This is the guard that keeps `draft` able to find an unmarked
/// supersession at all.** Two rules that contradict each other look exactly
/// like near-duplicates to a reduce step — same subject, different content —
/// and folding them deletes the disagreement before anything can notice it.
/// Observed: "Quiet hours run from 11:00 PM to 7:00 AM every night" and
/// "Quiet hours begin at 10:00 PM from Sunday through Thursday" were grouped
/// as duplicates, which on its own destroyed two of the eleven planted
/// tensions in the bench fixture.
///
/// Only fires when BOTH rules state measures. A rule with no numbers in it
/// may still be a duplicate of one that has them.
pub(crate) fn differs_by_measure(a: &str, b: &str) -> bool {
    let (ma, mb) = (measure_set(a), measure_set(b));
    !ma.is_empty() && !mb.is_empty() && ma != mb
}

// ── pairs a reader would notice and a long list hides ───────

/// Words too common to say two rules are about the same thing.
const COMMON: &[&str] = &[
    "the", "and", "for", "with", "that", "this", "must", "may", "not", "are", "any", "all", "from",
    "have", "has", "will", "member", "members", "house", "each", "every", "their", "them", "when",
    "who", "whoever", "than", "more", "less", "least", "most", "into", "out", "own", "one", "per",
    "before", "after", "during", "until", "unless", "any", "other", "someone", "anyone",
];

/// What two rules are visibly about, as words worth matching on.
fn subject_words(s: &str) -> std::collections::BTreeSet<String> {
    words_of(&normalize(s))
        .into_iter()
        .map(|w| singular(&w))
        // A number is the MEASURE, not the subject. Left in, "two consecutive
        // nights" and "two days before the meeting" share a subject word and
        // qualify as a disagreement about guests and agendas at once.
        .filter(|w| as_number(w).is_none() && !w.chars().any(|c| c.is_ascii_digit()))
        .filter(|w| w.chars().count() >= 3 && !COMMON.contains(&w.as_str()))
        .collect()
}

/// Pairs of rules that state different measures about a shared subject,
/// scored by how much subject they share.
///
/// **This is the half of conflict-finding that needs no model at all.** A
/// reader spots "quiet hours start at 11 PM" against "quiet hours begin at 10
/// PM" instantly; a model asked to weigh every pair in a list of sixty finds
/// one in eleven, and blocks of twelve find five (`tensions::BATCH`). The
/// casualty is attention, not judgement — so the pairs this finds are ones
/// worth putting in front of the model TOGETHER, not verdicts to publish.
/// Nothing here decides that a pair is a conflict: two rules can name
/// different numbers and be perfectly compatible, which is exactly what the
/// bench's labelled decoys are.
///
/// Returns positions into `texts`, most subject-overlap first.
pub(crate) fn conflicting_pairs(texts: &[&str]) -> Vec<(usize, usize)> {
    let subjects: Vec<_> = texts.iter().map(|t| subject_words(t)).collect();
    let mut scored: Vec<(usize, usize, usize)> = Vec::new();
    for a in 0..texts.len() {
        for b in (a + 1)..texts.len() {
            let shared = subjects[a].intersection(&subjects[b]).count();
            if shared >= SUBJECT_MIN && differs_by_measure(texts[a], texts[b]) {
                scored.push((a, b, shared));
            }
        }
    }
    // Most-shared first, then by position so the order is the same every run.
    scored.sort_by(|x, y| y.2.cmp(&x.2).then(x.0.cmp(&y.0)).then(x.1.cmp(&y.1)));
    scored.into_iter().map(|(a, b, _)| (a, b)).collect()
}

/// How much subject two rules must share before their differing numbers mean
/// anything. Below this, "quiet hours end at 7 AM" and "rent is due on the
/// 1st" qualify as a disagreement about a number.
const SUBJECT_MIN: usize = 2;

#[cfg(test)]
mod tests {
    use super::*;

    const RESERVE: &str = "To make room for the occasional birthday or study group, the house \
resolved that a member may reserve the dining room for their own private event up to twice a \
month by signing the shared calendar at least three days ahead.";

    #[test]
    fn a_rule_that_changes_a_unit_is_refused_even_with_a_perfect_quote() {
        // Observed against a live endpoint. The quote was word-for-word — "by
        // signing the shared calendar at least three days ahead" — and the rule
        // said "three hours". The citation check passed it, because it proves the
        // quote is real and not that the rule matches it.
        assert_eq!(
            unstated_measure(
                "Reservations must be made by signing the shared calendar at least three hours in advance.",
                RESERVE
            )
            .as_deref(),
            Some("3 hour(s)")
        );
    }

    #[test]
    fn a_faithful_rule_survives_the_measure_check() {
        assert_eq!(
            unstated_measure(
                "Reservations are made by signing the shared calendar at least three days ahead.",
                RESERVE
            ),
            None
        );
    }

    #[test]
    fn a_number_word_and_its_digit_are_the_same_measure() {
        // "up to twice a month" and "no more than 2 times per month" are one
        // rule. Reading them as different ones would drop a good candidate, and
        // a false drop costs exactly the recall this guard is meant to protect.
        assert_eq!(
            unstated_measure(
                "A member may reserve the dining room 2 times per month.",
                RESERVE
            ),
            None
        );
        assert_eq!(
            unstated_measure(
                "A member may reserve the dining room two times a month.",
                RESERVE
            ),
            None
        );
        // But a different count is a different rule.
        assert_eq!(
            unstated_measure(
                "A member may reserve the dining room four times a month.",
                RESERVE
            )
            .as_deref(),
            Some("4 month(s)")
        );
    }

    #[test]
    fn a_reworded_unit_with_no_number_attached_is_not_a_measure() {
        // "within any seven-day period" reworded as "per week" is a paraphrase,
        // not a different rule. The guard compares number-and-unit PAIRS for
        // exactly this reason.
        let src = "Any single guest may stay no more than two consecutive nights within any \
                   seven-day period.";
        assert_eq!(
            unstated_measure("A guest may stay two nights per week.", src),
            None
        );
    }

    #[test]
    fn a_clock_time_the_passage_does_not_state_is_refused() {
        let src = "Quiet hours are observed every night, running from 11:00 PM until 7:00 AM.";
        assert_eq!(unstated_measure("Quiet hours run 11pm to 7am.", src), None);
        assert_eq!(
            unstated_measure("Quiet hours run 10pm to 7am.", src).as_deref(),
            Some("10pm")
        );
    }

    #[test]
    fn a_rule_with_no_measure_can_still_be_folded_into_one_that_has_them() {
        // The guard fires only when BOTH state measures; otherwise it would keep
        // every vague restatement of a timed rule as a separate commitment.
        assert!(!differs_by_measure(
            "Quiet hours run from 11:00 PM to 7:00 AM.",
            "Quiet hours are observed overnight."
        ));
        assert!(differs_by_measure(
            "Quiet hours run from 11:00 PM to 7:00 AM.",
            "Quiet hours begin at 10:00 PM."
        ));
    }

    #[test]
    fn a_hyphenated_measure_is_read_as_a_measure() {
        // "within any seven-day period" and "per 7 days" state the same limit. A
        // tokeniser that splits only on spaces sees no number in the first, and
        // the guard then reads two identical rules as a contradiction.
        assert!(measures("within any seven-day period").contains(&(7, "day".into())));
        assert!(measures("a twenty-gallon tank").contains(&(20, "gallon".into())));
    }

    #[test]
    fn a_time_of_day_in_words_is_a_time_not_a_count() {
        // "eleven at night" is 11pm, not eleven nights.
        assert!(measures("quiet time begins at eleven at night").is_empty());
        assert_eq!(
            spelled_times("quiet time begins at eleven at night"),
            vec!["11pm"]
        );
        // But a real count of nights survives.
        assert!(measures("no more than two consecutive nights").contains(&(2, "night".into())));
        assert!(spelled_times("no more than two consecutive nights").is_empty());
    }

    #[test]
    fn a_punctuated_meridiem_is_the_same_clock_time() {
        // Charters and bylaws write "10 p.m." at least as often as "10 PM", and a
        // parser that reads only the unpunctuated form sees NO time in the
        // passage. Both halves of the guard then fail in the same document: a
        // faithful rule is dropped for stating a measure its passage supposedly
        // does not, and two rules that disagree about the hour fold into one.
        assert_eq!(clock_times("quiet hours begin at 10:00 p.m."), vec!["10pm"]);
        assert_eq!(
            clock_times("the kitchen closes at 9 p.m. sharp"),
            vec!["9pm"]
        );
        assert_eq!(
            unstated_measure(
                "Quiet hours begin at 10pm.",
                "Quiet hours begin at 10:00 p.m. on weeknights."
            ),
            None
        );
        assert!(differs_by_measure(
            "Quiet hours begin at 10 p.m.",
            "Quiet hours begin at 11:00 PM."
        ));
    }

    #[test]
    fn noon_and_midnight_are_clock_times() {
        // "Quiet hours begin at midnight" against "Quiet hours begin at 11:00 PM"
        // is a contradiction with no digit in one of its halves.
        assert_eq!(spelled_times("quiet hours begin at midnight"), vec!["12am"]);
        assert_eq!(spelled_times("the pool closes at noon"), vec!["12pm"]);
        assert!(differs_by_measure(
            "Quiet hours begin at midnight.",
            "Quiet hours begin at 11:00 PM."
        ));
        assert_eq!(
            unstated_measure(
                "Quiet hours begin at 12am.",
                "Quiet hours begin at midnight."
            ),
            None
        );
    }

    #[test]
    fn a_twenty_four_hour_clock_is_the_same_instant() {
        // A house that writes 22:00 states the same rule as one that writes
        // 10:00 PM, and only the unambiguous hours are read: a bare 7:00 could be
        // either half of the day, and guessing would invent a measure.
        assert_eq!(clock_times("quiet hours begin at 22:00"), vec!["10pm"]);
        assert_eq!(clock_times("the gate locks at 00:30"), vec!["12:30am"]);
        assert_eq!(clock_times("the gate locks at 00:00"), vec!["12am"]);
        assert_eq!(clock_times("the gate locks at 22:30"), vec!["10:30pm"]);
        // Half an hour is a different rule, not the same one rounded.
        assert!(differs_by_measure(
            "Quiet hours begin at 10:30 PM.",
            "Quiet hours begin at 10:00 PM."
        ));
        assert!(clock_times("the meeting starts at 7:00").is_empty());
        assert_eq!(
            unstated_measure(
                "Quiet hours begin at 10:00 PM.",
                "quiet hours begin at 22:00"
            ),
            None
        );
    }

    #[test]
    fn a_fee_written_with_a_symbol_is_a_measure() {
        // Without this the two fees never disagree: `$50` carries no unit word,
        // so the guard reads no measure at all and folds a $50 rule into a $75
        // one. The symbol IS the unit.
        assert!(measures("a $50 late fee applies").contains(&(50, "dollar".into())));
        assert!(measures("a 10% surcharge applies").contains(&(10, "percent".into())));
        assert!(differs_by_measure(
            "A late payment carries a $50 fee.",
            "A late payment carries a $75 fee."
        ));
        assert_eq!(
            unstated_measure(
                "A late payment carries a $50 fee.",
                "the late fee is fifty dollars"
            ),
            None
        );
    }

    #[test]
    fn a_compound_number_word_is_one_number() {
        // "a twenty-five dollar fee" tokenises to `twenty` and `five`, and read
        // as two numbers it states neither the fee it names nor the one the
        // passage does — so the faithful rule is dropped.
        assert!(measures("a twenty-five dollar fee").contains(&(25, "dollar".into())));
        assert!(measures("forty five minutes of quiet").contains(&(45, "minute".into())));
        assert_eq!(
            unstated_measure(
                "A late payment carries a twenty-five dollar fee.",
                "the fee is $25"
            ),
            None
        );
    }

    #[test]
    fn two_rules_disagreeing_about_the_same_subject_are_paired() {
        let texts = vec![
            "Quiet hours run from 11:00 PM to 7:00 AM every night.",
            "Rent is due on the first day of the month.",
            "Quiet hours begin at 10:00 PM from Sunday through Thursday.",
        ];
        assert_eq!(conflicting_pairs(&texts), vec![(0, 2)]);
    }

    #[test]
    fn two_numbers_about_unrelated_things_are_not_a_pair() {
        // The whole risk of a mechanical pass is that it floods: every rule
        // in a house canon states some number, and "different number" alone
        // is not the beginning of a disagreement.
        let texts = vec![
            "Quiet hours begin at 10:00 PM.",
            "Rent is due on the fifth day of the month.",
        ];
        assert!(conflicting_pairs(&texts).is_empty());
    }

    #[test]
    fn rules_that_agree_are_not_paired() {
        let texts = vec![
            "Quiet hours begin at 10:00 PM.",
            "Quiet hours begin at 10 p.m. on weeknights.",
        ];
        assert!(conflicting_pairs(&texts).is_empty());
    }

    #[test]
    fn the_most_related_pair_comes_first() {
        let texts = vec![
            "Overnight guests may stay two consecutive nights.",
            "Quiet hours for overnight guests begin at 10:00 PM.",
            "Overnight guests may stay four consecutive nights in the house.",
        ];
        let pairs = conflicting_pairs(&texts);
        assert_eq!(pairs.first(), Some(&(0, 2)), "{pairs:?}");
    }

    #[test]
    fn a_number_is_not_a_subject() {
        // Both rules say "two", and they are about guests and about meeting
        // agendas. Pairing them wastes the one focused comparison the
        // mechanical pass buys.
        let texts = vec![
            "A single guest must not stay more than two consecutive nights.",
            "Members add agenda items no less than two days before the meeting.",
        ];
        assert!(conflicting_pairs(&texts).is_empty());
    }

    #[test]
    fn a_weighting_letter_is_part_of_the_unit() {
        // Measured on real municipal text: an ordinance restated six permit
        // types changing only dB(A) to dB(C), and every one was folded into
        // its predecessor as a duplicate because neither reading stated a
        // measure this module could see. Five planted supersessions were
        // destroyed at the reduce step before comparison ever ran.
        assert!(measures("not more than 85 dbas").contains(&(85, "dba".into())));
        assert!(measures("not more than 85 dbcs").contains(&(85, "dbc".into())));
        assert!(measures("not greater than 85 decibels").contains(&(85, "decibel".into())));
        assert!(measures("140 db").contains(&(140, "db".into())));
        assert!(differs_by_measure(
            "Sound equipment may register not more than 85 dBAs.",
            "Sound equipment may register not more than 85 dBCs."
        ));
        // The same limit is still the same limit.
        assert!(!differs_by_measure(
            "Sound equipment may register not more than 85 dBCs.",
            "No more than 85 dBC when measured at the property boundary."
        ));
    }

    #[test]
    fn a_rule_that_swaps_the_weighting_its_passage_states_is_refused() {
        let src = "A type \"A\" permit may be issued for sound equipment registering not more \
                   than 85 dBAs when measured at the real property boundary.";
        assert_eq!(
            unstated_measure("Sound equipment may register up to 85 dBCs.", src).as_deref(),
            Some("85 dbc(s)")
        );
        assert_eq!(
            unstated_measure("Sound equipment may register up to 85 dBAs.", src),
            None
        );
    }

    /// Would the fold guard refuse the folds this artifact actually made?
    ///
    /// Replays `differs_by_measure` over the duplicate groups a real run
    /// persisted, so a change to [`UNITS`] can be priced before an hour of
    /// endpoint time is spent re-running the sweep.
    ///
    /// ```sh
    /// CANON_RUN=<run.json> cargo test --bin canon -- --ignored --nocapture would_refuse
    /// ```
    #[test]
    #[ignore = "needs a draft run artifact: set CANON_RUN"]
    fn what_the_fold_guard_would_refuse_in_a_real_run() {
        let path = std::env::var("CANON_RUN").expect("set CANON_RUN");
        let run: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let cands = run["candidates"].as_array().unwrap();
        let text = |i: usize| cands[i]["text"].as_str().unwrap_or("").to_string();
        let (mut refused, mut allowed) = (0, 0);
        for g in run["duplicates"].as_array().unwrap() {
            let m: Vec<usize> = g
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as usize)
                .collect();
            let head = m[0];
            for &other in &m[1..] {
                if differs_by_measure(&text(head), &text(other)) {
                    refused += 1;
                    println!(
                        "  WOULD KEEP APART:\n    {}\n    {}\n",
                        text(head),
                        text(other)
                    );
                } else {
                    allowed += 1;
                }
            }
        }
        println!("{refused} fold(s) would now be refused, {allowed} still allowed");
    }

    /// What the mechanical pass proposes on a real canon, and what it costs.
    ///
    /// Reads a persisted `draft --dry-run` artifact rather than calling a
    /// model, so the answer is about THIS code and nothing else (§18.4).
    ///
    /// ```sh
    /// CANON_RUN=fixtures/maple-house/runs/qwen-27b/run-….json \
    ///   cargo test --bin canon -- --ignored --nocapture proposes
    /// ```
    #[test]
    #[ignore = "needs a draft run artifact: set CANON_RUN"]
    fn what_the_mechanical_pass_proposes_on_a_real_canon() {
        let path = std::env::var("CANON_RUN").expect("set CANON_RUN to a draft-run artifact");
        let run: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        // `kept` is positions into `candidates`, which is the same shape the
        // tool's own tensions step consumes.
        let cands = run["candidates"].as_array().unwrap();
        let kept: Vec<String> = run["kept"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| {
                cands[i.as_u64().unwrap() as usize]["text"]
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect();
        let refs: Vec<&str> = kept.iter().map(String::as_str).collect();
        let pairs = conflicting_pairs(&refs);
        println!(
            "\n{} rule(s) -> {} pair(s) proposed\n",
            refs.len(),
            pairs.len()
        );
        for (a, b) in &pairs {
            println!("  {}\n  {}\n", refs[*a], refs[*b]);
        }
    }
}
