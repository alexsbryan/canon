// SPDX-License-Identifier: AGPL-3.0-or-later
//! Which vocabulary and renderer apply.
//!
//! A closed set, so an enum rather than a string matched at each use site
//! (ARCH_PRINCIPLES §2.1). The distinction is not cosmetic: `Personal`
//! deliberately cannot render a verdict, and that is easier to enforce
//! against a type than against a string read from a file.

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Profile {
    /// How a person wants to be treated and work. Reports stakes, never a
    /// ruling.
    #[default]
    Personal,
    /// A codebase's principles. Verdict-shaped, because CI reads exit codes.
    Code,
    /// A household's rules. Framed as which act a proposal needs.
    House,
}

impl Profile {
    pub const ALL: [Profile; 3] = [Profile::Personal, Profile::Code, Profile::House];

    pub fn as_str(self) -> &'static str {
        match self {
            Profile::Personal => "personal",
            Profile::Code => "code",
            Profile::House => "house",
        }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        match s.trim() {
            "personal" => Ok(Profile::Personal),
            "code" => Ok(Profile::Code),
            "house" => Ok(Profile::House),
            other => Err(format!(
                "unknown profile `{other}` — expected one of: {}",
                Self::ALL
                    .iter()
                    .map(|p| p.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        }
    }

    /// What this profile calls one of the things in the canon.
    ///
    /// **One noun in the format, three on screen.** The wire, the API and
    /// every doc say `commitment`; that is the concept and it does not fork
    /// (§10.6). What changes is the word a reader is shown, which is the same
    /// thing [`crate::check`]'s voice table already does when it renders one
    /// outcome as `CONFLICT` for a codebase and `THIS NEEDS AN AMENDMENT` for
    /// a house.
    ///
    /// It is not decoration. A housemate does not have a body of commitments,
    /// they have house rules, and asking them to learn a word before they can
    /// write one down is the tax that stops a tool spreading past the person
    /// who installed it.
    pub fn noun(self) -> &'static str {
        match self {
            Profile::Personal => "commitment",
            Profile::Code => "principle",
            Profile::House => "rule",
        }
    }

    pub fn nouns(self) -> &'static str {
        match self {
            Profile::Personal => "commitments",
            Profile::Code => "principles",
            Profile::House => "rules",
        }
    }

    /// `1 rule`, `3 rules`.
    pub fn count(self, n: usize) -> String {
        format!("{n} {}", if n == 1 { self.noun() } else { self.nouns() })
    }

    /// Read the profile a canon was initialised with.
    ///
    /// A missing file is the pre-profile default, not an error. An
    /// *unreadable* value is an error rather than a silent fallback: a
    /// canon whose profile was corrupted must not quietly start rendering
    /// verdicts at someone's personal commitments (§18.3).
    pub fn load(dir: &Path) -> Result<Self, String> {
        match std::fs::read_to_string(dir.join("profile")) {
            Ok(s) => Self::parse(&s),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Profile::default()),
            Err(e) => Err(format!("reading profile: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Profile;

    #[test]
    fn every_profile_round_trips_through_its_string() {
        for p in Profile::ALL {
            assert_eq!(Profile::parse(p.as_str()), Ok(p));
        }
    }

    #[test]
    fn an_unknown_profile_is_an_error_not_a_default() {
        assert!(Profile::parse("hosue").is_err());
        assert!(Profile::parse("").is_err());
    }

    #[test]
    fn a_canon_with_no_profile_file_reads_as_personal() {
        // The pre-profile default. A MISSING file is benign; an unreadable
        // one is an error, never a silent fallback (§18.3).
        let dir = std::env::temp_dir().join("canon-profile-default-test");
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::remove_file(dir.join("profile"));
        assert_eq!(Profile::load(&dir), Ok(Profile::Personal));
    }

    #[test]
    fn a_corrupt_profile_is_an_error_not_a_silent_default() {
        let dir = std::env::temp_dir().join("canon-profile-corrupt-test");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("profile"), "hosue\n").unwrap();
        assert!(Profile::load(&dir).is_err());
    }

    #[test]
    fn each_profile_names_the_thing_the_way_its_readers_do() {
        assert_eq!(Profile::House.count(1), "1 rule");
        assert_eq!(Profile::House.count(3), "3 rules");
        assert_eq!(Profile::Code.count(1), "1 principle");
        assert_eq!(Profile::Personal.count(2), "2 commitments");
        // Every profile has both forms, so no call site has to special-case
        // one of them and quietly print `1 rules`.
        for p in Profile::ALL {
            assert!(!p.noun().is_empty() && !p.nouns().is_empty());
            assert_ne!(p.noun(), p.nouns());
        }
    }
}
