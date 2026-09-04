// SPDX-License-Identifier: AGPL-3.0-or-later
//! Dates, in one place.
//!
//! Not a clock — nothing here reads the system time. This is the conversion
//! between the Unix seconds an act carries and the `YYYY-MM-DD` a person
//! writes, both directions, so a horizon typed on the command line and a
//! `revisit` date written a year ago compare against the same number.
//!
//! It lives in `canon-core` rather than in the CLI because it is the second
//! caller that makes it a decider: `Accept.revisit` is a date string INSIDE
//! the format, and the staleness query has to read it. Two implementations of
//! the civil calendar would be two answers to "is this overdue" (§10.6).

/// Howard Hinnant's civil-from-days.
pub fn ymd(ts: i64) -> String {
    let days = ts.div_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

/// `YYYY-MM-DD` to the Unix second that day begins, UTC.
///
/// **Returns `None` rather than repairing the input.** A `revisit` field
/// somebody typed as "spring" is not a date, and reading it as one — or as
/// epoch zero, which is worse — would make it permanently overdue and teach
/// everyone to ignore the query. Absence is reported (§18.3): the staleness
/// query says how many horizons it could not read.
pub fn parse_ymd(s: &str) -> Option<i64> {
    let s = s.trim();
    let mut parts = s.split('-');
    let y: i64 = parts.next()?.parse().ok()?;
    let m: i64 = parts.next()?.parse().ok()?;
    let d: i64 = parts.next()?.parse().ok()?;
    // **The year is bounded because the arithmetic below is not.** `YYYY` is
    // four digits by construction, and without the bound
    // `parse_ymd("999999999999-01-01")` panicked on `attempt to multiply with
    // overflow` in debug and wrapped to some other century in release — from
    // `--horizon`, and from an `Accept.revisit` string read straight out of
    // `acts.jsonl`, where a bad merge is enough to produce one.
    if parts.next().is_some()
        || !(1..=9999).contains(&y)
        || !(1..=12).contains(&m)
        || !(1..=31).contains(&d)
    {
        return None;
    }
    // days-from-civil, the inverse of the above.
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    // A round trip is the check: `2026-02-31` parses arithmetically and is
    // not a day, and only rendering it back catches that.
    let ts = days * 86_400;
    (ymd(ts) == format!("{:04}-{m:02}-{d:02}", if m <= 2 { y + 1 } else { y })).then_some(ts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_render_correctly() {
        assert_eq!(ymd(0), "1970-01-01");
        assert_eq!(ymd(1_771_027_200), "2026-02-14");
        assert_eq!(ymd(1_609_459_200), "2021-01-01");
    }

    #[test]
    fn a_date_round_trips_through_both_directions() {
        for ts in [0, 1_609_459_200, 1_771_027_200, 2_000_000_000] {
            assert_eq!(parse_ymd(&ymd(ts)), Some(ts - ts.rem_euclid(86_400)));
        }
        assert_eq!(parse_ymd("2026-02-14"), Some(1_771_027_200));
    }

    #[test]
    fn what_is_not_a_date_is_refused_and_never_repaired() {
        // The failure this prevents: a horizon nobody can read becoming
        // epoch zero, therefore permanently overdue, therefore ignored.
        for bad in [
            "",
            "spring",
            "2026",
            "2026-13-01",
            "2026-02-31",
            "2026-00-10",
            "2026-1-1-1",
            "next tuesday",
            // Arithmetic, not just grammar: these overflowed i64 on the way
            // to a day number — a panic in debug, a wrong century in release
            // — and they arrive from `--horizon` and from a `revisit` string
            // sitting in somebody's `acts.jsonl`.
            "999999999999-01-01",
            "9223372036854775807-01-01",
            "0000-01-01",
        ] {
            assert_eq!(parse_ymd(bad), None, "`{bad}` is not a date");
        }
        // And a legitimately short one still is.
        assert_eq!(parse_ymd(" 2026-01-01 "), Some(1_767_225_600));
    }
}
