// SPDX-License-Identifier: AGPL-3.0-or-later
//! Progress lines, and what they look like when nobody is watching.
//!
//! `draft` and `tensions` overwrite one progress line in place, which is the
//! right shape on a terminal and the wrong one everywhere else: redirected
//! to a file, a background run's log was one line of `\r\x1b[K` with the
//! last count at the end of it. The terminal check lives here and nowhere
//! else, so every progress line answers it the same way (§10.6).

use std::io::{IsTerminal as _, Write as _};

/// A line that overwrites itself on a terminal and stands on its own in a
/// log. Nothing is flushed for a log; a terminal is flushed so the line
/// shows before the work it announces.
pub fn progress(line: &str) {
    let tty = std::io::stderr().is_terminal();
    eprint!("{}", render(line, false, tty));
    if tty {
        let _ = std::io::stderr().flush();
    }
}

/// The line that ends a run of progress lines: replaces the last one on a
/// terminal, and is one more line in a log.
pub fn done(line: &str) {
    eprint!("{}", render(line, true, std::io::stderr().is_terminal()));
}

/// Whether someone is at the keyboard to answer. A pipe, CI or an agent on
/// the MCP surface is not, and a question nobody can answer is a hang.
pub fn interactive() -> bool {
    std::io::stdin().is_terminal()
}

/// Ask on stderr and read the answer; anything but y or yes is no.
pub fn yes(question: &str) -> bool {
    eprint!("{question} [y/N] ");
    let _ = std::io::stderr().flush();
    let mut answer = String::new();
    if std::io::stdin().read_line(&mut answer).is_err() {
        return false;
    }
    matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

/// The rule, apart from the streams, so it can be tested without a pty.
fn render(line: &str, done: bool, tty: bool) -> String {
    match (tty, done) {
        (true, false) => format!("\r\x1b[K{line}"),
        (true, true) => format!("\r\x1b[K{line}\n"),
        (false, _) => format!("{line}\n"),
    }
}

#[cfg(test)]
mod tests {
    use super::render;

    #[test]
    fn a_log_gets_plain_lines_and_a_terminal_gets_overwriting_ones() {
        assert_eq!(
            render("extracting 3/24…", false, false),
            "extracting 3/24…\n"
        );
        assert_eq!(render("24 candidate(s)", true, false), "24 candidate(s)\n");
        assert_eq!(
            render("extracting 3/24…", false, true),
            "\r\x1b[Kextracting 3/24…"
        );
        assert_eq!(
            render("24 candidate(s)", true, true),
            "\r\x1b[K24 candidate(s)\n"
        );
        // No control code ever reaches a log, whichever kind of line it is.
        for done in [false, true] {
            assert!(!render("x", done, false).contains('\x1b'));
            assert!(!render("x", done, false).contains('\r'));
        }
    }
}
