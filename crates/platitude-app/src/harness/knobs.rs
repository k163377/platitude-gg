//! What is driving this run, read once at startup.
//!
//! The only place in the crate that looks up a `PGG_*` automation variable
//! (.claude/rules/app-ui.md). Without the `automation` feature the reader
//! is not compiled and [`knobs`] answers [`Knobs::default`] — every string
//! empty, every flag off.
//!
//! Most fields are then read by nothing (their reader, `Harness`, is not
//! compiled either). `allow`, not `expect`: the tests read every field,
//! which would leave the expectation unfulfilled.
#![cfg_attr(not(feature = "automation"), allow(dead_code))]

use std::collections::HashSet;

/// Everything the harness is allowed to say to the app it drives, read
/// together at startup and never written after.
///
/// The `perf_*` fields exist only with the feature: their one reader is
/// `harness::perf_probe`. Everything else is in both builds, empty without
/// it, because the app asks the same questions however it was compiled.
#[derive(Default)]
pub(crate) struct Knobs {
    /// `PGG_AUTO_ACT` — the one operation to run once the repository is
    /// loaded, and `PGG_AUTO_ACT_ARG` its argument. A bare verb, so QML
    /// dispatches on equality.
    pub act: String,
    pub act_arg: String,
    /// `PGG_AUTO_OPEN` — `;`-separated repositories, and the whole of
    /// what this run opens as tabs.
    pub open: String,
    /// `PGG_SHOT_DIR` — where a headless run leaves its pictures.
    /// Backslashes forward: QML takes it as a URL.
    pub shot_dir: String,
    /// `PGG_OTHER_GIT` — a second git staged beside the pictures and
    /// deliberately **not** on PATH (`--other-git`): the settings screen
    /// grows a button only for a git that answers and is not the one
    /// running.
    pub other_git: String,
    /// `PGG_AUTO_WATCHDOG_MS` — the deadline that keeps a broken causal run
    /// bounded.
    pub watchdog_ms: i32,
    /// `PGG_FAULT_HANG` — the station to hold this run at for good, by its
    /// trail word (`harness::deadline`), so the parent's reading of a wedge
    /// can be checked (`xtask::verify::faults`).
    pub fault_hang: String,
    /// `PGG_FAULT_NO_DEADLINE` — leave the deadline thread down, so the
    /// run leaves no report however it ends: the shape a wedge past
    /// `exiting` has anyway.
    pub fault_no_deadline: bool,
    /// `PGG_FAULT_HOLD_ACT` — swallow the verb's completion, so the loop
    /// keeps turning and nothing says done (`xtask::verify::faults`).
    /// Ordered, not raced: a ceiling short enough to beat a completion on
    /// one machine loses to it on another.
    pub fault_hold_act: bool,
    /// `PGG_FAULT_HOLD_WIP_ROW` — walk every pass as one that began
    /// before this window's first status did, until the run says
    /// otherwise: an arrangement no repository can be built into
    /// (`platitude_core::session::PassHooks::holds_back_the_worktree_row`).
    pub fault_hold_wip_row: bool,
    /// `PGG_FAULT_HOLD_SAVE` — hold every configuration save the hub
    /// spawns until the station this names (`harness::deadline`), so a
    /// close lands on a save that is out and the shutdown is read joining
    /// one (`PGG_AUTO_ACT=quit-save-held`, `xtask::verify::child`). Empty:
    /// no save is held.
    pub fault_hold_save: String,
    /// `PGG_AUTO_SELECT` — select the newest commit and open the first
    /// changed file, so a picture has something in every pane.
    pub select: bool,
    /// `PGG_AUTO_SCROLL` — run the scroll benchmark.
    pub scroll: bool,
    /// `PGG_AUTO_PERF` — take the startup and interaction measurements.
    pub perf: bool,
    /// `PGG_AUTO_WIP` — open the worktree view once there is something
    /// uncommitted to show.
    pub wip: bool,
    /// `PGG_SYSTEM_TITLE_BAR` — take the window shape the two platforms
    /// that cannot fold the band into the title bar get, neither of
    /// which can be run here.
    pub system_title_bar: bool,
    /// `PGG_AUTO_IDENTITY` — `"<name>|<email>"` prefills the identity
    /// screen, and `PGG_AUTO_IDENTITY_SAVE` submits it straight away.
    pub identity: String,
    pub identity_save: bool,
    /// `PGG_MEM_REPORT` — file and print the memory breakdown. The walks
    /// it turns on are O(rows) per drain, so a run that did not ask for
    /// it pays nothing (`memprobe::enabled`).
    pub mem_report: bool,
    /// `PGG_PERF_SELECTION` — what the interaction measurement selects
    /// (`none` / `first` / `head`). Empty leaves it to [`Knobs::select`].
    #[cfg(feature = "automation")]
    pub perf_selection: String,
    /// `PGG_PERF_OID` / `PGG_PERF_FILE` — the commit it selects and the
    /// changed file it opens, when the run names them.
    #[cfg(feature = "automation")]
    pub perf_oid: String,
    #[cfg(feature = "automation")]
    pub perf_file: String,
    #[cfg(feature = "automation")]
    pub perf_cases: String,
    #[cfg(feature = "automation")]
    pub perf_cycles: u32,
    #[cfg(feature = "automation")]
    pub perf_completion: String,
    #[cfg(feature = "automation")]
    pub perf_diff_scroll: bool,
    /// `PGG_PERF_DIFF=0` — leave the diff out of the measurement. Negated
    /// so the idle, all-off record keeps the diff in.
    #[cfg(feature = "automation")]
    pub perf_no_diff: bool,
    /// `PGG_PERF_TRACE_FRAMES` — log every frame interval of the scroll
    /// benchmark. A diagnostic run only: the flush can move the last
    /// frame it is measuring.
    #[cfg(feature = "automation")]
    pub perf_trace_frames: bool,
    /// `PGG_PERF_FONT_WALK` — before `perf_done`, shape one glyph the UI
    /// family lacks and say when, either side of an idle: the first such
    /// glyph makes Qt populate its whole font database, and
    /// `cargo xtask perf` weighs that where its sampler can see
    /// (`WindowPerfDriver`, xtask `perf::fonts`).
    #[cfg(feature = "automation")]
    pub perf_font_walk: bool,
    /// Whether anything at all is driving this run
    /// (`settings::Env::automated`); a driven run gets no settings files
    /// (`settings::Build::driven`).
    pub automated: bool,
    /// `PGG_FAKE_PR` — branch names wearing the PR badge, so the design can
    /// be reviewed before Phase 4 joins the real thing in. The only knob
    /// that reaches what a row says about a repository; here so a build
    /// without the harness cannot be handed one.
    pub fake_pr: HashSet<String>,
}

/// What is driving this run. Read on the first call and held after that:
/// the encode path asks per row.
pub(crate) fn knobs() -> &'static Knobs {
    static KNOBS: std::sync::OnceLock<Knobs> = std::sync::OnceLock::new();
    KNOBS.get_or_init(read)
}

#[cfg(feature = "automation")]
fn read() -> Knobs {
    fn text(name: &str) -> String {
        std::env::var(name).unwrap_or_default()
    }
    fn on(name: &str) -> bool {
        std::env::var(name).as_deref() == Ok("1")
    }
    Knobs {
        act: text("PGG_AUTO_ACT"),
        act_arg: text("PGG_AUTO_ACT_ARG"),
        open: text("PGG_AUTO_OPEN"),
        shot_dir: text("PGG_SHOT_DIR").replace('\\', "/"),
        other_git: text("PGG_OTHER_GIT"),
        watchdog_ms: text("PGG_AUTO_WATCHDOG_MS").parse().unwrap_or(0),
        fault_hang: text("PGG_FAULT_HANG"),
        fault_no_deadline: on("PGG_FAULT_NO_DEADLINE"),
        fault_hold_act: on("PGG_FAULT_HOLD_ACT"),
        fault_hold_wip_row: on("PGG_FAULT_HOLD_WIP_ROW"),
        fault_hold_save: text("PGG_FAULT_HOLD_SAVE"),
        select: on("PGG_AUTO_SELECT"),
        scroll: on("PGG_AUTO_SCROLL"),
        perf: on("PGG_AUTO_PERF"),
        wip: on("PGG_AUTO_WIP"),
        system_title_bar: on("PGG_SYSTEM_TITLE_BAR"),
        identity: text("PGG_AUTO_IDENTITY"),
        identity_save: on("PGG_AUTO_IDENTITY_SAVE"),
        mem_report: on("PGG_MEM_REPORT"),
        perf_selection: text("PGG_PERF_SELECTION"),
        perf_oid: text("PGG_PERF_OID"),
        perf_file: text("PGG_PERF_FILE"),
        perf_cases: text("PGG_PERF_CASES"),
        perf_cycles: text("PGG_PERF_CYCLES").parse().unwrap_or(1),
        perf_completion: text("PGG_PERF_COMPLETION"),
        perf_diff_scroll: on("PGG_PERF_DIFF_SCROLL"),
        perf_no_diff: text("PGG_PERF_DIFF") == "0",
        perf_trace_frames: on("PGG_PERF_TRACE_FRAMES"),
        perf_font_walk: on("PGG_PERF_FONT_WALK"),
        automated: platitude_core::settings::Env::system().automated(),
        fake_pr: text("PGG_FAKE_PR")
            .split(',')
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .collect(),
    }
}

#[cfg(not(feature = "automation"))]
fn read() -> Knobs {
    Knobs::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The idle record: a build without the harness, and a run nobody
    /// drives in a build with one.
    fn assert_at_rest(knobs: &Knobs) {
        assert!(knobs.act.is_empty() && knobs.act_arg.is_empty());
        assert!(knobs.open.is_empty() && knobs.shot_dir.is_empty());
        assert!(knobs.identity.is_empty());
        assert!(!knobs.select && !knobs.scroll && !knobs.perf && !knobs.wip);
        assert!(!knobs.system_title_bar && !knobs.identity_save);
        assert!(!knobs.mem_report);
        #[cfg(feature = "automation")]
        {
            assert!(knobs.perf_selection.is_empty() && knobs.perf_oid.is_empty());
            assert!(knobs.perf_file.is_empty() && !knobs.perf_trace_frames);
            assert!(
                !knobs.perf_no_diff,
                "the diff is in until a run asks for it out"
            );
            assert!(
                !knobs.perf_font_walk,
                "the walk is paid where the rows ask for it until a run asks for it up front"
            );
        }
        assert!(
            !knobs.automated,
            "a build that reads no environment cannot find anything driving it"
        );
        assert_eq!(knobs.watchdog_ms, 0);
        assert!(
            knobs.fault_hang.is_empty()
                && !knobs.fault_no_deadline
                && !knobs.fault_hold_act
                && knobs.fault_hold_save.is_empty(),
            "a run nobody is driving is held by nothing, keeps its own deadline, and finishes what it starts"
        );
        assert!(
            knobs.fake_pr.is_empty(),
            "a badge nothing joined in is one nobody can be handed"
        );
    }

    #[test]
    fn an_idle_harness_says_nothing() {
        assert_at_rest(&Knobs::default());
    }

    /// Read off [`read`] itself, not by setting a variable: the
    /// environment is process-global state the parallel tests share
    /// (.claude/rules/code.md).
    #[cfg(not(feature = "automation"))]
    #[test]
    fn a_build_without_the_harness_reads_no_environment() {
        assert_at_rest(&read());
    }
}
