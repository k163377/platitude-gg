//! Performance scenario options.

use std::path::PathBuf;

pub(super) struct Options {
    pub(super) repo: PathBuf,
    pub(super) label: String,
    pub(super) runs: u32,
    pub(super) watchdog_ms: u64,
    /// How long to hold the app after `perf_done` before reading the
    /// memory one last time, or 0 to read it at once.
    ///
    /// What it is for: the peak says what the work cost while it ran, and
    /// on its own it cannot tell a process that is still holding the
    /// bytes from one that has already handed them back. The bench
    /// finishes and the harness ends in the same breath, so without this
    /// the two look identical.
    pub(super) settle_ms: u64,
    pub(super) scroll: bool,
    pub(super) select: bool,
    pub(super) breakdown: bool,
    pub(super) build: bool,
    /// Start with no repository at all — the window and nothing in it.
    /// What it is for: subtracting this from a run that opened an empty
    /// repository leaves the cost of putting the page up, which is
    /// otherwise indistinguishable from the toolkit's own floor.
    pub(super) open: bool,
}

pub(super) fn parse(args: &[String]) -> Result<Options, String> {
    let mut opts = Options {
        repo: PathBuf::new(),
        label: String::new(),
        runs: 3,
        watchdog_ms: 300_000,
        settle_ms: 0,
        scroll: true,
        select: true,
        breakdown: false,
        build: true,
        open: true,
    };
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        let mut value = || {
            i += 1;
            args.get(i)
                .cloned()
                .ok_or_else(|| format!("{arg} needs a value"))
        };
        match arg {
            "--repo" => opts.repo = PathBuf::from(value()?),
            "--label" => opts.label = value()?,
            "--runs" => {
                opts.runs = value()?
                    .parse()
                    .map_err(|_| "--runs takes a number".to_string())?;
            }
            "--watchdog-ms" => {
                opts.watchdog_ms = value()?
                    .parse()
                    .map_err(|_| "--watchdog-ms takes a number".to_string())?;
            }
            "--settle-ms" => {
                opts.settle_ms = value()?
                    .parse()
                    .map_err(|_| "--settle-ms takes a number".to_string())?;
            }
            "--quit-ms" => {
                return Err(
                    "--quit-ms is not supported; use --watchdog-ms as the outer hang ceiling"
                        .into(),
                );
            }
            "--no-scroll" => opts.scroll = false,
            "--no-select" => opts.select = false,
            "--no-open" => opts.open = false,
            "--breakdown" => opts.breakdown = true,
            "--no-build" => opts.build = false,
            other => return Err(format!("unknown option: {other}")),
        }
        i += 1;
    }
    if opts.repo.as_os_str().is_empty() && opts.open {
        return Err("--repo <path> is required (or --no-open for the bare window)".into());
    }
    if opts.label.is_empty() {
        opts.label = opts
            .repo
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "no repository".to_string());
    }
    Ok(opts)
}
