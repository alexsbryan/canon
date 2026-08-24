// SPDX-License-Identifier: AGPL-3.0-or-later
//! `.canon/config` — the one user-visible knob, and its file.
//!
//! Deliberately not a format: `key = value`, one per line, `#` comments.
//! The keys are a CLOSED set, so they are an enum rather than a string
//! matched at each use site (ARCH_PRINCIPLES §2.1) — `canon config set
//! endpiont ...` is an error naming the known keys, not a silently ignored
//! line that leaves the user wondering why nothing changed.

use std::path::Path;

/// Every configurable key. Adding one means adding it here and to `ALL`,
/// which is what keeps `--help`, the parser, and `config show` in step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// OpenAI-compatible base URL, e.g. `http://localhost:8080/v1`.
    Endpoint,
    /// Model name sent with each request. Most local servers ignore it.
    Model,
    /// Embedding model, used only to ORDER commitments before comparison so
    /// that near-twins share a block. Optional: unset means comparison runs
    /// in document order, which is what it always did.
    EmbedModel,
}

impl Key {
    pub const ALL: [Key; 3] = [Key::Endpoint, Key::Model, Key::EmbedModel];

    pub fn as_str(self) -> &'static str {
        match self {
            Key::Endpoint => "endpoint",
            Key::Model => "model",
            Key::EmbedModel => "embed_model",
        }
    }

    /// The environment variable that overrides this key for one invocation.
    pub fn env_var(self) -> &'static str {
        match self {
            Key::Endpoint => "CANON_ENDPOINT",
            Key::Model => "CANON_MODEL",
            Key::EmbedModel => "CANON_EMBED_MODEL",
        }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|k| k.as_str() == s.trim())
            .ok_or_else(|| {
                format!(
                    "unknown config key `{s}` — known keys: {}",
                    Self::ALL
                        .iter()
                        .map(|k| k.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
    }
}

pub const FILE: &str = "config";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Config {
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub embed_model: Option<String>,
}

impl Config {
    pub fn get(&self, key: Key) -> Option<&str> {
        match key {
            Key::Endpoint => self.endpoint.as_deref(),
            Key::Model => self.model.as_deref(),
            Key::EmbedModel => self.embed_model.as_deref(),
        }
    }

    fn set_in_place(&mut self, key: Key, value: String) {
        let slot = match key {
            Key::Endpoint => &mut self.endpoint,
            Key::Model => &mut self.model,
            Key::EmbedModel => &mut self.embed_model,
        };
        *slot = Some(value);
    }

    /// Parse the file's text. An unknown key is an ERROR, not a skipped line:
    /// a typo that silently does nothing is the failure this refuses (§18.3).
    pub fn parse(s: &str) -> Result<Self, String> {
        let mut cfg = Config::default();
        for (i, raw) in s.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                return Err(format!("line {}: expected `key = value`", i + 1));
            };
            let key = Key::parse(k).map_err(|e| format!("line {}: {e}", i + 1))?;
            cfg.set_in_place(key, v.trim().to_string());
        }
        Ok(cfg)
    }

    /// Read `<dir>/config`, then apply environment overrides.
    ///
    /// A MISSING file is the benign default; an unreadable or malformed one is
    /// an error rather than an empty config, for the same reason
    /// [`crate::profile::Profile::load`] refuses a corrupt profile.
    pub fn load(dir: &Path) -> Result<Self, String> {
        let mut cfg = match std::fs::read_to_string(dir.join(FILE)) {
            Ok(s) => Self::parse(&s).map_err(|e| format!("{}/{FILE}: {e}", dir.display()))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Config::default(),
            Err(e) => return Err(format!("reading {}/{FILE}: {e}", dir.display())),
        };
        for key in Key::ALL {
            if let Ok(v) = std::env::var(key.env_var()) {
                if !v.trim().is_empty() {
                    cfg.set_in_place(key, v.trim().to_string());
                }
            }
        }
        Ok(cfg)
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        for key in Key::ALL {
            if let Some(v) = self.get(key) {
                out.push_str(&format!("{} = {v}\n", key.as_str()));
            }
        }
        out
    }

    /// Rewrite the file with one key changed. Config is small and
    /// last-write-wins, so unlike the act log it is rewritten rather than
    /// appended — there is no history here worth keeping.
    pub fn write(dir: &Path, key: Key, value: &str) -> Result<(), String> {
        let path = dir.join(FILE);
        let mut cfg = match std::fs::read_to_string(&path) {
            Ok(s) => Self::parse(&s).map_err(|e| format!("{}: {e}", path.display()))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Config::default(),
            Err(e) => return Err(format!("reading {}: {e}", path.display())),
        };
        cfg.set_in_place(key, value.to_string());
        std::fs::write(&path, cfg.render()).map_err(|e| format!("writing {}: {e}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::{Config, Key};

    #[test]
    fn every_key_round_trips_through_its_string() {
        for k in Key::ALL {
            assert_eq!(Key::parse(k.as_str()), Ok(k));
        }
    }

    #[test]
    fn an_unknown_key_is_an_error_not_a_skipped_line() {
        // A typo that silently does nothing is worse than a refusal: the user
        // sets `endpiont` and spends the next ten minutes wondering why
        // `tensions` still says there is no endpoint.
        assert!(Key::parse("endpiont").is_err());
        assert!(Config::parse("endpiont = http://x/v1\n").is_err());
    }

    #[test]
    fn config_round_trips_through_its_rendering() {
        let src = "# a comment\nendpoint = http://localhost:8080/v1\nmodel = qwen\n";
        let cfg = Config::parse(src).unwrap();
        assert_eq!(cfg.endpoint.as_deref(), Some("http://localhost:8080/v1"));
        assert_eq!(cfg.model.as_deref(), Some("qwen"));
        assert_eq!(Config::parse(&cfg.render()), Ok(cfg));
    }

    #[test]
    fn a_line_without_an_equals_is_an_error() {
        assert!(Config::parse("endpoint http://x\n").is_err());
    }
}
