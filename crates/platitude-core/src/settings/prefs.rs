//! `settings.toml`: what a person decided.

use toml::{Table, Value};

use super::SCHEMA_VERSION;
use super::toml::{clamp_to_i64, minutes, sub_table, timeout_secs};

/// The values that decide how the application talks to remotes, read from
/// and written back to the `[defaults]` table this is named after.
///
/// **One set, for every repository.** How often this computer reaches the
/// network, and how long it waits when it does, are answers about the
/// machine and the line it is on rather than about whichever repository is
/// in front. A person who wants to be left alone turns the interval off,
/// and that is the whole of the vocabulary — there is no way to say it
/// about one repository and not another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Defaults {
    pub auto_fetch_minutes: u32,
    pub network_timeout_secs: u64,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            auto_fetch_minutes: crate::session::AUTO_FETCH_DEFAULT_MINUTES,
            network_timeout_secs: crate::remote::DEFAULT_NETWORK_TIMEOUT.as_secs(),
        }
    }
}

/// `settings.toml`.
///
/// Nothing in here is kept per repository. What git already remembers per
/// repository — the identity on a commit, the merge tool — stays in that
/// repository's own config, where git put it and where every other tool
/// can see it; and what is left over is about this computer rather than
/// about one repository ([`Defaults`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Settings {
    pub defaults: Defaults,
    /// Pictures put against authors. Kept here rather than in a third file
    /// because the split between the two is a write-rate one: an avatar is
    /// assigned about as often as an interval is changed, and neither
    /// happens while a pane is being dragged.
    pub avatars: crate::avatar::Avatars,
}

impl Settings {
    pub(super) fn from_table(table: &Table) -> Self {
        let fallback = Defaults::default();
        let defaults = match sub_table(table, "defaults") {
            Some(t) => Defaults {
                auto_fetch_minutes: minutes(t, "auto_fetch_minutes")
                    .unwrap_or(fallback.auto_fetch_minutes),
                network_timeout_secs: timeout_secs(t, "network_timeout_secs")
                    .unwrap_or(fallback.network_timeout_secs),
            },
            None => fallback,
        };
        let avatars = table
            .get("avatar")
            .and_then(Value::as_array)
            .map(|values| crate::avatar::Avatars::from_values(values))
            .unwrap_or_default();
        Self { defaults, avatars }
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

        if !self.avatars.is_empty() {
            root.insert("avatar".into(), Value::Array(self.avatars.to_values()));
        }
        root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file written while repositories could differ still parses, and the
    /// table that carried the difference decides nothing. Pinned because the
    /// alternative is silent: a `[repo.…]` heading read again later would put
    /// one repository back on an interval of its own, and the only place that
    /// would show is a tooltip disagreeing with the timer.
    #[test]
    fn a_per_repository_table_left_in_a_file_decides_nothing() {
        let text = r#"
version = 1

[defaults]
auto_fetch_minutes = 5

[repo."C:/big"]
auto_fetch_minutes = 0
network_timeout_secs = 9
"#;
        let settings = Settings::from_table(&text.parse::<Table>().expect("parse"));
        assert_eq!(settings.defaults.auto_fetch_minutes, 5);
        assert_eq!(
            settings.defaults.network_timeout_secs,
            Defaults::default().network_timeout_secs,
        );
        let written = settings.to_table();
        assert!(
            !written.contains_key("repo"),
            "and it is not written back out:\n{written}"
        );
    }

    /// An interval longer than the ceiling means the ceiling, whichever door
    /// it arrived through. The settings screen cannot offer one, so a file
    /// written by hand is the only door there is.
    #[test]
    fn an_interval_past_the_ceiling_is_the_ceiling() {
        let text = format!(
            "[defaults]\nauto_fetch_minutes = {}\n",
            crate::session::AUTO_FETCH_MAX_MINUTES + 1
        );
        let settings = Settings::from_table(&text.parse::<Table>().expect("parse"));
        assert_eq!(
            settings.defaults.auto_fetch_minutes,
            crate::session::AUTO_FETCH_MAX_MINUTES
        );
    }

    /// A number that is not an interval at all still falls back on its own,
    /// so one mistyped key costs only itself (rules-refs/core.md §読みは
    /// `toml::Table` からキーごとに取る).
    #[test]
    fn a_negative_interval_falls_back_to_the_default() {
        let text = "[defaults]\nauto_fetch_minutes = -5\n";
        let settings = Settings::from_table(&text.parse::<Table>().expect("parse"));
        assert_eq!(
            settings.defaults.auto_fetch_minutes,
            Defaults::default().auto_fetch_minutes
        );
    }
}
