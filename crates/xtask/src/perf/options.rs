//! Performance scenario options.

use std::path::PathBuf;

use super::sampler::Limits;

#[derive(Debug)]
pub(super) struct Options {
    pub(super) repo: PathBuf,
    pub(super) label: String,
    pub(super) runs: u32,
    /// How many times a run refused for the state of the machine is taken
    /// again. A person walking away, a parallel build and a locked
    /// session are all transient; the answer to one is another run, not a
    /// published number (`sampler::Conditions`).
    pub(super) retries: u32,
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
    pub(super) selection: String,
    pub(super) oid: String,
    pub(super) file: String,
    pub(super) diff: bool,
    pub(super) output: Option<PathBuf>,
    pub(super) breakdown: bool,
    pub(super) trace_frames: bool,
    pub(super) build: bool,
    /// Start with no repository at all — the window and nothing in it.
    /// What it is for: subtracting this from a run that opened an empty
    /// repository leaves the cost of putting the page up, which is
    /// otherwise indistinguishable from the toolkit's own floor.
    pub(super) open: bool,
    /// The OS device name of the screen to put the window on, empty for
    /// the primary. Everything about the frame rate is downstream of this
    /// on a machine whose monitors run at different rates.
    pub(super) screen: String,
    /// The corpus fingerprint this run must find, empty to take whatever
    /// is there. A benchmark repository that was fetched is a different
    /// benchmark (`perf::corpus`).
    pub(super) corpus: String,
    /// How quiet the machine has to be. Opened by `--allow-noisy`, which
    /// publishes the numbers a busy machine produced.
    pub(super) limits: Limits,
}

impl Options {
    /// The cargo features the measured binary is built with, named so the
    /// evidence says which build a number was taken on. `perf` and
    /// `shipped` land on the same path, so a `--no-build` run measures
    /// whichever of them ran last.
    pub(super) fn features(&self) -> String {
        let mut features = vec![crate::tree::HARNESS_FEATURE];
        if self.breakdown {
            features.push("memprobe");
        }
        features.join(",")
    }
}

pub(super) fn parse(args: &[String]) -> Result<Options, String> {
    let mut opts = Options {
        repo: PathBuf::new(),
        label: String::new(),
        runs: 3,
        retries: 3,
        watchdog_ms: 300_000,
        settle_ms: 0,
        scroll: true,
        select: true,
        selection: "first".into(),
        oid: String::new(),
        file: String::new(),
        diff: true,
        output: None,
        breakdown: false,
        trace_frames: false,
        build: true,
        open: true,
        screen: String::new(),
        corpus: String::new(),
        limits: Limits::default(),
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
            "--retries" => {
                opts.retries = value()?
                    .parse()
                    .map_err(|_| "--retries takes a number".to_string())?;
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
            "--no-select" => opts.selection = "none".into(),
            "--selection" => opts.selection = value()?,
            "--select-oid" => opts.oid = value()?,
            "--file" => opts.file = value()?,
            "--no-diff" => opts.diff = false,
            "--output" => opts.output = Some(PathBuf::from(value()?)),
            "--no-open" => opts.open = false,
            "--breakdown" => opts.breakdown = true,
            "--trace-frames" => opts.trace_frames = true,
            "--no-build" => opts.build = false,
            "--screen" => opts.screen = value()?,
            "--corpus" => opts.corpus = value()?,
            "--allow-noisy" => opts.limits = Limits::OPEN,
            other => return Err(format!("unknown option: {other}")),
        }
        i += 1;
    }
    settle(opts)
}

/// Everything that has to hold whatever the run asked for, and the one
/// name a run that gave none takes.
fn settle(mut opts: Options) -> Result<Options, String> {
    if !["none", "first", "head"].contains(&opts.selection.as_str()) {
        return Err("--selection takes none, first, or head".into());
    }
    opts.select = opts.selection != "none";
    if (!opts.oid.is_empty() || !opts.file.is_empty()) && !opts.select {
        return Err("--select-oid and --file require selection".into());
    }
    if !opts.oid.is_empty()
        && (!matches!(opts.oid.len(), 40 | 64) || !opts.oid.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        return Err("--select-oid requires a full hexadecimal object id".into());
    }
    if !opts.file.is_empty() && !opts.diff {
        return Err("--file conflicts with --no-diff".into());
    }
    if opts.runs == 0 || opts.watchdog_ms == 0 {
        return Err("--runs and --watchdog-ms must be positive".into());
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

#[cfg(test)]
mod tests {
    use super::parse;

    fn options(words: &[&str]) -> Result<super::Options, String> {
        parse(&words.iter().map(|w| (*w).to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn the_measured_build_names_itself() {
        assert_eq!(
            options(&["--repo", "C:/r"]).map(|o| o.features()),
            Ok("automation".to_string())
        );
        assert_eq!(
            options(&["--repo", "C:/r", "--breakdown"]).map(|o| o.features()),
            Ok("automation,memprobe".to_string())
        );
    }

    #[test]
    fn the_host_gate_opens_only_when_asked() {
        let strict = options(&["--repo", "C:/r"]).unwrap();
        assert!(strict.limits.foreign_percent.is_finite());
        let open = options(&["--repo", "C:/r", "--allow-noisy"]).unwrap();
        assert!(open.limits.foreign_percent.is_infinite());
    }
}
