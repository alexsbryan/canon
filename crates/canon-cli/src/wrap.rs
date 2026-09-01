// SPDX-License-Identifier: AGPL-3.0-or-later
//! Wrapping long lines at the terminal's width, with a hanging indent.
//!
//! A rule is a sentence and a sentence is often longer than a screen is wide.
//! Left to the terminal, it breaks mid-word at the right edge and the next
//! line starts in the id column, which is unreadable at a desk and hopeless
//! on a projector. Every renderer that prints a commitment's text or a reason
//! goes through [`hang`], so the continuation lines up under the text.
//!
//! The width comes from `CANON_WIDTH`, then `COLUMNS`, then a default. There
//! is no ioctl and no crate: shells do not export `COLUMNS` by default, so a
//! script that wants exact wrapping exports it (`demo.sh` does), and everyone
//! else gets a width that reads well in a typical window.

const DEFAULT: usize = 100;
const MIN: usize = 40;
const MAX: usize = 240;

pub fn width() -> usize {
    ["CANON_WIDTH", "COLUMNS"]
        .iter()
        .find_map(|k| std::env::var(k).ok()?.trim().parse::<usize>().ok())
        .map(|w| w.clamp(MIN, MAX))
        .unwrap_or(DEFAULT)
}

/// `prefix` is printed once, verbatim; `text` is wrapped so that no line
/// exceeds the width, and every continuation line is indented to where the
/// text began. Words longer than the available width are left whole.
pub fn hang(prefix: &str, text: &str) -> String {
    hang_at(prefix, text, width())
}

pub fn hang_at(prefix: &str, text: &str, width: usize) -> String {
    let indent = prefix.chars().count();
    let avail = width.saturating_sub(indent).max(16);
    let pad = " ".repeat(indent);
    let mut out = String::from(prefix);
    let mut col = 0usize;
    for word in text.split_whitespace() {
        let w = word.chars().count();
        if col > 0 && col + 1 + w > avail {
            out.push('\n');
            out.push_str(&pad);
            col = 0;
        } else if col > 0 {
            out.push(' ');
            col += 1;
        }
        out.push_str(word);
        col += w;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continuation_lines_sit_under_the_text() {
        let s = hang_at("  RULE      ", "one two three four five six", 26);
        assert_eq!(s, "  RULE      one two three\n            four five six");
    }

    #[test]
    fn a_short_line_is_untouched() {
        assert_eq!(hang_at("id  ", "short", 80), "id  short");
    }

    #[test]
    fn a_word_wider_than_the_screen_stays_whole() {
        let s = hang_at("", "supercalifragilistic tiny", 20);
        assert_eq!(s, "supercalifragilistic\ntiny");
    }
}
