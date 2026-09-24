//! The command line `verify-ui` takes.

use std::path::PathBuf;

/// The run's ceiling, and the height every other one here is set from:
/// the app's own watchdog is handed this (`PGG_AUTO_WATCHDOG_MS`), the
/// deadline thread looks past it, and the parent reaps behind them both
/// (`super::child`).
///
/// **A backstop** (.claude/rules/core.md §非同期・並行
/// テスト). The verdict is the words a verb comes back with: a machine
/// running several gates at once slows every verb down, and a ceiling
/// low enough to be reached by that decides by load.
/// The suite's own answer to the same question is
/// `tests/it/support/wait::OVERALL_BUDGET`; this one is held under it
/// because a red costs differently on this side — a verb that reaches
/// the ceiling spends the whole of it in wall clock, and a block whose
/// verbs are all red spends it one verb at a time (`gate::verbs` runs
/// alone until one comes back green). What that buys and what it costs
/// is in internal-docs/反映前テストの機械化.md §動詞の天井.
///
/// Lower it for one run with `--watchdog-ms` when the wait itself is
/// what is being diagnosed; a suite keeps it.
const WATCHDOG_MS: u64 = 600_000;

pub(super) struct Options {
    pub(super) verb: String,
    pub(super) arg: String,
    /// Repositories to open, in tab order. Both flags repeat, because a
    /// gesture on the tab strip needs a strip to land on — the neighbour
    /// staying is what a second tab shows.
    pub(super) repo: Vec<PathBuf>,
    pub(super) preset: Vec<String>,
    pub(super) build: bool,
    pub(super) select: bool,
    /// Park the view at one end once the page has stopped arriving
    /// (`PGG_SCROLL_TO`: `top` / `bottom` / `nav-bottom`). Empty leaves the
    /// view where the run puts it. Like [`Options::system_title_bar`],
    /// the variable alone does not reach the app — `app_env` clears every
    /// `PGG_*` the parent shell carries — so this flag is the only way a
    /// headless run reaches the parking at all.
    pub(super) scroll_to: String,
    /// Ask for the window shape the two platforms that cannot fold the
    /// band into the title bar get (`PGG_SYSTEM_TITLE_BAR`), so the layout
    /// they come up in can be photographed from a machine that merges.
    /// The variable alone does not reach the app: `app_env` clears every
    /// `PGG_*` the parent shell carries from the runs xtask starts, so a
    /// headless run asks for it with this flag alone.
    pub(super) system_title_bar: bool,
    /// Diagnostic ceiling for a run whose causal completion never arrives
    /// ([`WATCHDOG_MS`]).
    pub(super) watchdog_ms: u64,
    /// Hold the app at the station this names, for good (`PGG_FAULT_HANG`
    /// — the words are `harness::deadline`'s). **A run that always
    /// fails**: it is how the two shapes a wedged run comes in are made
    /// to order, so that what the parent reads back can be checked
    /// (`super::faults`).
    pub(super) fault_hang: String,
    /// Start the app with no deadline thread (`PGG_FAULT_NO_DEADLINE`), so
    /// it leaves no report of its own however it is stopped — the shape a
    /// wedge past `exiting` has anyway.
    pub(super) fault_no_deadline: bool,
    /// Swallow the verb's completion (`PGG_FAULT_HOLD_ACT`), so the run
    /// ends at the ceiling with the loop still turning. **A run that
    /// always fails**, and the one shape this fault alone can make: a
    /// ceiling that beats the completion on this machine loses to it on
    /// a busier one (`super::faults`).
    pub(super) fault_hold_act: bool,
    /// Hold every configuration save the app makes until the station
    /// this names (`PGG_FAULT_HOLD_SAVE`), so a close can land on one
    /// provably out and the exit can be read joining it. Armed on its
    /// own for the verb whose subject that is (`super::child`).
    pub(super) fault_hold_save: String,
    /// Stall the parent's own look at a run reaped at the ceiling: the
    /// thread listing is a process that does nothing for longer than the
    /// listing's ceiling (`super::look::look_at`). The one fault that is
    /// the runner's own, and the fourth shape `wedge-check` drives: a
    /// diagnostic that ran out of time, ended, said so, and still
    /// followed by the app's reaping and the verdict.
    pub(super) fault_stall_look: bool,
    pub(super) shot_dir: Option<PathBuf>,
    /// Where the run keeps its settings and state. A fresh directory per
    /// run unless one is named, so a headless run keeps to settings of
    /// its own and leaves this machine's alone.
    pub(super) config_dir: Option<PathBuf>,
    /// Let the app put back the tabs its config directory remembers,
    /// the way a plain launch does.
    pub(super) restore: bool,
    /// Whether a write git refused is part of what the verb is showing.
    pub(super) allow_write_failure: bool,
    /// Run the app against a git that answers `--version` with this and
    /// passes everything else to the real one (`git_shim`). Empty is the
    /// ordinary case: the git this machine has.
    pub(super) old_git: String,
    /// A second git the run may point the settings box at: staged beside
    /// the pictures, off PATH (`shim::stage_other_git`). The version
    /// it answers `--version` with is what is asked for here.
    pub(super) other_git: String,
    /// What the pictures show, for the board, written in the language
    /// the board is read in (`shots::written_label`). Empty falls back
    /// to the verb and its argument, which is a poor name but a true one
    /// — the board keeps the run, named as best it can.
    pub(super) label: String,
    /// Keep this run off the board. For every run nobody asked to look
    /// at — a sweep measuring flakiness, where ten identical pictures
    /// bury what somebody wanted to see, and the runs a suite drives
    /// (`verify::suite_words`), which are a picture per verb per side.
    pub(super) no_board: bool,
    /// Keep this run out of the verb census. For a verb repeated to
    /// measure how steady it is, or run to look at a picture, where
    /// moving a checked-in file is noise the tree then has to be
    /// cleaned of.
    pub(super) no_census: bool,
}

impl Options {
    /// The line the census records a passing run under: what somebody
    /// would type to run it again, options that change the run included
    /// and options that only change where its output goes left out. None
    /// for a run nobody can type again elsewhere (`--repo`, `--restore`).
    ///
    /// **The census is the host's alone.** The container runs on the
    /// host's own checkout over a mount, so a run there would file the
    /// census of another machine, and the two sides of the gate — which
    /// run at the same time — would take the line in
    /// turns.
    pub(super) fn census_line(&self) -> Option<String> {
        if self.no_census
            || !self.repo.is_empty()
            || self.restore
            || self.config_dir.is_some()
            // A run made to wedge shows nothing: it never reaches the
            // walk the census is written off, and the line would be one
            // nobody could type again to a green.
            || !self.fault_hang.is_empty()
            || self.fault_no_deadline
            || self.fault_hold_act
            || std::env::var_os(crate::linux::IN_CONTAINER).is_some()
            // The line is read back one word per argument (`suite_words`),
            // so an argument with a space in it comes back as several and
            // the gate's replay is refused before the app starts.
            || self.arg.contains(char::is_whitespace)
        {
            return None;
        }
        let mut words = vec![self.verb.clone()];
        if !self.arg.is_empty() {
            words.push(self.arg.clone());
        }
        // The flag goes in only where it changes the run. A preset that
        // names exactly what leaving it off would have built is the same
        // run under a second spelling, and the census is keyed by the
        // line: two keys is the verb recorded twice and run twice by
        // every gate that owes it, on both sides
        // (`repos::preset_is_the_default`).
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
/// typed, and `--no-board`.
///
/// A gate takes a picture per selected verb on each side, so the runs
/// somebody asked to look at drown in runs nobody asked for — and `land`
/// sweeps the seat's runs off the board the moment it is done, so what a
/// landing gate files is thrown away unread. The flag rides *in* the
/// line because the container's pictures are filed from the host once
/// the run is over, and `keepsakes` reads the same line to know where
/// they go.
pub(crate) fn suite_words(line: &str) -> Vec<String> {
    let mut words: Vec<String> = line.split_whitespace().map(String::from).collect();
    words.push("--no-board".to_string());
    words
}

/// A path off the command line, pinned to where it was typed.
///
/// The run starts the app in a directory of its own (`run`), so a relative
/// path that reached the child as written would name one place to xtask
/// and another to the app. Resolved here, once, against the directory the
/// command was actually run in.
fn typed_path(raw: &str) -> PathBuf {
    let path = PathBuf::from(raw);
    match path.is_absolute() {
        true => path,
        false => std::env::current_dir().map_or(path, |cwd| cwd.join(raw)),
    }
}

/// The name this run's pictures go up under, held to the board's own
/// rule at the door it is typed at: a run is named in the language the
/// board is read in (`shots::written_label`).
fn typed_label(word: Option<&String>) -> Result<String, String> {
    crate::shots::written_label(word.ok_or("--label needs a phrase")?)
}

pub(super) fn parse(args: &[String]) -> Result<Options, String> {
    let mut opts = Options {
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
    };
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

/// What the positional words mean — the verb and its one argument —
/// and the verb that needs its argument.
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
    // The corner is a question about one pane, and a bare run meaning
    // the working tree's would be the same staging, the same press and
    // the same expected line as naming it — one path run twice.
    // Named, because a default that is one of the answers is the arm a
    // door can stop passing without a run going red
    // (rules-refs/app-ui.md).
    if opts.verb == "corner" && opts.arg.is_empty() {
        return Err("corner needs the pane it is about: wip, or the row to land on".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{parse, suite_words};

    /// A verb line carries its own preset and argument, and the flag goes
    /// last, where the `--no-build` the runner adds can follow it — and
    /// `parse` takes it there, which is what makes the pair one file.
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
        // And the census cannot tell the two apart: the flag says where
        // the pictures go. Compared here, because the answer is None
        // wherever this runs in the container — which is where the
        // gate's Linux side runs it.
        let typed = parse(&words[..words.len() - 1]).expect("the same line, boarded");
        assert!(!typed.no_board);
        assert_eq!(opts.census_line(), typed.census_line());
    }

    /// `--label` is the second door onto the board, `shots add` being
    /// the first, and a name is held to the same rule at both: it is
    /// written in the language the board is read in
    /// (`shots::written_label`). A run filed under a name its reader
    /// cannot read is the picture they asked for, on a page where they
    /// cannot find it.
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

    /// A preset naming what the run would have built anyway is that run
    /// under a second spelling, and the census is keyed by the line: the
    /// two have to come to one key, or the file holds the verb twice and
    /// every gate that owes it runs it twice on each side
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
        // The run itself is untouched: what was named is still what is
        // built, and only the line the census is keyed by is the same.
        assert_eq!(named.preset, ["basic"]);
        assert!(unsaid.preset.is_empty());
        assert_eq!(named.census_line(), unsaid.census_line());
        // The container files no census at all, so the text is only there
        // to be read on the side that does (`census_line`).
        if std::env::var_os(crate::linux::IN_CONTAINER).is_none() {
            assert_eq!(
                named.census_line().as_deref(),
                Some("delete-gone v0.3-local")
            );
        }
    }

    /// An argument with a space in it is a run the census could write
    /// down and never type again: the replay splits the line on
    /// whitespace, and `find` handed three words is refused as too many
    /// positional arguments on both sides of the gate.
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

    /// And a preset that is not that default keeps its words: a second
    /// preset is a second tab, and a verb whose fixture the flag does not
    /// decide is not one this rule has measured — its line stays as typed.
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

    /// The app is started in a directory of its own, so a path that stayed
    /// as it was typed would name one place to xtask and another to the
    /// child — and a run against the wrong repository photographs a window
    /// that opened, which is the failure that passes.
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

        // And one that was already absolute is left exactly as it stands:
        // the container is handed paths of its own (`linux`).
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

    /// The one fault that is the runner's own: it changes nothing
    /// about what the app shows, so the census line is whatever the
    /// verb's is.
    #[test]
    fn a_stalled_look_is_asked_for_and_leaves_the_census_alone() {
        let plain = parse(&["band".to_string()]).expect("verb only");
        assert!(!plain.fault_stall_look);
        let asked =
            parse(&["band".to_string(), "--fault-stall-look".to_string()]).expect("verb and fault");
        assert!(asked.fault_stall_look);
        assert_eq!(asked.census_line(), plain.census_line());
    }

    /// The fault that makes a run stop answering, and the census line it
    /// takes away with it: a run whose act is swallowed never reaches the
    /// walk, so the line would be one nobody could type again to a green.
    #[test]
    fn a_held_act_is_asked_for_and_writes_no_census_line() {
        let plain = parse(&["band".to_string()]).expect("verb only");
        assert!(!plain.fault_hold_act);
        let asked =
            parse(&["band".to_string(), "--fault-hold-act".to_string()]).expect("verb and fault");
        assert!(asked.fault_hold_act);
        assert!(asked.census_line().is_none());
        // The other half only exists where a line does. Every run in the
        // container is off the census by where it runs, so asking there
        // whether the fault took the line away is asking about a line
        // that was never there (`census_line`, and the comparison the
        // board's own test is written as for the same reason).
        if std::env::var_os(crate::linux::IN_CONTAINER).is_none() {
            assert!(plain.census_line().is_some());
        }
    }

    /// The ceiling is a backstop, so the default is set high enough that
    /// a machine running several gates at once cannot reach it: the
    /// verdict is the words a verb comes back with, whatever a busy
    /// machine spends on it. Held under the suite's own backstop
    /// because a red here is paid in wall clock, one verb at a time.
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
