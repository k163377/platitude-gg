//! What a branch owes main, worked out before anything runs: the reach of
//! its diff, the steps that reach selects, and which of those are already
//! green for the inputs they read.

mod imports;
mod loads;
mod make;
mod pins;
mod steps;
mod urls;

#[cfg(test)]
mod controls;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::lines::{Lines, Shown};
use super::tiers::Tiers;
use super::{environment, inputs, lines, reuse, shown, tiers};
use make::{
    CARGO, DOCKERFILE, DOCS, IMAGE, QMLTEST, QT_PIN, app, clippy_config, core, deny_policy,
    harness, qml_dirs, under,
};
pub(crate) use make::{describe, make};

/// Which side of stage 2 a step runs on: the host, or the Linux container
/// (which is "here" when the host is Linux — `cargo xtask linux`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Side {
    Host,
    Linux,
}

#[derive(Clone, Debug)]
pub(crate) struct Step {
    pub id: String,
    pub side: Side,
    /// Runs on every gate regardless of the diff, never cached: its
    /// seconds are not worth a stamp.
    pub always: bool,
    /// A verify-ui run, which builds the release unless the runner says an
    /// earlier verb of the side built it in this invocation (a cached
    /// step's build may have been another tree's).
    pub builds_app: bool,
    /// Reads the release the side's verbs build (a verb, or `bare`), so
    /// runs in the side's built-app group (`gate::sides::side`).
    pub release: bool,
    /// Shares no build directory with either group — neither cargo's
    /// `target/debug` and `target/shipped` nor the verbs' release — so runs
    /// beside both instead of in the checks' chain (`qmltest`: Qt's own
    /// runner over a staging copy of the QML).
    pub beside: bool,
    pub command: Vec<String>,
    /// What the step reads, as workspace paths (a directory covers
    /// everything under it): the cache key is their object ids.
    pub inputs: BTreeSet<String>,
    /// Which id of each input the key takes, for a step that neither
    /// builds nor reads the app ([`make::owed`] keys those on the code).
    pub keyed: Keyed,
    /// What the step is held to beyond its inputs, in the key too: a
    /// verify-ui line's promised parts and judgment (`steps::binary_steps`),
    /// so a row or a claim moved runs the line again and a green of
    /// another contract answers for nothing. Empty for the other steps.
    pub contract: String,
    /// Why no key answers for what it reads, where none can — a load
    /// nothing names (`steps::qmltest_steps`): it runs on every gate, and
    /// its green stamps nothing.
    pub unkeyed: Option<String>,
}

/// Which id of an input a step's key takes ([`make::TreeIds`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Keyed {
    /// Every byte: a tool that reads the sources sees them all.
    Bytes,
    /// A Rust file's code, every other file's bytes: a program built from
    /// Rust, which no comment of its reaches, over files it or what it
    /// runs reads as they stand (`steps::qmltest_steps`).
    RustCode,
}

/// One step the branch owes.
pub(crate) struct Required {
    pub step: Step,
    /// Empty for an always-step.
    pub key: String,
    pub cached: bool,
}

/// What one side owes beyond the reach of the diff.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub(crate) struct Scope {
    /// Every file in the tree counted as reached, and why: `--whole`, a
    /// moved version, or the build input that changed.
    pub everything: Option<String>,
    /// The whole tier's steps (`periodic`, every registered line): `--whole`,
    /// or a moved version ([`pins`]) before a merge.
    pub whole_tier: bool,
}

pub(crate) struct Plan {
    pub dir: PathBuf,
    pub head: String,
    pub main: String,
    pub base: String,
    /// The branch sits on main: its diff against main is exactly what
    /// landing would add.
    pub onto_main: bool,
    pub host_only: bool,
    /// Every step runs whether or not a stamp answers for it, even one
    /// another tree stamps while this run queues (`gate::step::run_one`
    /// looks again).
    pub fresh: bool,
    pub host: Scope,
    /// The host's, and more where the change is the container's own: the
    /// files its image is built from, or a version it alone runs.
    pub container: Scope,
    pub changed: Vec<String>,
    pub reach: BTreeSet<String>,
    pub required: Vec<Required>,
    /// Product components the change reaches that no registered line
    /// shows (`lines::Shown`) — nothing headless brings them up, so the
    /// gate cannot pass them.
    pub uncovered: Vec<String>,
    /// Changed files no step reads, listed so that a kind of file nothing
    /// tests shows in the report.
    pub unclaimed: Vec<String>,
    /// What the graph says is wrong with the tree itself
    /// (`graph::complaints`); no run over such a tree can be stamped.
    pub complaints: Vec<String>,
    /// How many of the selected lines show a component the change itself
    /// touched (`lines::Shown`), for comparison only: what a selection by
    /// what each line shows would run, against the selection by reach
    /// (反映前テストの機械化.md §動詞の絞り込みは比較表示に留める).
    pub verbs_by_shown: usize,
    /// Registered lines the reach selected that this gate leaves to the
    /// whole one (`tiers`) — before a merge only; the whole gate leaves none.
    pub verbs_left: usize,
}

pub(crate) struct Ask<'a> {
    pub main_ref: &'a str,
    pub host_only: bool,
    pub whole: bool,
    pub fresh: bool,
    /// Registered verify-ui lines to run besides the reach's, as typed.
    pub extra_verbs: &'a [String],
}

/// What a selection reads off the tree besides the reach.
struct Reading<'a> {
    dir: &'a Path,
    /// The registered lines (`lines`).
    lines: &'a Lines,
    /// The lines the tables' own move owes — a row added, a tier moved —
    /// whatever else moved (`rows_the_tables_owe`).
    added: &'a [String],
    /// What each of them shows, read off the sources.
    shown: &'a Shown,
    /// Which of the lines each gate owes, and where (`tiers`).
    tiers: &'a Tiers,
    everything: bool,
    /// Owes the whole tier ([`Scope::whole_tier`]).
    whole_tier: bool,
}

/// What the verb selection counted beside the steps it made.
struct Counted {
    /// The selected lines that show a component the change touched
    /// (`steps::lines_showing_the_change`).
    by_shown: usize,
    /// Selected lines left to the whole gate.
    left: usize,
}
