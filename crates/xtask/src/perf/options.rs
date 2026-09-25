//! Performance scenario options.

use std::path::PathBuf;

use super::sampler::Limits;

#[derive(Debug, Clone)]
pub(super) struct Options {
    pub(super) repo: PathBuf,
    pub(super) label: String,
    pub(super) runs: u32,
    /// How many times a run refused for the state of the machine is taken
    /// again (`sampler::Conditions`).
    pub(super) retries: u32,
    pub(super) watchdog_ms: u64,
    /// How long to hold the app after `perf_done` before reading the
    /// memory one last time, or 0 to read it at once: the peak alone
    /// cannot tell a process still holding the bytes from one that has
    /// handed them back.
    pub(super) settle_ms: u64,
    /// Read the settled process from outside at the end of that wait:
    /// private / mapped / image, resident by file, and the heaps block by
    /// block (`perf::attribution`). The Rust counter (`breakdown`) cannot
    /// see the C++ side.
    pub(super) attribute: bool,
    /// Start the invocation with the calibration run, which weighs the
    /// font database's population alone so the budget line can be read
    /// net of it (`perf::fonts`). Off, the working set is published as
    /// sampled, walk included.
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
    pub(super) cases: Vec<super::cases::Case>,
    pub(super) cycles: u32,
    pub(super) completion: String,
    pub(super) diff_scroll: bool,
    pub(super) cache: String,
    pub(super) cold_prepare: String,
    pub(super) compare: String,
    pub(super) diff: bool,
    pub(super) output: Option<PathBuf>,
    pub(super) breakdown: bool,
    pub(super) trace_frames: bool,
    pub(super) build: bool,
    /// False (`--no-open`) starts with no repository — the window and
    /// nothing in it: subtracted from a run on an empty repository, it
    /// leaves the cost of putting the page up.
    pub(super) open: bool,
    /// Measure the build that carries the verification harness. False
    /// measures the shipped build (`cargo build --profile shipped`, no features),
    /// which can be weighed and timed to its first graph but not driven
    /// or asked (`platitude-app` §features).
    pub(super) harness: bool,
    /// The OS device name of the screen to put the window on, empty for
    /// the primary.
    pub(super) screen: String,
    /// The corpus fingerprint this run must find, empty to take whatever
    /// is there. A benchmark repository that was fetched is a different
    /// benchmark (`perf::corpus`).
    pub(super) corpus: String,
    /// The commit to measure, built on the rig (`perf::rig`); empty
    /// measures this tree's own build, edits and all.
    pub(super) at: String,
    /// How quiet the machine has to be. Opened by `--allow-noisy`, which
    /// publishes the numbers a busy machine produced.
    pub(super) limits: Limits,
    /// Draw with the software scene graph (`--software`): the reading does
    /// not depend on the display, nothing holds the screen awake or pokes
    /// the input timer, and the window is not raised over what a person
    /// has in front. What its numbers mean is `perf::report`'s to say.
    pub(super) software: bool,
    /// `[defaults]` keys written into the run's `settings.toml`
    /// (`--setting key=value`, repeatable), for an A/B over a setting. The
    /// value is copied verbatim: a string carries its own quotes.
    pub(super) settings: Vec<(String, String)>,
    /// The app's log level (`PGG_LOG`): `debug` prints the per-command
    /// breakdown (`process::executor`), and a run taken to read it is not
    /// a budget run.
    pub(super) log: String,
}

impl Options {
    /// The cargo features the measured binary is built with, so the
    /// evidence says which build it was taken on.
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

fn defaults() -> Options {
    Options {
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
        cases: Vec::new(),
        cycles: 1,
        completion: "raw".into(),
        diff_scroll: false,
        cache: "warm".into(),
        cold_prepare: String::new(),
        compare: String::new(),
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
        settings: Vec::new(),
        log: "info".into(),
    }
}

pub(super) fn parse(args: &[String]) -> Result<Options, String> {
    let mut opts = defaults();
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
            "--cases" => {
                let file = value()?;
                let text = std::fs::read_to_string(&file).map_err(|e| format!("{file}: {e}"))?;
                opts.cases = super::cases::parse(&text)?;
            }
            "--cycles" => opts.cycles = value()?.parse().map_err(|_| "--cycles takes a number")?,
            "--completion" => opts.completion = value()?,
            "--diff-scroll" => opts.diff_scroll = true,
            "--cache" => opts.cache = value()?,
            "--cold-prepare" => opts.cold_prepare = value()?,
            "--compare" => opts.compare = value()?,
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
            "--log" => {
                let level = value()?;
                if !["error", "warn", "info", "debug", "trace"].contains(&level.as_str()) {
                    return Err("--log takes error, warn, info, debug or trace".into());
                }
                opts.log = level;
            }
            "--setting" => {
                let given = value()?;
                let (key, value) = given.split_once('=').ok_or("--setting takes key=value")?;
                if key.trim().is_empty() || value.trim().is_empty() {
                    return Err("--setting takes key=value".into());
                }
                opts.settings
                    .push((key.trim().to_string(), value.trim().to_string()));
            }
            other => return Err(format!("unknown option: {other}")),
        }
        i += 1;
    }
    shipped(&mut opts)?;
    settle(opts)
}

/// Refuses what a build with no harness cannot answer, and turns off what
/// it cannot drive.
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
        // The calibration run is the harness paying the walk on cue, and
        // a build with no harness takes no cue.
        opts.calibrate = false;
    }
    Ok(())
}

fn cache(opts: &mut Options) -> Result<(), String> {
    if !["warm", "first", "cold"].contains(&opts.cache.as_str()) {
        return Err("--cache takes warm, first, or cold".into());
    }
    if (opts.cache == "cold") != !opts.cold_prepare.is_empty() {
        return Err(
            "--cache cold requires --cold-prepare <executable>; first launch alone is not cold"
                .into(),
        );
    }
    if !opts.cold_prepare.is_empty() {
        opts.cold_prepare = std::fs::canonicalize(&opts.cold_prepare)
            .map_err(|e| format!("cold preparation executable: {e}"))?
            .to_string_lossy()
            .into_owned();
    }
    if opts.cache != "warm" {
        opts.calibrate = false;
        opts.retries = 0;
    }
    if opts.cache == "first" && opts.runs != 1 {
        return Err(
            "--cache first requires --runs 1; later launches are not first launches".into(),
        );
    }
    if !opts.compare.is_empty() && (opts.at.is_empty() || opts.cache == "first") {
        return Err(
            "--compare needs --at and warm or cold cache (A/B cannot share one first launch)"
                .into(),
        );
    }
    Ok(())
}

fn settle(mut opts: Options) -> Result<Options, String> {
    cache(&mut opts)?;
    if !["raw", "coloured"].contains(&opts.completion.as_str()) || opts.cycles == 0 {
        return Err("--completion takes raw or coloured; --cycles must be positive".into());
    }
    if (!opts.cases.is_empty() || opts.diff_scroll || opts.completion != "raw")
        && (!opts.harness || !opts.open || !opts.diff || opts.selection == "none")
    {
        return Err("cases, colour completion and diff scroll require the harness, repository, selection and diff".into());
    }
    if !opts.cases.is_empty() && (!opts.oid.is_empty() || !opts.file.is_empty()) {
        return Err(
            "--cases owns the OIDs and paths; do not combine with --select-oid or --file".into(),
        );
    }
    if opts.cycles > 1
        && opts
            .cases
            .iter()
            .map(|c| &c.oid)
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            < 2
    {
        return Err("--cycles requires cases with at least two distinct OIDs".into());
    }
    if opts
        .cycles
        .checked_mul(opts.cases.len().max(1) as u32)
        .is_none_or(|n| n > i32::MAX as u32)
    {
        return Err("too many case operations".into());
    }
    if opts.cases.windows(2).any(|pair| pair[0].oid == pair[1].oid)
        || (opts.cycles > 1
            && opts
                .cases
                .first()
                .zip(opts.cases.last())
                .is_some_and(|(a, b)| a.oid == b.oid))
    {
        return Err("successive cases must select different OIDs, including the cycle boundary (a reclick is a different gesture)".into());
    }
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
    // Without the settle wait the walk reads a process still busy letting
    // go of the bench.
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

    #[test]
    fn cache_modes_do_not_relabel_a_warm_repeat_as_cold() {
        assert!(
            options(&["--repo", "x", "--cache", "cold"])
                .unwrap_err()
                .contains("cold-prepare")
        );
        assert!(
            options(&["--repo", "x", "--cache", "first"])
                .unwrap_err()
                .contains("--runs 1")
        );
        let first = options(&["--repo", "x", "--cache", "first", "--runs", "1"]).unwrap();
        assert!(!first.calibrate);
        assert_eq!(first.retries, 0);
        assert!(options(&["--repo", "x", "--compare", "main"]).is_err());
        assert!(options(&["--repo", "x", "--cycles", "100"]).is_err());
        assert!(options(&["--repo", "x", "--no-diff", "--completion", "coloured"]).is_err());
    }

    fn options(words: &[&str]) -> Result<super::Options, String> {
        parse(&words.iter().map(|w| (*w).to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn every_selection_changes_commit_including_the_cycle_boundary() {
        let mut opts = options(&["--repo", "x"]).unwrap();
        opts.cases = super::super::cases::parse(&format!(
            "one\t{}\ta.kt\traw\ntwo\t{}\tb.kt\traw\nthree\t{}\tc.kt\traw",
            "a".repeat(40),
            "b".repeat(40),
            "a".repeat(40)
        ))
        .unwrap();
        assert!(super::settle(opts.clone()).is_ok());
        opts.cycles = 2;
        assert!(super::settle(opts.clone()).is_err());
        opts.cases.pop();
        assert!(super::settle(opts.clone()).is_ok());
        opts.cases[1].oid = opts.cases[0].oid.clone();
        opts.cycles = 1;
        assert!(super::settle(opts).is_err());
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

    /// A window the sampler's arithmetic could not hold is refused
    /// where every other bad option is.
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

    /// With a repository and without one alike; no run pays the walk on
    /// its own account — that is the calibration run's shape (`fonts`).
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
