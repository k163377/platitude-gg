//! The command line `verify-ui` takes.

use std::path::PathBuf;

pub(super) struct Options {
    pub(super) verb: String,
    pub(super) arg: String,
    /// Repositories to open, in tab order. Both flags repeat, because a
    /// gesture on the tab strip needs a strip to land on — one tab can
    /// only show that a tab closed, never that the neighbour stayed.
    pub(super) repo: Vec<PathBuf>,
    pub(super) preset: Vec<String>,
    pub(super) build: bool,
    pub(super) select: bool,
    /// Diagnostic ceiling for a run whose causal completion never arrives.
    pub(super) watchdog_ms: u64,
    pub(super) shot_dir: Option<PathBuf>,
    /// Where the run keeps its settings and state. A fresh directory per
    /// run unless one is named, so a headless run never reads or writes
    /// the settings of whoever is sitting at this machine.
    pub(super) config_dir: Option<PathBuf>,
    /// Let the app put back the tabs its config directory remembers,
    /// instead of being told which repository to open.
    pub(super) restore: bool,
    /// Whether a write git refused is part of what the verb is showing.
    pub(super) allow_write_failure: bool,
    /// Run the app against a git that answers `--version` with this and
    /// passes everything else to the real one (`git_shim`). Empty is the
    /// ordinary case: the git this machine has.
    pub(super) old_git: String,
    /// What the pictures show, for the board. Empty falls back to the
    /// verb and its argument, which is a poor name but a true one — the
    /// board would rather hold a weakly named run than lose the run.
    pub(super) label: String,
    /// Keep this run off the board. For a sweep measuring flakiness,
    /// where ten identical pictures bury what somebody wanted to look at.
    pub(super) no_board: bool,
    /// Do not record what the run showed in the verb census. For the runs
    /// that are not about what the line shows — a verb repeated to measure
    /// how steady it is, or to look at a picture — where moving a
    /// checked-in file is noise the tree then has to be cleaned of.
    pub(super) no_census: bool,
}

impl Options {
    /// The line the census records a passing run under: what somebody
    /// would type to run it again, options that change the run included
    /// and options that only change where its output goes left out. None
    /// for a run nobody can type again elsewhere (`--repo`, `--restore`).
    ///
    /// **The container never records.** It runs against the host's own
    /// checkout over a mount, so a run there would write the census of a
    /// machine that is not the one the file follows, and the two sides of
    /// the gate — which run at the same time — would take the line in
    /// turns.
    pub(super) fn census_line(&self) -> Option<String> {
        if self.no_census
            || !self.repo.is_empty()
            || self.restore
            || self.config_dir.is_some()
            || std::env::var_os(crate::linux::IN_CONTAINER).is_some()
        {
            return None;
        }
        let mut words = vec![self.verb.clone()];
        if !self.arg.is_empty() {
            words.push(self.arg.clone());
        }
        for preset in &self.preset {
            words.push("--preset".to_string());
            words.push(preset.clone());
        }
        if self.select {
            words.push("--select".to_string());
        }
        if self.allow_write_failure {
            words.push("--allow-write-failure".to_string());
        }
        if !self.old_git.is_empty() {
            words.push("--old-git".to_string());
            words.push(self.old_git.clone());
        }
        Some(words.join(" "))
    }
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

pub(super) fn parse(args: &[String]) -> Result<Options, String> {
    let mut opts = Options {
        verb: String::new(),
        arg: String::new(),
        repo: Vec::new(),
        preset: Vec::new(),
        build: true,
        select: false,
        watchdog_ms: 120_000,
        shot_dir: None,
        config_dir: None,
        restore: false,
        allow_write_failure: false,
        old_git: String::new(),
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
            "--label" => opts.label = it.next().ok_or("--label needs a phrase")?.clone(),
            "--no-board" => opts.no_board = true,
            "--no-census" => opts.no_census = true,
            "--quit-ms" => {
                return Err(
                    "unknown verify-ui option: --quit-ms (shots are not picked by wall clock; use --watchdog-ms only as a diagnostic ceiling)"
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
            // A misspelled flag must not ride on as a verb argument — the
            // run would go out under different conditions than asked for.
            other if other.starts_with("--") => {
                return Err(format!("unknown verify-ui option {other}"));
            }
            other => positional.push(other),
        }
    }
    match positional.as_slice() {
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
    Ok(opts)
}

#[cfg(test)]
mod tests {
    use super::parse;

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

    #[test]
    fn the_watchdog_is_the_only_time_ceiling() {
        let plain = parse(&["commit".to_string()]).expect("verb only");
        assert_eq!(plain.watchdog_ms, 120_000);

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
                panic!("an unanswerable dialog must not run until the watchdog")
            };
            assert!(error.contains("non-empty URL"));
        }
    }
}
