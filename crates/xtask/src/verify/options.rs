//! The command line `verify-ui` takes.

use std::path::PathBuf;

/// The run's ceiling, and the height every other one here is set from:
/// the app's own watchdog is handed this (`PGG_AUTO_WATCHDOG_MS`), the
/// deadline thread looks past it, and the parent reaps behind them both
/// (`super::child`).
///
/// A backstop, high enough that load cannot reach it and held under
/// `tests/it/support/wait::OVERALL_BUDGET` because a red here is paid in
/// wall clock (internal-docs/反映前テストの機械化.md §動詞の天井). Lower it
/// with `--watchdog-ms` only to diagnose the wait itself.
const WATCHDOG_MS: u64 = 600_000;

pub(super) struct Options {
    pub(super) verb: String,
    pub(super) arg: String,
    /// Repositories to open, in tab order. Both flags repeat: a gesture on
    /// the tab strip needs a second tab.
    pub(super) repo: Vec<PathBuf>,
    pub(super) preset: Vec<String>,
    pub(super) build: bool,
    pub(super) select: bool,
    /// Park the view at one end once the page has stopped arriving
    /// (`PGG_SCROLL_TO`: `top` / `bottom` / `nav-bottom`); empty leaves it
    /// where the run puts it. A flag because `app_env` clears the parent
    /// shell's `PGG_*`, so the variable alone never reaches the app.
    pub(super) scroll_to: String,
    /// Ask for the window shape of the two platforms that cannot fold the
    /// band into the title bar (`PGG_SYSTEM_TITLE_BAR`), to photograph it
    /// from a machine that merges. A flag for the same reason as
    /// [`Options::scroll_to`].
    pub(super) system_title_bar: bool,
    /// Diagnostic ceiling for a run whose causal completion never arrives
    /// ([`WATCHDOG_MS`]).
    pub(super) watchdog_ms: u64,
    /// Hold the app at the station this names, for good (`PGG_FAULT_HANG`;
    /// the words are `harness::deadline`'s). A run that always fails, for
    /// `super::faults`.
    pub(super) fault_hang: String,
    /// Start the app with no deadline thread (`PGG_FAULT_NO_DEADLINE`), so
    /// it leaves no report of its own — the shape a wedge past `exiting`
    /// has anyway.
    pub(super) fault_no_deadline: bool,
    /// Swallow the verb's completion (`PGG_FAULT_HOLD_ACT`), so the run
    /// ends at the ceiling with the loop still turning. A run that always
    /// fails, for `super::faults`.
    pub(super) fault_hold_act: bool,
    /// Hold every configuration save the app makes until the station
    /// this names (`PGG_FAULT_HOLD_SAVE`), so a close can land on one
    /// provably out and the exit can be read joining it. Set unasked for
    /// the verb whose subject that is (`super::child`).
    pub(super) fault_hold_save: String,
    /// Stall the parent's own look at a run reaped at the ceiling
    /// (`super::look::look_at`) — the one fault that is the runner's own,
    /// driven by `wedge-check`.
    pub(super) fault_stall_look: bool,
    pub(super) shot_dir: Option<PathBuf>,
    /// Where the run keeps its settings and state: a fresh directory per
    /// run unless one is named, so this machine's are left alone.
    pub(super) config_dir: Option<PathBuf>,
    /// Let the app put back the tabs its config directory remembers,
    /// the way a plain launch does.
    pub(super) restore: bool,
    /// Whether a write git refused is part of what the verb is showing.
    pub(super) allow_write_failure: bool,
    /// Run the app against a git that answers `--version` with this and
    /// passes everything else to the real one (`git_shim`). Empty: the git
    /// this machine has.
    pub(super) old_git: String,
    /// A second git the run may point the settings box at, answering
    /// `--version` with this: staged beside the pictures, off PATH
    /// (`shim::stage_other_git`).
    pub(super) other_git: String,
    /// What the pictures show, for the board, in the language the board
    /// is read in (`shots::written_label`). Empty falls back to the verb
    /// and its argument.
    pub(super) label: String,
    /// Keep this run off the board: for runs nobody asked to look at — a
    /// flakiness sweep, or the runs a suite drives (`verify::suite_words`).
    pub(super) no_board: bool,
    /// Keep this run out of the verb census: for a verb repeated to
    /// measure how steady it is, or run to look at a picture, where moving
    /// a checked-in file is noise.
    pub(super) no_census: bool,
}

impl Options {
    /// The line the census records a passing run under: what somebody
    /// would type to run it again, less the options that only move its
    /// output. None for a run nobody can type again elsewhere (`--repo`,
    /// `--restore`).
    ///
    /// The census is the host's alone: the container runs on the host's
    /// checkout over a mount, and the two sides of the gate would take
    /// the line in turns.
    pub(super) fn census_line(&self) -> Option<String> {
        if self.no_census
            || !self.repo.is_empty()
            || self.restore
            || self.config_dir.is_some()
            // A run made to wedge never reaches the walk the census is
            // written off.
            || !self.fault_hang.is_empty()
            || self.fault_no_deadline
            || self.fault_hold_act
            || std::env::var_os(crate::linux::IN_CONTAINER).is_some()
            // Read back one word per argument (`suite_words`): an argument
            // with a space comes back as several and the replay is refused.
            || self.arg.contains(char::is_whitespace)
        {
            return None;
        }
        let mut words = vec![self.verb.clone()];
        if !self.arg.is_empty() {
            words.push(self.arg.clone());
        }
        // A preset that builds what leaving it off would is left out: the
        // census is keyed by the line (`repos::preset_is_the_default`).
        if !super::repos::preset_is_the_default(&self.verb, &self.arg, &self.preset) {
            for preset in &self.preset {
                words.push("--preset".to_string());
                words.push(preset.clone());
            }
        }
        if self.select {
            words.push("--select".to_string());
        }
        if !self.scroll_to.is_empty() {
            words.push("--scroll-to".to_string());
            words.push(self.scroll_to.clone());
        }
        if self.system_title_bar {
            words.push("--system-title-bar".to_string());
        }
        if self.allow_write_failure {
            words.push("--allow-write-failure".to_string());
        }
        if !self.old_git.is_empty() {
            words.push("--old-git".to_string());
            words.push(self.old_git.clone());
        }
        if !self.other_git.is_empty() {
            words.push("--other-git".to_string());
            words.push(self.other_git.clone());
        }
        Some(words.join(" "))
    }
}

/// The words after `verify-ui` when a suite drives the run: the line as
/// typed, and `--no-board` — a suite's pictures would drown the ones
/// somebody asked for. The flag rides in the line because `keepsakes`
/// files the container's pictures from the host off the same line.
pub(crate) fn suite_words(line: &str) -> Vec<String> {
    let mut words: Vec<String> = line.split_whitespace().map(String::from).collect();
    words.push("--no-board".to_string());
    words
}

/// A path off the command line, pinned to where it was typed: the app
/// starts in a directory of its own (`run`), where a relative path would
/// name another place.
fn typed_path(raw: &str) -> PathBuf {
    let path = PathBuf::from(raw);
    match path.is_absolute() {
        true => path,
        false => std::env::current_dir().map_or(path, |cwd| cwd.join(raw)),
    }
}

/// The name this run's pictures go up under, held to the board's rule
/// where it is typed (`shots::written_label`).
fn typed_label(word: Option<&String>) -> Result<String, String> {
    crate::shots::written_label(word.ok_or("--label needs a phrase")?)
}

/// A run the command line says nothing about but its verb: built, and
/// under the stock ceiling.
impl Default for Options {
    fn default() -> Self {
        Options {
            verb: String::new(),
            arg: String::new(),
            repo: Vec::new(),
            preset: Vec::new(),
            build: true,
            select: false,
            scroll_to: String::new(),
            system_title_bar: false,
            watchdog_ms: WATCHDOG_MS,
            fault_hang: String::new(),
            fault_no_deadline: false,
            fault_hold_act: false,
            fault_hold_save: String::new(),
            fault_stall_look: false,
            shot_dir: None,
            config_dir: None,
            restore: false,
            allow_write_failure: false,
            old_git: String::new(),
            other_git: String::new(),
            label: String::new(),
            no_board: false,
            no_census: false,
        }
    }
}

pub(super) fn parse(args: &[String]) -> Result<Options, String> {
    let mut opts = Options::default();
    let mut positional: Vec<&str> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--repo" => opts
                .repo
                .push(typed_path(it.next().ok_or("--repo needs a path")?)),
            "--preset" => opts
                .preset
                .push(it.next().ok_or("--preset needs a name")?.clone()),
            "--no-build" => opts.build = false,
            "--select" => opts.select = true,
            "--scroll-to" => {
                let where_to = it.next().ok_or("--scroll-to needs top/bottom/nav-bottom")?;
                if !matches!(where_to.as_str(), "top" | "bottom" | "nav-bottom") {
                    return Err(format!(
                        "--scroll-to takes top, bottom or nav-bottom; got {where_to:?}"
                    ));
                }
                opts.scroll_to = where_to.clone();
            }
            "--system-title-bar" => opts.system_title_bar = true,
            "--label" => opts.label = typed_label(it.next())?,
            "--no-board" => opts.no_board = true,
            "--no-census" => opts.no_census = true,
            "--quit-ms" => {
                return Err(
                    "unknown verify-ui option: --quit-ms (shots follow the act's completion; use --watchdog-ms only as a diagnostic ceiling)"
                        .into(),
                );
            }
            "--watchdog-ms" => {
                opts.watchdog_ms = it
                    .next()
                    .ok_or("--watchdog-ms needs a number")?
                    .parse()
                    .map_err(|e| format!("--watchdog-ms: {e}"))?;
            }
            "--fault-hang" => {
                opts.fault_hang = it
                    .next()
                    .ok_or("--fault-hang needs a station (e.g. exiting)")?
                    .clone();
            }
            "--fault-no-deadline" => opts.fault_no_deadline = true,
            "--fault-hold-act" => opts.fault_hold_act = true,
            "--fault-hold-save" => {
                opts.fault_hold_save = it
                    .next()
                    .ok_or("--fault-hold-save needs a station (e.g. writes-joining)")?
                    .clone();
            }
            "--fault-stall-look" => opts.fault_stall_look = true,
            "--shot-dir" => {
                opts.shot_dir = Some(typed_path(it.next().ok_or("--shot-dir needs a path")?));
            }
            "--config-dir" => {
                opts.config_dir = Some(typed_path(it.next().ok_or("--config-dir needs a path")?));
            }
            "--restore" => opts.restore = true,
            "--allow-write-failure" => opts.allow_write_failure = true,
            "--old-git" => {
                opts.old_git = it.next().ok_or("--old-git needs a version")?.clone();
            }
            "--other-git" => {
                opts.other_git = it.next().ok_or("--other-git needs a version")?.clone();
            }
            // A misspelled flag stops the run here: riding on as a verb
            // argument, it would go out under conditions nobody asked for.
            other if other.starts_with("--") => {
                return Err(format!("unknown verify-ui option {other}"));
            }
            other => positional.push(other),
        }
    }
    name_the_verb(&mut opts, &positional)?;
    Ok(opts)
}

/// The verb and its one argument off the positional words, and the verbs
/// refused without theirs.
fn name_the_verb(opts: &mut Options, positional: &[&str]) -> Result<(), String> {
    match positional {
        [] => return Err("verify-ui needs a verb (see `cargo xtask`)".into()),
        [verb] => opts.verb = (*verb).to_string(),
        [verb, arg] => {
            opts.verb = (*verb).to_string();
            opts.arg = (*arg).to_string();
        }
        more => return Err(format!("too many positional arguments: {more:?}")),
    }
    if opts.verb == "publish-new-go"
        && !opts
            .arg
            .split_once('|')
            .is_some_and(|(_, url)| !url.trim().is_empty())
    {
        return Err("publish-new-go needs <name>|<url> with a non-empty URL".into());
    }
    // No default pane: a bare run meaning `wip` would be one path run
    // twice, and a default that is one of the answers can stop being
    // passed without going red (rules-refs/app-ui.md「その kind だけが持つ側」).
    if opts.verb == "corner" && opts.arg.is_empty() {
        return Err("corner needs the pane it is about: wip, or the row to land on".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{parse, suite_words};

    /// The flag goes last, where the runner's `--no-build` can follow it,
    /// and `parse` takes it there.
    #[test]
    fn a_suite_run_stays_off_the_board() {
        let words = suite_words("co-authors 4 --preset co-authors");
        assert_eq!(
            words,
            ["co-authors", "4", "--preset", "co-authors", "--no-board"].map(String::from)
        );
        let opts = parse(&words).expect("the line a suite hands verify-ui");
        assert!(opts.no_board);
        assert_eq!(opts.verb, "co-authors");
        assert_eq!(opts.arg, "4");
        assert_eq!(opts.preset, ["co-authors"]);
        // The census cannot tell the two apart. Compared, not spelled: in
        // the container both are None.
        let typed = parse(&words[..words.len() - 1]).expect("the same line, boarded");
        assert!(!typed.no_board);
        assert_eq!(opts.census_line(), typed.census_line());
    }

    /// `--label` is the second door onto the board after `shots add`, and
    /// a name is held to the same rule at both (`shots::written_label`).
    #[test]
    fn a_run_is_named_in_the_language_the_board_is_read_in() {
        let named = parse(&[
            "tab-carry".to_string(),
            "--label".to_string(),
            "チップの余白".to_string(),
        ])
        .expect("a verb and the name its pictures go up under");
        assert_eq!(named.label, "チップの余白");
        let Err(refusal) = parse(&[
            "tab-carry".to_string(),
            "--label".to_string(),
            "chip padding".to_string(),
        ]) else {
            panic!("a name written past the rule is refused where it is typed");
        };
        assert!(
            refusal.contains("Japanese"),
            "a refusal has to say the rule it is holding to: {refusal}"
        );
    }

    /// Two keys would be the verb run twice by every gate that owes it
    /// (`repos::preset_is_the_default`).
    #[test]
    fn the_default_preset_named_and_left_off_is_one_census_line() {
        let named = parse(&[
            "delete-gone".to_string(),
            "v0.3-local".to_string(),
            "--preset".to_string(),
            "basic".to_string(),
        ])
        .expect("a verb, its argument and the default preset");
        let unsaid = parse(&["delete-gone".to_string(), "v0.3-local".to_string()])
            .expect("the same run with the flag left off");
        // The run itself is untouched: only the census line is the same.
        assert_eq!(named.preset, ["basic"]);
        assert!(unsaid.preset.is_empty());
        assert_eq!(named.census_line(), unsaid.census_line());
        // The container files no census (`census_line`).
        if std::env::var_os(crate::linux::IN_CONTAINER).is_none() {
            assert_eq!(
                named.census_line().as_deref(),
                Some("delete-gone v0.3-local")
            );
        }
    }

    /// The replay splits the line on whitespace, so `find` would come back
    /// with three words and be refused.
    #[test]
    fn an_argument_with_a_space_is_no_line_for_the_census() {
        let spaced = parse(&[
            "find".to_string(),
            "filler commit 150".to_string(),
            "--preset".to_string(),
            "deep".to_string(),
        ])
        .expect("a query with spaces, typed as one argument");
        assert_eq!(spaced.arg, "filler commit 150");
        assert_eq!(spaced.census_line(), None);
        let Err(refusal) = parse(&suite_words("find filler commit 150 --preset deep")) else {
            panic!("the line read back is not the run that was recorded");
        };
        assert!(refusal.contains("positional"), "{refusal}");
    }

    /// A second preset is a second tab, and a verb that builds its own
    /// fixture is outside the rule: both lines stay as typed.
    #[test]
    fn a_preset_the_default_does_not_answer_for_stays_in_the_line() {
        let two = parse(&[
            "tab-carry".to_string(),
            "--preset".to_string(),
            "basic".to_string(),
            "--preset".to_string(),
            "stashes".to_string(),
        ])
        .expect("two presets, two tabs");
        let strip = parse(&[
            "tab-widths".to_string(),
            "short".to_string(),
            "--preset".to_string(),
            "basic".to_string(),
        ])
        .expect("a verb that builds its own strip");
        if std::env::var_os(crate::linux::IN_CONTAINER).is_none() {
            assert_eq!(
                two.census_line().as_deref(),
                Some("tab-carry --preset basic --preset stashes")
            );
            assert_eq!(
                strip.census_line().as_deref(),
                Some("tab-widths short --preset basic")
            );
        }
    }

    /// A run against the wrong repository still photographs a window that
    /// opened — the failure that passes.
    #[test]
    fn a_path_off_the_command_line_is_pinned_where_it_was_typed() {
        let opts = parse(&[
            "commit".to_string(),
            "--repo".to_string(),
            "demo".to_string(),
            "--shot-dir".to_string(),
            "shots".to_string(),
            "--config-dir".to_string(),
            "conf".to_string(),
        ])
        .expect("a verb and three paths");
        let shot = opts.shot_dir.as_ref().expect("--shot-dir");
        let config = opts.config_dir.as_ref().expect("--config-dir");
        for path in [&opts.repo[0], shot, config] {
            assert!(path.is_absolute(), "{} stayed relative", path.display());
        }
        assert!(opts.repo[0].ends_with("demo"));

        // An absolute one is left as it stands: the container is handed
        // paths of its own (`linux`).
        let typed = if cfg!(windows) {
            "C:\\tmp\\demo"
        } else {
            "/tmp/demo"
        };
        let kept = parse(&[
            "commit".to_string(),
            "--repo".to_string(),
            typed.to_string(),
        ])
        .expect("an absolute path");
        assert_eq!(kept.repo[0], std::path::Path::new(typed));
    }

    #[test]
    fn the_flag_is_off_until_it_is_asked_for() {
        let plain = parse(&["commit".to_string()]).expect("verb only");
        assert!(!plain.allow_write_failure);
        let asked = parse(&[
            "fetch-fail".to_string(),
            "3".to_string(),
            "--allow-write-failure".to_string(),
        ])
        .expect("verb, arg and flag");
        assert!(asked.allow_write_failure);
        assert_eq!(asked.verb, "fetch-fail");
        assert_eq!(asked.arg, "3");
    }

    /// The runner's own fault changes nothing the app shows, so the
    /// census line is the verb's.
    #[test]
    fn a_stalled_look_is_asked_for_and_leaves_the_census_alone() {
        let plain = parse(&["band".to_string()]).expect("verb only");
        assert!(!plain.fault_stall_look);
        let asked =
            parse(&["band".to_string(), "--fault-stall-look".to_string()]).expect("verb and fault");
        assert!(asked.fault_stall_look);
        assert_eq!(asked.census_line(), plain.census_line());
    }

    #[test]
    fn a_held_act_is_asked_for_and_writes_no_census_line() {
        let plain = parse(&["band".to_string()]).expect("verb only");
        assert!(!plain.fault_hold_act);
        let asked =
            parse(&["band".to_string(), "--fault-hold-act".to_string()]).expect("verb and fault");
        assert!(asked.fault_hold_act);
        assert!(asked.census_line().is_none());
        // In the container no run has a line (`census_line`).
        if std::env::var_os(crate::linux::IN_CONTAINER).is_none() {
            assert!(plain.census_line().is_some());
        }
    }

    #[test]
    fn the_watchdog_is_the_only_time_ceiling() {
        let plain = parse(&["commit".to_string()]).expect("verb only");
        assert_eq!(plain.watchdog_ms, 600_000);

        let asked = parse(&[
            "commit".to_string(),
            "--watchdog-ms".to_string(),
            "9000".to_string(),
        ])
        .expect("diagnostic ceiling");
        assert_eq!(asked.watchdog_ms, 9000);
    }

    #[test]
    fn the_removed_shot_clock_fails_fast() {
        let result = parse(&[
            "commit".to_string(),
            "--quit-ms".to_string(),
            "2500".to_string(),
        ]);
        let Err(err) = result else {
            panic!("wall-clock screenshot selection must stay removed");
        };
        assert!(err.contains("unknown verify-ui option: --quit-ms"));
    }

    #[test]
    fn publish_new_go_fails_before_launch_without_a_remote_url() {
        for args in [
            vec!["publish-new-go".to_string()],
            vec!["publish-new-go".to_string(), "origin|".to_string()],
        ] {
            let Err(error) = parse(&args) else {
                panic!("an unanswerable dialog is refused before the launch")
            };
            assert!(error.contains("non-empty URL"));
        }
    }
}
