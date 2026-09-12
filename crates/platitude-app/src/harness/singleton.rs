//! What the harness is, as QML sees it: one object carrying everything a
//! run was told to do, and the three ways back out to `tracing` and the hub.
//!
//! **A QObject of its own rather than fields on `AppBackend`**, because
//! `#[cfg]` does not reach inside `#[qslot]` — the macro emits the
//! registration and the dispatch arm without looking (app-ui.md §Qt
//! Bridges). A whole type behind the feature is the one shape that
//! disappears: no properties, no slots, no names in the shipped binary's
//! meta-object. What the product keeps is the single bit that says whether
//! this build has one at all (`AppBackend::harness_present`).
//!
//! Nothing in `platitude.ui` may name it. The QML that reads it is the
//! module the same feature leaves out (`src/auto`), and the product asks
//! its own questions of its own properties instead — counted by machine
//! (`cargo xtask structure`).

use qtbridge::qobject;

use crate::hub::Hub;
use crate::models::qml_register;

/// Everything a run was told to do, read once at startup (`super::knobs`).
///
/// Every property is `Constant`: these are what the run was started with,
/// and nothing writes one afterwards.
pub struct Harness {
    /// The one operation this run is to make, and its argument. A bare verb
    /// rather than a script, so QML dispatches on equality; the argument
    /// passes through as the verb needs it.
    auto_act: String,
    auto_act_arg: String,
    /// `;`-separated repositories to open as tabs instead of restoring the
    /// ones that were left.
    auto_open: String,
    /// Where a headless run leaves its pictures; empty is a run that takes
    /// none.
    shot_dir: String,
    /// A second git this run may be pointed at, staged beside the pictures
    /// and not on PATH; empty where none was asked for (`--other-git`).
    other_git: String,
    /// The deadline that keeps a broken causal run bounded. It never
    /// chooses when a screenshot is taken.
    auto_watchdog_ms: i32,
    /// The three ways a run picks the row a page stands on, so a picture
    /// has something in every pane and a measurement has a selection to
    /// time.
    auto_select: bool,
    auto_perf: bool,
    auto_wip: bool,
    /// Run the scroll benchmark.
    auto_scroll: bool,
    /// `"<name>|<email>"` prefills the identity screen, and the second
    /// submits it straight away.
    auto_identity: String,
    auto_identity_save: bool,
    /// The window drives the memory breakdown off a timer instead of
    /// leaving it to whoever remembers to ask.
    mem_report: bool,
    /// Shape one glyph the UI family lacks before `perf_done`, either side
    /// of an idle, and say when: the font database's population, paid
    /// where the memory sampler can weigh it (`WindowPerfDriver`).
    perf_font_walk: bool,
    /// Whether anything at all is driving this run — any `PGG_*` knob but
    /// the three that say nothing about who is at the window
    /// (`settings::Env::automated`).
    automated: bool,
    /// Swallow the verb's completion where a run asked for it: the act
    /// runs, the loop turns, and nothing ever reports done
    /// (`xtask::verify::faults` orders this shape rather than racing a
    /// ceiling against it).
    fault_hold_act: bool,
}

impl Default for Harness {
    fn default() -> Self {
        let knobs = super::knobs();
        Self {
            auto_act: knobs.act.clone(),
            auto_act_arg: knobs.act_arg.clone(),
            auto_open: knobs.open.clone(),
            shot_dir: knobs.shot_dir.clone(),
            other_git: knobs.other_git.clone(),
            auto_watchdog_ms: knobs.watchdog_ms,
            auto_select: knobs.select,
            auto_scroll: knobs.scroll,
            auto_perf: knobs.perf,
            auto_wip: knobs.wip,
            auto_identity: knobs.identity.clone(),
            auto_identity_save: knobs.identity_save,
            mem_report: super::memprobe::enabled(),
            perf_font_walk: knobs.perf_font_walk,
            automated: knobs.automated,
            fault_hold_act: knobs.fault_hold_act,
        }
    }
}

#[qobject(ConvertToCamelCase, NoQmlElement)]
impl Harness {
    // What each of these means is on the field it names: `qproperty!` takes
    // no attributes, so a doc comment here is a compile error.
    qproperty!("autoAct", Member = auto_act, Constant);
    qproperty!("autoActArg", Member = auto_act_arg, Constant);
    qproperty!("autoOpen", Member = auto_open, Constant);
    qproperty!("shotDir", Member = shot_dir, Constant);
    qproperty!("otherGit", Member = other_git, Constant);
    qproperty!("autoWatchdogMs", Member = auto_watchdog_ms, Constant);
    qproperty!("autoSelect", Member = auto_select, Constant);
    qproperty!("autoPerf", Member = auto_perf, Constant);
    qproperty!("autoWip", Member = auto_wip, Constant);
    qproperty!("autoScroll", Member = auto_scroll, Constant);
    qproperty!("autoIdentity", Member = auto_identity, Constant);
    qproperty!("autoIdentitySave", Member = auto_identity_save, Constant);
    qproperty!("memReport", Member = mem_report, Constant);
    qproperty!("perfFontWalk", Member = perf_font_walk, Constant);
    qproperty!("automated", Member = automated, Constant);
    qproperty!("faultHoldAct", Member = fault_hold_act, Constant);

    /// The reporting channel the harness reads its answers off (QML →
    /// tracing → `xtask`).
    #[qslot]
    fn report(&self, message: String) {
        super::report(&message);
    }

    /// Writes one line of the memory breakdown, tagged with where the run
    /// had got to (`PGG_MEM_REPORT=1` only). Asked from QML because the
    /// sessions live on the Qt main thread — what it does is the probe's
    /// (`harness::memprobe::note_now`).
    #[qslot]
    fn note_memory(&self, label: String) {
        super::memprobe::note_now(&label);
    }

    /// How many open tabs are holding a repository.
    ///
    /// One, however many tabs the strip has: a tab that is not in front has
    /// let go of everything it read (`Hub::release_tab`), and one that has
    /// never been in front never read anything. **This is the whole of what
    /// a released tab looks like from outside**, and the one thing a picture
    /// cannot say.
    #[qslot]
    fn open_session_count(&self) -> i32 {
        Hub::with(|hub| i32::try_from(hub.sessions().len()).unwrap_or(i32::MAX)).unwrap_or(0)
    }
}
qml_register!(Harness, "Harness", singleton = true);
