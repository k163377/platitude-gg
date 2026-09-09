//! Performance scenario options.

use std::path::PathBuf;

use super::sampler::Limits;

#[derive(Debug, Clone)]
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
    /// Read the settled process from outside it at the end of that wait:
    /// private / mapped / image, what is resident by file, and the
    /// process heaps block by block (`perf::attribution`). What it is
    /// for: the Rust counter (`breakdown`) cannot see the C++ side, and
    /// that is where the excess over the budget sits.
    pub(super) attribute: bool,
    /// Start the invocation with the calibration run, which weighs the
    /// font database's population on its own so the budget line can be
    /// read net of it (`perf::fonts`). Off, the working set is published
    /// as sampled, walk included.
    pub(super) calibrate: bool,
    /// Ask this run's app to pay the font walk before `perf_done`, idle
    /// either side, and say when (`PGG_PERF_FONT_WALK`). The calibration
    /// run's own; no option sets it.
    pub(super) font_walk: bool,
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
    /// Measure the build that carries the verification harness, which is
    /// every measurement that needs the app to say anything about itself.
    /// False measures the shipped build — `cargo build --release` with no
    /// features — which can be weighed and timed to its first graph but
    /// cannot be driven or asked (`platitude-app` §features).
    pub(super) harness: bool,
    /// The OS device name of the screen to put the window on, empty for
    /// the primary. Everything about the frame rate is downstream of this
    /// on a machine whose monitors run at different rates.
    pub(super) screen: String,
    /// The corpus fingerprint this run must find, empty to take whatever
    /// is there. A benchmark repository that was fetched is a different
    /// benchmark (`perf::corpus`).
    pub(super) corpus: String,
    /// The commit to measure, built on the rig rather than in this tree
    /// (`perf::rig`); empty measures this tree's own build, edits and all.
    pub(super) at: String,
    /// How quiet the machine has to be. Opened by `--allow-noisy`, which
    /// publishes the numbers a busy machine produced.
    pub(super) limits: Limits,
    /// Draw with the software scene graph (`--software`), whose frames
    /// go through the backing store rather than to a display: the
    /// reading does not depend on the display being on, off or turned
    /// on and off while the runs go, nothing holds the screen awake or
    /// pokes the input timer, and the window is not raised over whatever
    /// a person has in front. What that renderer's numbers are is the
    /// report's to say (`perf::report`).
    pub(super) software: bool,
}

impl Options {
    /// The cargo features the measured binary is built with, named so the
    /// evidence can say which of the two builds it was taken on.
    pub(super) fn features(&self) -> String {
        let mut features = Vec::new();
        if self.harness {
            features.push(crate::tree::HARNESS_FEATURE);
        }
        if self.breakdown {
            features.push("memprobe");
        }
        if features.is_empty() {
            "none (the shipped set)".into()
        } else {
            features.join(",")
        }
    }

    /// The same set as a file name: `automation`, `automation+memprobe`,
    /// `shipped` — what the rig's shelf is keyed by (`perf::rig`).
    pub(super) fn feature_slug(&self) -> String {
        let features = self.features();
        if features.starts_with("none") {
            "shipped".into()
        } else {
            features.replace(',', "+")
        }
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
        attribute: false,
        calibrate: true,
        font_walk: false,
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
        harness: true,
        screen: String::new(),
        corpus: String::new(),
        at: String::new(),
        limits: Limits::default(),
        software: false,
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
            "--attribute" => opts.attribute = true,
            "--no-font-walk" => opts.calibrate = false,
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
            "--shipped" => opts.harness = false,
            "--screen" => opts.screen = value()?,
            "--corpus" => opts.corpus = value()?,
            "--at" => opts.at = value()?,
            "--allow-noisy" => opts.limits = Limits::OPEN,
            "--software" => opts.software = true,
            other => return Err(format!("unknown option: {other}")),
        }
        i += 1;
    }
    shipped(&mut opts)?;
    settle(opts)
}

/// What a build with no harness in it can and cannot be asked.
///
/// Nothing in a shipped build answers a knob, drives an action or reports
/// a frame. What is left is a window that opens a tab: its weight, and
/// how long it took to have a graph in it.
fn shipped(opts: &mut Options) -> Result<(), String> {
    if !opts.harness {
        for (asked, name) in [
            (opts.selection != "first", "--selection / --no-select"),
            (!opts.scroll, "--no-scroll"),
            (!opts.diff, "--no-diff"),
            (!opts.oid.is_empty(), "--select-oid"),
            (!opts.file.is_empty(), "--file"),
            (opts.breakdown, "--breakdown"),
            (opts.trace_frames, "--trace-frames"),
        ] {
            if asked {
                return Err(format!(
                    "--shipped cannot take {name}: a build without the harness answers no knob \
                     and reports no frame, so the only measurement it can give is memory and the \
                     time to its first graph (platitude-app §features)"
                ));
            }
        }
        if !opts.open {
            return Err(
                "--shipped needs a repository: an empty window has no graph to wait for, \
                        and the shipped build reports no frame of its own"
                    .into(),
            );
        }
        opts.selection = "none".into();
        opts.scroll = false;
        opts.diff = false;
        // The calibration run is the harness paying the walk on cue,
        // and a build with no harness takes no cue: its working set is
        // published as sampled.
        opts.calibrate = false;
    }
    Ok(())
}

/// Everything that has to hold whichever build is being measured, and the
/// one name a run that gave none takes.
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
    // A day is longer than any run; past it the sampler's window
    // arithmetic in `measure` would overflow.
    if opts.watchdog_ms.saturating_add(opts.settle_ms) > 86_400_000 {
        return Err("--watchdog-ms and --settle-ms together must stay under a day".into());
    }
    if opts.runs == 0 || opts.watchdog_ms == 0 {
        return Err("--runs and --watchdog-ms must be positive".into());
    }
    // The attribution is of a process that has stopped working, and the
    // settle wait is what makes it one; without it the walk would read a
    // process still busy letting go of the bench.
    if opts.attribute && opts.settle_ms == 0 {
        return Err(
            "--attribute reads the process once it has settled, so it needs --settle-ms".into(),
        );
    }
    if opts.attribute && !cfg!(windows) {
        return Err(
            "--attribute is implemented for Windows only: VirtualQueryEx, QueryWorkingSetEx and \
             the process heap walk have no Linux form here"
                .into(),
        );
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
        let slug = |words: &[&str]| options(words).map(|o| o.feature_slug());
        assert_eq!(slug(&["--repo", "C:/r"]), Ok("automation".to_string()));
        assert_eq!(
            slug(&["--repo", "C:/r", "--breakdown"]),
            Ok("automation+memprobe".to_string())
        );
        assert_eq!(
            slug(&["--repo", "C:/r", "--shipped"]),
            Ok("shipped".to_string())
        );
        assert_eq!(
            options(&["--repo", "C:/r"]).map(|o| o.features()),
            Ok("automation".to_string())
        );
        assert_eq!(
            options(&["--repo", "C:/r", "--breakdown"]).map(|o| o.features()),
            Ok("automation,memprobe".to_string())
        );
        assert_eq!(
            options(&["--repo", "C:/r", "--shipped"]).map(|o| o.features()),
            Ok("none (the shipped set)".to_string())
        );
    }

    #[test]
    fn a_shipped_run_refuses_the_knobs_it_could_not_answer() {
        let refused = options(&["--repo", "C:/r", "--shipped", "--no-scroll"]).unwrap_err();
        assert!(refused.contains("--no-scroll"), "{refused}");
        assert!(
            options(&["--repo", "C:/r", "--shipped", "--breakdown"])
                .unwrap_err()
                .contains("--breakdown")
        );
        assert!(
            options(&["--shipped", "--no-open"])
                .unwrap_err()
                .contains("needs a repository")
        );
    }

    /// A window the sampler's arithmetic could not hold is refused where
    /// every other bad option is, not found out after the release build.
    #[test]
    fn a_run_longer_than_a_day_is_refused() {
        assert!(
            options(&["--repo", "C:/r", "--settle-ms", "18446744073709551000"])
                .unwrap_err()
                .contains("under a day")
        );
        assert!(options(&["--repo", "C:/r", "--settle-ms", "8000"]).is_ok());
    }

    #[test]
    fn a_shipped_run_drives_nothing() {
        let opts = options(&["--repo", "C:/r", "--shipped"]).expect("a plain shipped run");
        assert!(!opts.harness && !opts.select && !opts.scroll && !opts.diff);
        assert!(!opts.calibrate && !opts.font_walk);
        assert_eq!(opts.selection, "none");
    }

    /// Every invocation weighs the font walk unless told not to, with a
    /// repository and without one alike; no run pays it on its own
    /// account — that is the calibration run's shape (`fonts`).
    #[test]
    fn the_font_walk_is_weighed_unless_declined() {
        let asked = options(&["--repo", "C:/r"]).expect("the plain options");
        assert!(asked.calibrate && !asked.font_walk);
        assert!(
            !options(&["--repo", "C:/r", "--no-font-walk"])
                .unwrap()
                .calibrate
        );
        assert!(options(&["--no-open"]).unwrap().calibrate);
    }

    /// `--at` names a commit for the rig to build; without it the
    /// measurement is of this tree, whatever it holds.
    #[test]
    fn a_commit_to_measure_is_named_by_at() {
        assert_eq!(options(&["--repo", "C:/r"]).unwrap().at, "");
        assert_eq!(
            options(&["--repo", "C:/r", "--at", "main"]).unwrap().at,
            "main"
        );
        assert!(
            options(&["--repo", "C:/r", "--at"])
                .unwrap_err()
                .contains("needs a value")
        );
    }

    /// The attribution is of a settled process, and only the settle wait
    /// makes it one; it is also a Windows walk, refused where the
    /// sampling is the only thing implemented.
    #[test]
    fn an_attribution_needs_a_settled_process() {
        let refused = options(&["--repo", "C:/r", "--attribute"]).unwrap_err();
        assert!(refused.contains("--settle-ms"), "{refused}");
        let asked = options(&["--repo", "C:/r", "--attribute", "--settle-ms", "8000"]);
        if cfg!(windows) {
            assert!(asked.expect("a Windows attribution").attribute);
        } else {
            assert!(asked.unwrap_err().contains("Windows only"));
        }
        assert!(!options(&["--repo", "C:/r"]).unwrap().attribute);
        // The shipped build can be attributed: the walk asks the process
        // nothing, so the harness is not needed for it.
        assert!(
            options(&[
                "--repo",
                "C:/r",
                "--shipped",
                "--attribute",
                "--settle-ms",
                "1"
            ])
            .map(|o| o.attribute)
            .unwrap_or(!cfg!(windows))
        );
    }

    /// The software scene graph is asked for by name; a plain run draws
    /// with whatever Qt would, which on this machine is D3D.
    #[test]
    fn the_software_scene_graph_is_asked_for_by_name() {
        assert!(!options(&["--repo", "C:/r"]).unwrap().software);
        assert!(
            options(&["--repo", "C:/r", "--software"])
                .expect("a software run")
                .software
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
