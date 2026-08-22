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
    };
    let mut positional: Vec<&str> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--repo" => opts
                .repo
                .push(PathBuf::from(it.next().ok_or("--repo needs a path")?)),
            "--preset" => opts
                .preset
                .push(it.next().ok_or("--preset needs a name")?.clone()),
            "--no-build" => opts.build = false,
            "--select" => opts.select = true,
            "--label" => opts.label = it.next().ok_or("--label needs a phrase")?.clone(),
            "--no-board" => opts.no_board = true,
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
                opts.shot_dir = Some(PathBuf::from(it.next().ok_or("--shot-dir needs a path")?));
            }
            "--config-dir" => {
                opts.config_dir =
                    Some(PathBuf::from(it.next().ok_or("--config-dir needs a path")?));
            }
            "--restore" => opts.restore = true,
            "--allow-write-failure" => opts.allow_write_failure = true,
            "--old-git" => {
                opts.old_git = it.next().ok_or("--old-git needs a version")?.clone();
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
