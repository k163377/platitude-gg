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
    pub(super) quit_ms: u64,
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
}

pub(super) fn parse(args: &[String]) -> Result<Options, String> {
    let mut opts = Options {
        verb: String::new(),
        arg: String::new(),
        repo: Vec::new(),
        preset: Vec::new(),
        build: true,
        select: false,
        quit_ms: 10_000,
        shot_dir: None,
        config_dir: None,
        restore: false,
        allow_write_failure: false,
        old_git: String::new(),
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
            "--quit-ms" => {
                opts.quit_ms = it
                    .next()
                    .ok_or("--quit-ms needs a number")?
                    .parse()
                    .map_err(|e| format!("--quit-ms: {e}"))?;
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
}
