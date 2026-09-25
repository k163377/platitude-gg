//! `settings.toml`: what a person decided.

use toml::{Table, Value};

use super::SCHEMA_VERSION;
use super::toml::{
    clamp_to_i64, concurrency, copies_interval, initial_commits, minutes, sub_table, text,
    timeout_secs,
};

/// The values a person decided once and every repository is opened with
/// (the `[defaults]` table). One set for every repository: they are
/// answers about the machine and the person at it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Defaults {
    pub auto_fetch_minutes: u32,
    pub network_timeout_secs: u64,
    /// Commits the graph opens with, `None` for the whole history — what
    /// is written into [`crate::session::LogOptions::limit`] on open
    /// (`RepoSession::set_log_limit`).
    pub initial_commits: Option<u32>,
    /// The git this computer runs, or empty for whichever one `PATH`
    /// resolves. A path: it is handed to the process spawner as the
    /// program, so a bare word says nothing the empty string does not.
    /// Not checked on the way in — [`crate::version::probe`] asks the
    /// binary itself.
    pub git_path: String,
    /// How many git processes the application runs at once, across every
    /// repository (`process::Limits::of`).
    pub git_concurrency: u32,
    /// Seconds between passes over the other working copies of a
    /// repository for uncommitted work — one `git status` per copy per
    /// pass (`session::carried`). Zero is off.
    pub copies_interval_secs: u32,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            auto_fetch_minutes: crate::session::AUTO_FETCH_DEFAULT_MINUTES,
            network_timeout_secs: crate::remote::DEFAULT_NETWORK_TIMEOUT.as_secs(),
            initial_commits: Some(crate::session::DEFAULT_LOG_LIMIT),
            git_path: String::new(),
            git_concurrency: crate::process::default_concurrency(),
            copies_interval_secs: crate::session::COPIES_INTERVAL_DEFAULT_SECS,
        }
    }
}

/// `settings.toml`. Nothing in here is kept per repository: what git
/// remembers per repository (the identity on a commit, the merge tool)
/// stays in that repository's own config, where every other tool can see
/// it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Settings {
    pub defaults: Defaults,
    /// Pictures put against authors. In this file because the split is by
    /// write rate: an avatar is assigned about as often as an interval is
    /// changed.
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
                initial_commits: initial_commits(t, "initial_commits")
                    .unwrap_or(fallback.initial_commits),
                git_path: text(t, "git_path").unwrap_or(fallback.git_path),
                git_concurrency: concurrency(t, "git_concurrency")
                    .unwrap_or(fallback.git_concurrency),
                copies_interval_secs: copies_interval(t, "copies_interval_secs")
                    .unwrap_or(fallback.copies_interval_secs),
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
        // The whole history is written as `0`: a key left out would read
        // as the default, the opposite answer.
        defaults.insert(
            "initial_commits".into(),
            Value::Integer(self.defaults.initial_commits.map_or(0, i64::from)),
        );
        // Written even when empty: empty is an answer ("whichever git
        // `PATH` resolves"), not a missing key.
        defaults.insert(
            "git_path".into(),
            Value::String(self.defaults.git_path.clone()),
        );
        defaults.insert(
            "git_concurrency".into(),
            Value::Integer(self.defaults.git_concurrency.into()),
        );
        // Off is written as `0`: a key left out would read as the
        // default, which is on.
        defaults.insert(
            "copies_interval_secs".into(),
            Value::Integer(self.defaults.copies_interval_secs.into()),
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

    /// A leftover `[repo.…]` table parses and decides nothing. Read again, it
    /// would silently put one repository back on an interval of its own.
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

    /// Only a file written by hand can ask for one.
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

    /// Falling back to the default here would answer the most aggressive
    /// interval to the value that asked for the least.
    #[test]
    fn an_interval_past_u32_is_still_the_ceiling() {
        let text = format!("[defaults]\nauto_fetch_minutes = {}\n", i64::MAX);
        let settings = Settings::from_table(&text.parse::<Table>().expect("parse"));
        assert_eq!(
            settings.defaults.auto_fetch_minutes,
            crate::session::AUTO_FETCH_MAX_MINUTES
        );
    }

    /// One mistyped key costs only itself (rules-refs/core.md「読みは
    /// `toml::Table` からキーごとに取る」).
    #[test]
    fn a_negative_interval_falls_back_to_the_default() {
        let text = "[defaults]\nauto_fetch_minutes = -5\n";
        let settings = Settings::from_table(&text.parse::<Table>().expect("parse"));
        assert_eq!(
            settings.defaults.auto_fetch_minutes,
            Defaults::default().auto_fetch_minutes
        );
    }

    fn commits_from(text: &str) -> Option<u32> {
        Settings::from_table(&text.parse::<Table>().expect("parse"))
            .defaults
            .initial_commits
    }

    /// The whole history and "nobody said" are different answers; collapsed,
    /// a reader who asked for all of it would silently come back to a cut.
    #[test]
    fn the_whole_history_is_zero_in_the_file_and_survives_a_round_trip() {
        assert_eq!(commits_from("[defaults]\ninitial_commits = 0\n"), None);

        let settings = Settings {
            defaults: Defaults {
                initial_commits: None,
                ..Defaults::default()
            },
            ..Settings::default()
        };
        let written = settings.to_table();
        assert_eq!(Settings::from_table(&written), settings, "{written}");
    }

    /// Only a file written by hand can ask for one.
    #[test]
    fn a_count_under_the_floor_is_the_floor() {
        assert_eq!(
            commits_from("[defaults]\ninitial_commits = 1\n"),
            Some(crate::session::MIN_LOG_LIMIT)
        );
    }

    /// The reader asked for as much history as could be had.
    #[test]
    fn a_count_past_u32_is_the_largest_there_is() {
        let text = format!("[defaults]\ninitial_commits = {}\n", i64::MAX);
        assert_eq!(commits_from(&text), Some(u32::MAX));
    }

    fn git_path_from(text: &str) -> String {
        Settings::from_table(&text.parse::<Table>().expect("parse"))
            .defaults
            .git_path
    }

    /// Read back as "nobody said", a cleared path would silently bring the
    /// last binary back on the next write.
    #[test]
    fn an_empty_git_path_is_an_answer_rather_than_a_missing_key() {
        assert_eq!(git_path_from("[defaults]\ngit_path = \"\"\n"), "");

        let settings = Settings {
            defaults: Defaults {
                git_path: String::new(),
                ..Defaults::default()
            },
            ..Settings::default()
        };
        let written = settings.to_table();
        assert_eq!(Settings::from_table(&written), settings, "{written}");
    }

    /// Separators are kept; a trailing space would be part of the program
    /// name the spawner looks for.
    #[test]
    fn a_git_path_is_carried_through_with_the_air_taken_off() {
        assert_eq!(
            git_path_from("[defaults]\ngit_path = \"  C:/tools/git/bin/git.exe  \"\n"),
            "C:/tools/git/bin/git.exe"
        );

        let settings = Settings {
            defaults: Defaults {
                git_path: "/opt/git/bin/git".into(),
                ..Defaults::default()
            },
            ..Settings::default()
        };
        assert_eq!(Settings::from_table(&settings.to_table()), settings);
    }

    #[test]
    fn a_git_path_that_is_not_a_string_falls_back_to_the_default() {
        assert_eq!(git_path_from("[defaults]\ngit_path = 7\n"), "");
    }

    fn defaults_from(text: &str) -> Defaults {
        Settings::from_table(&text.parse::<Table>().expect("parse")).defaults
    }

    /// Zero and anything past the ceiling become the nearest count in
    /// range; a value that is not a count falls back.
    #[test]
    fn a_process_count_is_held_to_its_range_and_survives_a_round_trip() {
        assert_eq!(
            defaults_from("[defaults]\ngit_concurrency = 0\n").git_concurrency,
            1
        );
        assert_eq!(
            defaults_from("[defaults]\ngit_concurrency = 1000\n").git_concurrency,
            crate::process::MAX_CONCURRENCY
        );
        assert_eq!(
            defaults_from("[defaults]\ngit_concurrency = \"many\"\n").git_concurrency,
            Defaults::default().git_concurrency
        );
        let settings = Settings {
            defaults: Defaults {
                git_concurrency: 8,
                ..Defaults::default()
            },
            ..Settings::default()
        };
        assert_eq!(Settings::from_table(&settings.to_table()), settings);
    }

    /// A number under the floor is the floor, so a file cannot ask for a
    /// `status` per copy every second.
    #[test]
    fn the_copies_interval_keeps_off_and_its_floor_through_the_file() {
        assert_eq!(
            defaults_from("[defaults]\ncopies_interval_secs = 0\n").copies_interval_secs,
            0
        );
        assert_eq!(
            defaults_from("[defaults]\ncopies_interval_secs = 1\n").copies_interval_secs,
            crate::session::COPIES_INTERVAL_MIN_SECS
        );
        let settings = Settings {
            defaults: Defaults {
                copies_interval_secs: 0,
                ..Defaults::default()
            },
            ..Settings::default()
        };
        assert_eq!(
            Settings::from_table(&settings.to_table()),
            settings,
            "off survives a round trip"
        );
    }

    #[test]
    fn a_negative_count_falls_back_to_the_default() {
        assert_eq!(
            commits_from("[defaults]\ninitial_commits = -1\n"),
            Defaults::default().initial_commits
        );
        assert_eq!(
            commits_from("[defaults]\ninitial_commits = \"lots\"\n"),
            Defaults::default().initial_commits
        );
    }
}
