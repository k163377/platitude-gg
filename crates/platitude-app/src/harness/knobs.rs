//! What is driving this run, read once at startup.
//!
//! **This is the only place in the crate that looks up a `PG_*` automation
//! variable**, which is what makes the harness separable: without the
//! `automation` feature the reader below is not compiled at all and
//! [`knobs`] answers from [`Knobs::default`] — every string empty, every
//! flag off — so the app takes the arm a person at the window takes and
//! carries no other.
//!
//! Which is also why most of the record below is read by nothing there:
//! what reads it is the QML-facing `Harness`, and that type is not
//! compiled either (`harness::singleton`). `allow` rather than `expect`,
//! because the tests at the foot read every field whenever tests are
//! compiled and an expectation that goes unfulfilled is a warning of its
//! own.
#![cfg_attr(not(feature = "automation"), allow(dead_code))]

use std::collections::HashSet;

/// Everything the harness is allowed to say to the app it drives.
///
/// One record rather than a lookup apiece: the values are read together,
/// held for the length of the run, and handed to the properties QML reads
/// them off (`AppBackend`). Nothing here is ever written after startup.
///
/// The `perf_*` group is the one part that is not in every build: the only
/// thing that reads it is the probe the feature brings in
/// (`harness::perf_probe`), so a build without one carries no field for it
/// either. Everything else is here in both, empty, because the app asks
/// the same questions however it was compiled.
#[derive(Default)]
pub(crate) struct Knobs {
    /// `PG_AUTO_ACT` — the one operation to run once the repository is
    /// loaded, and `PG_AUTO_ACT_ARG` its argument. A bare verb rather than
    /// a script, so QML dispatches on equality.
    pub act: String,
    pub act_arg: String,
    /// `PG_AUTO_OPEN` — `;`-separated repositories to open as tabs
    /// instead of restoring the ones that were left.
    pub open: String,
    /// `PG_SHOT_DIR` — where a headless run leaves its pictures.
    /// Backslashes forward: QML takes it as a URL.
    pub shot_dir: String,
    /// `PG_OTHER_GIT` — a second git this run may be pointed at, staged
    /// beside the pictures and deliberately **not** on PATH. What the
    /// settings screen's chapter needs to be photographed at all: the one
    /// state it grows a button for is a git that answers and is not the
    /// one running, and a run cannot name a second installation that
    /// exists on both a desk and a container (`--other-git`).
    pub other_git: String,
    /// `PG_AUTO_WATCHDOG_MS` — the deadline that keeps a broken causal run
    /// bounded. It never chooses when a screenshot is taken.
    pub watchdog_ms: i32,
    /// `PG_AUTO_SELECT` — select the newest commit and open the first
    /// changed file, so a picture has something in every pane.
    pub select: bool,
    /// `PG_AUTO_SCROLL` — run the scroll benchmark.
    pub scroll: bool,
    /// `PG_AUTO_PERF` — take the startup and interaction measurements.
    pub perf: bool,
    /// `PG_AUTO_WIP` — open the working-tree view once there is something
    /// uncommitted to show.
    pub wip: bool,
    /// `PG_SYSTEM_TITLE_BAR` — take the window shape the two platforms
    /// that cannot fold the band into the title bar get, neither of
    /// which can be run here.
    pub system_title_bar: bool,
    /// `PG_AUTO_IDENTITY` — `"<name>|<email>"` prefills the identity
    /// screen, and `PG_AUTO_IDENTITY_SAVE` submits it straight away.
    pub identity: String,
    pub identity_save: bool,
    /// `PG_SCROLL_TO` — `top` / `bottom` jumps the graph once the page
    /// has stopped arriving (`auto/PageSettled`); `nav-bottom` jumps the
    /// sidebar's branch list instead.
    pub scroll_to: String,
    /// `PG_MEM_REPORT` — file and print the memory breakdown. The walks
    /// it turns on are O(rows) per drain, so a run that did not ask for
    /// it pays nothing (`memprobe::enabled`).
    pub mem_report: bool,
    /// `PG_PERF_SELECTION` — what the interaction measurement selects
    /// (`none` / `first` / `head`). Empty leaves it to [`Knobs::select`],
    /// which is what a run that only asked for a selection wants.
    #[cfg(feature = "automation")]
    pub perf_selection: String,
    /// `PG_PERF_OID` / `PG_PERF_FILE` — the commit it selects and the
    /// changed file it opens, when the run names them.
    #[cfg(feature = "automation")]
    pub perf_oid: String,
    #[cfg(feature = "automation")]
    pub perf_file: String,
    /// `PG_PERF_DIFF=0` — leave the diff out of the measurement.
    ///
    /// Spelled as the refusal rather than the permission because an idle
    /// harness is every flag off, and the diff is *in* unless a run says
    /// otherwise.
    #[cfg(feature = "automation")]
    pub perf_no_diff: bool,
    /// `PG_PERF_TRACE_FRAMES` — log every frame interval of the scroll
    /// benchmark. A diagnostic run only: the flush can move the last
    /// frame it is measuring.
    #[cfg(feature = "automation")]
    pub perf_trace_frames: bool,
    /// `PG_PERF_FONT_WALK` — before `perf_done`, shape one glyph the UI
    /// family lacks and say when, either side of an idle. What it is for:
    /// the first such glyph makes Qt populate its whole font database,
    /// and `cargo xtask perf` weighs that at a moment its sampler can see
    /// rather than in the middle of the scroll (`WindowPerfDriver`,
    /// xtask `perf::fonts`).
    #[cfg(feature = "automation")]
    pub perf_font_walk: bool,
    /// Whether anything at all is driving this run
    /// (`settings::Env::automated` — any `PG_*` knob but the three that
    /// say nothing about who is at the window). The settings store is
    /// what reads it, and gives a driven run no files at all
    /// (`settings::Build::driven`).
    pub automated: bool,
    /// `PG_FAKE_PR` — branch names wearing the PR badge, so the design can
    /// be reviewed before Phase 4 joins the real thing in. **The only
    /// harness knob that reaches what a row says about a repository**, and
    /// the reason it is here: a build without the harness has an empty set
    /// and no way to be handed a full one.
    pub fake_pr: HashSet<String>,
}

/// What is driving this run. Read on the first call and held after that —
/// the encode path asks per row, and an environment lookup there would be
/// its own small cost on a path the harness is supposed to leave alone.
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
        act: text("PG_AUTO_ACT"),
        act_arg: text("PG_AUTO_ACT_ARG"),
        open: text("PG_AUTO_OPEN"),
        shot_dir: text("PG_SHOT_DIR").replace('\\', "/"),
        other_git: text("PG_OTHER_GIT"),
        watchdog_ms: text("PG_AUTO_WATCHDOG_MS").parse().unwrap_or(0),
        select: on("PG_AUTO_SELECT"),
        scroll: on("PG_AUTO_SCROLL"),
        perf: on("PG_AUTO_PERF"),
        wip: on("PG_AUTO_WIP"),
        system_title_bar: on("PG_SYSTEM_TITLE_BAR"),
        identity: text("PG_AUTO_IDENTITY"),
        identity_save: on("PG_AUTO_IDENTITY_SAVE"),
        scroll_to: text("PG_SCROLL_TO"),
        mem_report: on("PG_MEM_REPORT"),
        perf_selection: text("PG_PERF_SELECTION"),
        perf_oid: text("PG_PERF_OID"),
        perf_file: text("PG_PERF_FILE"),
        perf_no_diff: text("PG_PERF_DIFF") == "0",
        perf_trace_frames: on("PG_PERF_TRACE_FRAMES"),
        perf_font_walk: on("PG_PERF_FONT_WALK"),
        automated: platitude_core::settings::Env::system().automated(),
        fake_pr: text("PG_FAKE_PR")
            .split(',')
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .collect(),
    }
}

/// Nobody is driving, and no environment was asked.
#[cfg(not(feature = "automation"))]
fn read() -> Knobs {
    Knobs::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every field of an idle harness spelled out once: the shape a build
    /// without one runs in, and the shape a run nobody is driving comes
    /// out as in a build with one.
    fn assert_at_rest(knobs: &Knobs) {
        assert!(knobs.act.is_empty() && knobs.act_arg.is_empty());
        assert!(knobs.open.is_empty() && knobs.shot_dir.is_empty());
        assert!(knobs.identity.is_empty() && knobs.scroll_to.is_empty());
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
            knobs.fake_pr.is_empty(),
            "a badge nothing joined in is one nobody can be handed"
        );
    }

    #[test]
    fn an_idle_harness_says_nothing() {
        assert_at_rest(&Knobs::default());
    }

    /// The whole of what the feature buys. Read rather than asserted about
    /// the environment: this build has no reader to hand one to, which is
    /// the property, and a test that set a variable would be reaching for
    /// process-global state its neighbours share (CLAUDE.md ビルド・テスト).
    #[cfg(not(feature = "automation"))]
    #[test]
    fn a_build_without_the_harness_reads_no_environment() {
        assert_at_rest(&read());
    }
}
