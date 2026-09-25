//! `cargo xtask waits` — no test in this workspace takes a stretch of
//! clock for an answer, and no verb of this runner takes a wait from
//! anywhere but `crate::wait`.
//!
//! What it reads is every statement of test code — the integration
//! suites with their support modules, the `#[cfg(test)]` blocks (a
//! `mod`, a `fn` or an `impl`) and `*_tests.rs` files of every crate,
//! the QtTest files, and the app's own automation harness — and what it
//! names is four shapes (.claude/rules/core.md §非同期・並行テスト):
//!
//! - `sleep`: a sleep or a `yield_now` spent in place of an answer.
//! - `deadline`: a clock read, or a `timeout` given a budget of its own.
//! - `ignored`: a wait whose answer is thrown away.
//! - `naked`: a tracked completion (`outcome()`), a session boundary
//!   (`wait_for_graph_passes`, `wait_for_snapshot_reads`) or a
//!   hand-stepped tick awaited with no backstop under it.
//!
//! Of this runner's own crate it reads the tool bodies too, and names a
//! `sleep` (in the code or in a PowerShell script the code writes) or a
//! `deadline` (a monotonic clock read, or a receive under any budget)
//! that does not go through `crate::wait` (.claude/rules-refs/core.md「xtask
//! の待ちは `crate::wait` 1 本から取る」). The product crates' bodies are
//! not read.
//!
//! The exception is the app's harness, test code compiled inside the
//! window, in two halves: the QML driver ([`audit::HARNESS`]) is named by rules
//! of its own ([`qml::Kind::Harness`]), and its Rust half
//! ([`audit::HARNESS_RUST`]) by this runner's body rules. The window beside them
//! (`src/ui`) is the product and stays unread, `ui/SampleTimer.qml`
//! included — the quit dialog waits on it too.
//!
//! What a test — or a verb — may still do it says on the line:
//! `// waits(<purpose>): <reason>` above or beside the statement, with a
//! purpose from [`Purpose`]. A marker that covers nothing is a finding of
//! its own.

mod audit;
mod qml;
mod rust;
mod source;

pub use audit::run;

use source::Purpose;

use crate::command::{self, Permission, Where};

pub(crate) static WAITS: command::Command = command::Command {
    id: "waits.audit",
    call: "waits",
    purpose: "no test takes a stretch of clock for an answer",
    run_in: Where::Either,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static COMMANDS: &[&command::Command] = &[&WAITS];

/// A statement that breaks a rule.
#[derive(Debug)]
struct Finding {
    file: String,
    line: usize,
    rule: &'static str,
    /// The line of the statement, as the file has it.
    excerpt: String,
}

/// A statement a marker lets stand, by the purpose it names.
#[derive(Debug)]
struct Exception {
    file: String,
    line: usize,
    purpose: Purpose,
}

/// A statement that breaks a rule before its markers are read.
struct Candidate {
    first: usize,
    last: usize,
    /// The line it is named at and quoted from: its first, or the line
    /// of a script's sleep inside it.
    shown: usize,
    rule: &'static str,
}
