//! `settings.toml`: what a person decided.

use toml::{Table, Value};

use super::SCHEMA_VERSION;
use super::toml::{
    clamp_to_i64, concurrency, copies, initial_commits, minutes, pace_pair, sub_table, text,
    timeout_secs,
};

/// How the other working copies of a repository are read for uncommitted
/// work (`session::pace`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CopiesReading {
    /// Each copy at an interval its weight sets — what a fresh file starts
    /// with.
    #[default]
    Auto,
    /// Every copy at [`Defaults::copies_interval_secs`].
    Fixed,
    Off,
}

impl CopiesReading {
    /// As the file spells it.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Fixed => "fixed",
            Self::Off => "off",
        }
    }

    #[must_use]
    pub fn from_word(word: &str) -> Option<Self> {
        match word.trim() {
            "auto" => Some(Self::Auto),
            "fixed" => Some(Self::Fixed),
            "off" => Some(Self::Off),
            _ => None,
        }
    }
}

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
    /// How the other working copies of a repository are read — one
    /// `git status` per copy per read (`session::carried`).
    pub copies_reading: CopiesReading,
    /// Seconds between reads of each other copy where the reading is
    /// [`CopiesReading::Fixed`]; kept while another is chosen, so choosing
    /// it again brings the number back.
    pub copies_interval_secs: u32,
    /// The shortest and the longest interval a repository on screen is read
    /// again at (`session::pace`), through `session::pace_bounds_secs`.
    pub refresh_floor_secs: u32,
    pub refresh_ceiling_secs: u32,
    /// The same pair for each other copy read automatically.
    pub copies_floor_secs: u32,
    pub copies_ceiling_secs: u32,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            auto_fetch_minutes: crate::session::AUTO_FETCH_DEFAULT_MINUTES,
            network_timeout_secs: crate::remote::DEFAULT_NETWORK_TIMEOUT.as_secs(),
            initial_commits: Some(crate::session::DEFAULT_LOG_LIMIT),
            git_path: String::new(),
            git_concurrency: crate::process::default_concurrency(),
            copies_reading: CopiesReading::default(),
            copies_interval_secs: crate::session::COPIES_INTERVAL_DEFAULT_SECS,
            refresh_floor_secs: secs_of(crate::session::OWN_FLOOR),
            refresh_ceiling_secs: secs_of(crate::session::OWN_CEILING),
            copies_floor_secs: secs_of(crate::session::COPY_FLOOR),
            copies_ceiling_secs: secs_of(crate::session::COPY_CEILING),
        }
    }
}

fn secs_of(duration: std::time::Duration) -> u32 {
    u32::try_from(duration.as_secs()).unwrap_or(u32::MAX)
}

impl Defaults {
    /// The floors and ceilings the session paces its reads by.
    #[must_use]
    pub fn pace_bounds(&self) -> crate::session::PaceBounds {
        let secs = |s: u32| std::time::Duration::from_secs(s.into());
        crate::session::PaceBounds {
            own_floor: secs(self.refresh_floor_secs),
            own_ceiling: secs(self.refresh_ceiling_secs),
            copy_floor: secs(self.copies_floor_secs),
            copy_ceiling: secs(self.copies_ceiling_secs),
        }
    }

    /// What the session paces the other copies by.
    #[must_use]
    pub fn copies_pace(&self) -> crate::session::CopiesPace {
        match self.copies_reading {
            CopiesReading::Auto => crate::session::CopiesPace::Auto,
            CopiesReading::Fixed => crate::session::CopiesPace::Fixed(
                std::time::Duration::from_secs(self.copies_interval_secs.into()),
            ),
            CopiesReading::Off => crate::session::CopiesPace::Off,
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
            Some(t) => {
                let (copies_reading, copies_interval_secs) =
                    copies(t, (fallback.copies_reading, fallback.copies_interval_secs));
                let (refresh_floor_secs, refresh_ceiling_secs) = pace_pair(
                    t,
                    ("refresh_floor_secs", "refresh_ceiling_secs"),
                    (fallback.refresh_floor_secs, fallback.refresh_ceiling_secs),
                );
                let (copies_floor_secs, copies_ceiling_secs) = pace_pair(
                    t,
                    ("copies_floor_secs", "copies_ceiling_secs"),
                    (fallback.copies_floor_secs, fallback.copies_ceiling_secs),
                );
                Defaults {
                    auto_fetch_minutes: minutes(t, "auto_fetch_minutes")
                        .unwrap_or(fallback.auto_fetch_minutes),
                    network_timeout_secs: timeout_secs(t, "network_timeout_secs")
                        .unwrap_or(fallback.network_timeout_secs),
                    initial_commits: initial_commits(t, "initial_commits")
                        .unwrap_or(fallback.initial_commits),
                    git_path: text(t, "git_path").unwrap_or(fallback.git_path),
                    git_concurrency: concurrency(t, "git_concurrency")
                        .unwrap_or(fallback.git_concurrency),
                    copies_reading,
                    copies_interval_secs,
                    refresh_floor_secs,
                    refresh_ceiling_secs,
                    copies_floor_secs,
                    copies_ceiling_secs,
                }
            }
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
        // Both, always: a file without the reading is read as one written
        // before it was a choice of its own (`toml::copies`).
        defaults.insert(
            "copies_reading".into(),
            Value::String(self.defaults.copies_reading.word().into()),
        );
        defaults.insert(
            "copies_interval_secs".into(),
            Value::Integer(self.defaults.copies_interval_secs.into()),
        );
        for (key, secs) in [
            ("refresh_floor_secs", self.defaults.refresh_floor_secs),
            ("refresh_ceiling_secs", self.defaults.refresh_ceiling_secs),
            ("copies_floor_secs", self.defaults.copies_floor_secs),
            ("copies_ceiling_secs", self.defaults.copies_ceiling_secs),
        ] {
            defaults.insert(key.into(), Value::Integer(secs.into()));
        }
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

    fn copies_from(text: &str) -> (CopiesReading, u32) {
        let defaults = defaults_from(text);
        (defaults.copies_reading, defaults.copies_interval_secs)
    }

    /// A file from before the reading was a choice of its own keeps what
    /// its number said; only a file that says nothing reads automatically.
    #[test]
    fn a_file_from_before_the_reading_keeps_its_off_and_its_number() {
        assert_eq!(
            copies_from("[defaults]\ncopies_interval_secs = 0\n"),
            (
                CopiesReading::Off,
                crate::session::COPIES_INTERVAL_DEFAULT_SECS
            ),
            "zero was off, and the number to bring back is the default"
        );
        assert_eq!(
            copies_from(&format!(
                "[defaults]\ncopies_interval_secs = {}\n",
                crate::session::COPIES_INTERVAL_DEFAULT_SECS
            )),
            (
                CopiesReading::Fixed,
                crate::session::COPIES_INTERVAL_DEFAULT_SECS
            ),
            "a number equal to the old default is a fixed choice all the same"
        );
        assert_eq!(
            copies_from("[defaults]\ncopies_interval_secs = 3600\n"),
            (CopiesReading::Fixed, 3600)
        );
        assert_eq!(
            copies_from("[defaults]\nauto_fetch_minutes = 5\n"),
            (
                CopiesReading::Auto,
                crate::session::COPIES_INTERVAL_DEFAULT_SECS
            ),
            "a file that says nothing reads automatically"
        );
    }

    /// A number under the floor is the floor, so a file cannot ask for a
    /// `status` per copy every second; a reading the file names wins over
    /// what its number would have said.
    #[test]
    fn the_floors_and_ceilings_survive_the_file_and_a_ceiling_never_sits_under_its_floor() {
        let defaults = defaults_from(
            "[defaults]\nrefresh_floor_secs = 2\nrefresh_ceiling_secs = 30\n\
             copies_floor_secs = 20\ncopies_ceiling_secs = 10\n",
        );
        assert_eq!(
            (defaults.refresh_floor_secs, defaults.refresh_ceiling_secs),
            (2, 30)
        );
        assert_eq!(
            (defaults.copies_floor_secs, defaults.copies_ceiling_secs),
            (20, 20),
            "a ceiling written under its floor is the floor"
        );
        let fresh = defaults_from("[defaults]\nauto_fetch_minutes = 5\n");
        assert_eq!(fresh.pace_bounds(), crate::session::PaceBounds::default());
        let settings = Settings {
            defaults: Defaults {
                refresh_floor_secs: 1,
                refresh_ceiling_secs: 90,
                copies_floor_secs: 7,
                copies_ceiling_secs: 600,
                ..Defaults::default()
            },
            ..Settings::default()
        };
        assert_eq!(Settings::from_table(&settings.to_table()), settings);
    }

    #[test]
    fn the_copies_reading_and_its_number_survive_the_file() {
        assert_eq!(
            copies_from("[defaults]\ncopies_reading = \"fixed\"\ncopies_interval_secs = 1\n"),
            (
                CopiesReading::Fixed,
                crate::session::COPIES_INTERVAL_MIN_SECS
            )
        );
        assert_eq!(
            copies_from("[defaults]\ncopies_reading = \"auto\"\ncopies_interval_secs = 0\n"),
            (
                CopiesReading::Auto,
                crate::session::COPIES_INTERVAL_DEFAULT_SECS
            )
        );
        assert_eq!(
            copies_from("[defaults]\ncopies_reading = \"sometimes\"\ncopies_interval_secs = 20\n"),
            (CopiesReading::Fixed, 20),
            "a reading that is no word falls back to what the number says"
        );
        for reading in [
            CopiesReading::Auto,
            CopiesReading::Fixed,
            CopiesReading::Off,
        ] {
            let settings = Settings {
                defaults: Defaults {
                    copies_reading: reading,
                    copies_interval_secs: 90,
                    ..Defaults::default()
                },
                ..Settings::default()
            };
            assert_eq!(
                Settings::from_table(&settings.to_table()),
                settings,
                "{reading:?} survives a round trip with its number"
            );
        }
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
