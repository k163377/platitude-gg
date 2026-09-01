//! What is driving this run, read once at startup.
//!
//! **This is the only place in the crate that looks up a `PG_*` automation
//! variable**, which is what makes the harness separable: without the
//! `automation` feature the reader below is not compiled at all and
//! [`knobs`] answers from [`Knobs::default`] — every string empty, every
//! flag off — so the app takes the arm a person at the window takes and
//! carries no other.

use std::collections::HashSet;

/// Everything the harness is allowed to say to the app it drives.
///
/// One record rather than a lookup apiece: the values are read together,
/// held for the length of the run, and handed to the properties QML reads
/// them off (`AppBackend`). Nothing here is ever written after startup.
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
    /// `PG_PLAIN_CHROME` — take the window shape the two platforms that
    /// cannot fold the band into the title bar get, neither of which can
    /// be run here.
    pub plain_chrome: bool,
    /// `PG_AUTO_IDENTITY` — `"<name>|<email>"` prefills the identity
    /// screen, and `PG_AUTO_IDENTITY_SAVE` submits it straight away.
    pub identity: String,
    pub identity_save: bool,
    /// `PG_SCROLL_TO` — `top` / `bottom` jumps the graph once the final
    /// pass settles; `nav-bottom` jumps the sidebar's branch list instead.
    pub scroll_to: String,
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
        watchdog_ms: text("PG_AUTO_WATCHDOG_MS").parse().unwrap_or(0),
        select: on("PG_AUTO_SELECT"),
        scroll: on("PG_AUTO_SCROLL"),
        perf: on("PG_AUTO_PERF"),
        wip: on("PG_AUTO_WIP"),
        plain_chrome: on("PG_PLAIN_CHROME"),
        identity: text("PG_AUTO_IDENTITY"),
        identity_save: on("PG_AUTO_IDENTITY_SAVE"),
        scroll_to: text("PG_SCROLL_TO"),
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
        assert!(!knobs.plain_chrome && !knobs.identity_save);
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
