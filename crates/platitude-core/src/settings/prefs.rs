//! `settings.toml`: what a person decided.

use toml::{Table, Value};

use super::SCHEMA_VERSION;
use super::toml::{clamp_to_i64, minutes, repo_key, sub_table, timeout_secs};

/// The values that apply to a repository. One set is the application's
/// defaults; a repository may override any of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoSettings {
    pub auto_fetch_minutes: u32,
    pub network_timeout_secs: u64,
}

impl Default for RepoSettings {
    fn default() -> Self {
        Self {
            auto_fetch_minutes: crate::session::AUTO_FETCH_DEFAULT_MINUTES,
            network_timeout_secs: crate::remote::DEFAULT_NETWORK_TIMEOUT.as_secs(),
        }
    }
}

/// What one repository asked to differ on. Absent fields take the default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoOverride {
    /// The work tree path, as [`repo_key`] writes it.
    pub key: String,
    pub auto_fetch_minutes: Option<u32>,
    pub network_timeout_secs: Option<u64>,
}

impl RepoOverride {
    pub fn new(path: &str) -> Self {
        Self {
            key: repo_key(path),
            auto_fetch_minutes: None,
            network_timeout_secs: None,
        }
    }

    fn is_empty(&self) -> bool {
        self.auto_fetch_minutes.is_none() && self.network_timeout_secs.is_none()
    }
}

/// `settings.toml`.
///
/// Per-repository overrides are only for values the application owns. What
/// git already remembers per repository — the identity on a commit, the
/// merge tool — stays in that repository's own config, where git put it and
/// where every other tool can see it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Settings {
    pub defaults: RepoSettings,
    pub repos: Vec<RepoOverride>,
    /// Pictures put against authors. Kept here rather than in a third file
    /// because the split between the two is a write-rate one: an avatar is
    /// assigned about as often as an interval is changed, and neither
    /// happens while a pane is being dragged.
    pub avatars: crate::avatar::Avatars,
}

impl Settings {
    /// The values in force for one repository.
    pub fn for_repo(&self, path: &str) -> RepoSettings {
        let key = repo_key(path);
        let Some(over) = self.repos.iter().find(|r| r.key == key) else {
            return self.defaults.clone();
        };
        RepoSettings {
            auto_fetch_minutes: over
                .auto_fetch_minutes
                .unwrap_or(self.defaults.auto_fetch_minutes),
            network_timeout_secs: over
                .network_timeout_secs
                .unwrap_or(self.defaults.network_timeout_secs),
        }
    }

    pub(super) fn from_table(table: &Table) -> Self {
        let fallback = RepoSettings::default();
        let defaults = match sub_table(table, "defaults") {
            Some(t) => RepoSettings {
                auto_fetch_minutes: minutes(t, "auto_fetch_minutes")
                    .unwrap_or(fallback.auto_fetch_minutes),
                network_timeout_secs: timeout_secs(t, "network_timeout_secs")
                    .unwrap_or(fallback.network_timeout_secs),
            },
            None => fallback,
        };
        let mut repos = Vec::new();
        if let Some(by_repo) = sub_table(table, "repo") {
            for (key, value) in by_repo {
                let Some(entry) = value.as_table() else {
                    continue;
                };
                let over = RepoOverride {
                    key: repo_key(key),
                    auto_fetch_minutes: minutes(entry, "auto_fetch_minutes"),
                    network_timeout_secs: timeout_secs(entry, "network_timeout_secs"),
                };
                if !over.is_empty() {
                    repos.push(over);
                }
            }
        }
        let avatars = table
            .get("avatar")
            .and_then(Value::as_array)
            .map(|values| crate::avatar::Avatars::from_values(values))
            .unwrap_or_default();
        Self {
            defaults,
            repos,
            avatars,
        }
    }

    pub(super) fn to_table(&self) -> Table {
        let mut root = Table::new();
        root.insert("version".into(), Value::Integer(SCHEMA_VERSION));

        let mut defaults = Table::new();
        defaults.insert(
            "auto_fetch_minutes".into(),
            Value::Integer(self.defaults.auto_fetch_minutes.into()),
        );
        defaults.insert(
            "network_timeout_secs".into(),
            Value::Integer(clamp_to_i64(self.defaults.network_timeout_secs)),
        );
        root.insert("defaults".into(), Value::Table(defaults));

        let mut by_repo = Table::new();
        for over in &self.repos {
            if over.is_empty() || over.key.is_empty() {
                continue;
            }
            let mut entry = Table::new();
            if let Some(minutes) = over.auto_fetch_minutes {
                entry.insert("auto_fetch_minutes".into(), Value::Integer(minutes.into()));
            }
            if let Some(secs) = over.network_timeout_secs {
                entry.insert(
                    "network_timeout_secs".into(),
                    Value::Integer(clamp_to_i64(secs)),
                );
            }
            by_repo.insert(over.key.clone(), Value::Table(entry));
        }
        if !by_repo.is_empty() {
            root.insert("repo".into(), Value::Table(by_repo));
        }
        if !self.avatars.is_empty() {
            root.insert("avatar".into(), Value::Array(self.avatars.to_values()));
        }
        root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_repository_takes_its_own_values_and_the_defaults_for_the_rest() {
        let settings = Settings {
            defaults: RepoSettings {
                auto_fetch_minutes: 5,
                network_timeout_secs: 300,
            },
            avatars: crate::avatar::Avatars::default(),
            repos: vec![RepoOverride {
                key: "C:/big".into(),
                auto_fetch_minutes: Some(0),
                network_timeout_secs: None,
            }],
        };
        let big = settings.for_repo(r"C:\big");
        assert_eq!(big.auto_fetch_minutes, 0, "separators do not matter");
        assert_eq!(big.network_timeout_secs, 300, "the rest is the default");
        assert_eq!(settings.for_repo("C:/other"), settings.defaults);
    }
}
