//! Integration-test support: builds real repositories in temp directories
//! with the system git CLI, isolated from the developer's global config and
//! fully deterministic (fixed identities and timestamps → stable SHAs).

// Test-only helper: panicking on setup failure is the desired behavior, but
// the `allow-*-in-tests` clippy options only cover `#[test]` functions.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic, dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use platitude_core::process::{CommandEnd, CommandObserver};
use platitude_core::session::{LogRow, RefLabel, SessionEvent};

pub mod integrate;
pub mod session;

/// Base timestamp for deterministic commits (arbitrary fixed epoch).
const BASE_EPOCH: u64 = 1_700_000_000;

pub struct TestRepo {
    // Kept alive for the lifetime of the repo; dropped last.
    _dir: tempfile::TempDir,
    /// Replaces the developer's global config for TestRepo-spawned git only
    /// (the code under test inherits the process environment instead and
    /// reads the repo-local config below).
    global_config: PathBuf,
    pub path: PathBuf,
    tick: u64,
}

/// Written into `.git/config` right after init — one file write instead of
/// five `git config` spawns per repo (process spawns dominate suite time on
/// Windows). Must stay repo-local: the code under test does not see
/// `global_config`, only this file. `{autocrlf}` is filled by
/// [`TestRepo::init`] / [`TestRepo::init_autocrlf`].
const REPO_CONFIG: &str = "\
[user]
\tname = Test User
\temail = test@example.com
[commit]
\tgpgsign = false
[tag]
\tgpgSign = false
[core]
\tautocrlf = {autocrlf}
";

/// Test-only speed knobs for TestRepo-spawned git (setup commits are the
/// bulk of the suite's writes): no fsync — repos are throwaway — and no
/// auto-gc mid-test. Unknown keys are ignored by older git, so nothing here
/// is a compatibility constraint.
const GLOBAL_CONFIG: &str = "\
[core]
\tfsync = none
[gc]
\tauto = 0
";

impl TestRepo {
    /// A repository with `core.autocrlf=false`: git stores and checks out
    /// bytes verbatim, so what a test writes is what the tests see.
    pub fn init() -> Self {
        Self::init_with_autocrlf("false")
    }

    /// A repository with `core.autocrlf=true` — the setting the Git for
    /// Windows installer offers by default, where git converts CRLF to LF on
    /// the way into the index and back on the way out. Pinning behaviour
    /// under it is the only way the conversion path gets walked at all; a
    /// suite that only ever runs with `false` has never seen what most
    /// Windows checkouts do.
    pub fn init_autocrlf() -> Self {
        Self::init_with_autocrlf("true")
    }

    fn init_with_autocrlf(autocrlf: &str) -> Self {
        let dir = tempfile::tempdir().expect("create tempdir");
        let path = dir.path().join("repo");
        let global_config = dir.path().join("global-config");
        std::fs::write(&global_config, GLOBAL_CONFIG).expect("write global config");
        std::fs::create_dir(&path).expect("create repo dir");
        let mut repo = Self {
            _dir: dir,
            global_config,
            path,
            tick: 0,
        };
        repo.git(&["init", "-b", "main"]);
        let config = repo.path.join(".git").join("config");
        let existing = std::fs::read_to_string(&config).expect("read repo config");
        let repo_config = REPO_CONFIG.replace("{autocrlf}", autocrlf);
        std::fs::write(&config, format!("{existing}{repo_config}")).expect("write repo config");
        repo
    }

    /// Runs git in the repo and panics on failure. Returns trimmed stdout.
    pub fn git(&mut self, args: &[&str]) -> String {
        let dir = self.path.clone();
        self.git_in(&dir, args)
    }

    /// Like [`TestRepo::git`] but returns raw stdout bytes (for capturing
    /// parser fixtures byte-exactly).
    pub fn git_raw(&mut self, args: &[&str]) -> Vec<u8> {
        let dir = self.path.clone();
        let out = self.run(&dir, args);
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
        out.stdout
    }

    /// Runs git and reports only whether it succeeded — for the commands
    /// whose exit code *is* the answer (`check-ref-format`).
    pub fn git_ok(&mut self, args: &[&str]) -> bool {
        let dir = self.path.clone();
        self.run(&dir, args).status.success()
    }

    /// Runs git expecting a non-zero exit (e.g. a conflicting merge).
    pub fn git_expect_failure(&mut self, args: &[&str]) {
        let dir = self.path.clone();
        let out = self.run(&dir, args);
        assert!(!out.status.success(), "git {args:?} unexpectedly succeeded");
    }

    /// Runs git in an arbitrary directory (e.g. for `clone`).
    pub fn git_in(&mut self, dir: &Path, args: &[&str]) -> String {
        let out = self.run(dir, args);
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    fn run(&mut self, dir: &Path, args: &[&str]) -> Output {
        self.tick += 1;
        let stamp = format!("{} +0000", BASE_EPOCH + self.tick * 60);
        Command::new("git")
            .args(args)
            .current_dir(dir)
            // Isolate from developer/global configuration.
            .env("GIT_CONFIG_GLOBAL", &self.global_config)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("LC_ALL", "C")
            // Deterministic identities and times.
            .env("GIT_AUTHOR_NAME", "Test User")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_NAME", "Test User")
            .env("GIT_COMMITTER_EMAIL", "test@example.com")
            .env("GIT_AUTHOR_DATE", &stamp)
            .env("GIT_COMMITTER_DATE", &stamp)
            .output()
            .expect("spawn git")
    }

    /// Writes a file (creating parent dirs) relative to the work tree.
    pub fn write_file(&self, rel: &str, content: &str) {
        let p = self.path.join(rel);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).expect("create parent dirs");
        }
        std::fs::write(p, content).expect("write file");
    }

    /// Writes + stages + commits a file; returns the commit id.
    pub fn commit_file(&mut self, rel: &str, content: &str, message: &str) -> String {
        self.write_file(rel, content);
        self.git(&["add", "--", rel]);
        self.git(&["commit", "-m", message]);
        self.git(&["rev-parse", "HEAD"])
    }

    /// `file://` URL of this repository, so remote-protocol paths can be
    /// exercised without a network (実装計画 §11.3).
    pub fn file_url(&self) -> String {
        let mut p = self.path.to_string_lossy().replace('\\', "/");
        // Windows paths start with a drive letter; the URL needs a root.
        if !p.starts_with('/') {
            p.insert(0, '/');
        }
        format!("file://{p}")
    }
}

/// One row as a consumer of the event stream ends up showing it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeenRow {
    pub oid_hex: String,
    pub labels: Vec<RefLabel>,
}

/// Replays the graph events the way the UI model does (`GraphModel::drain`
/// in platitude-app), keyed by row number.
///
/// Rows arrive with the walk and their chips are corrected afterwards: a
/// fetch that moves no ref this repository holds rebuilds nothing, so what
/// the remote turned out to have reaches the graph as `LabelsChanged`
/// against rows already on screen. Reading only the walk would miss it.
///
/// The generation rules are part of the model and are kept here: a message
/// about a graph that has been replaced is dropped rather than applied to
/// whatever now sits at those row numbers.
pub fn replay_graph(events: &[SessionEvent]) -> BTreeMap<u32, SeenRow> {
    fn take(batch: &[LogRow], into: &mut BTreeMap<u32, SeenRow>) {
        for r in batch {
            into.insert(
                r.row,
                SeenRow {
                    oid_hex: r.oid_hex.clone(),
                    labels: r.labels.clone(),
                },
            );
        }
    }
    let mut generation = 0;
    let mut rows: BTreeMap<u32, SeenRow> = BTreeMap::new();
    for event in events {
        match event {
            SessionEvent::LogStarted { generation: g } if *g > generation => {
                generation = *g;
                rows.clear();
            }
            SessionEvent::LogChunk {
                generation: g,
                rows: chunk,
            } if *g == generation => take(chunk, &mut rows),
            SessionEvent::LogReplaced {
                generation: g,
                rows: fresh,
                ..
            } if *g > generation => {
                generation = *g;
                rows.clear();
                take(fresh, &mut rows);
            }
            SessionEvent::LabelsChanged {
                generation: g,
                rows: changed,
            } if *g == generation => {
                for (row, labels) in changed {
                    if let Some(seen) = rows.get_mut(row) {
                        seen.labels = labels.clone();
                    }
                }
            }
            _ => {}
        }
    }
    rows
}

/// How long a wait puts up with the session saying nothing. Every event
/// renews it, so what spends it is silence — not the wait taking a while.
pub const QUIET_BUDGET: Duration = Duration::from_secs(20);

/// The whole of a wait, as a backstop under [`QUIET_BUDGET`]: a session
/// talking without ever getting to the answer renews the silence budget
/// forever, and only a livelock reaches this one.
pub const OVERALL_BUDGET: Duration = Duration::from_secs(300);

/// What a wait spends while it waits.
///
/// A wait is here to catch a session that stopped, and a budget counted
/// from the first poll cannot tell that from one that is merely slow.
/// `session_integration::concurrent_writes_are_serialized` puts 12 writes
/// through the queue one at a time, and under `cargo test --workspace` —
/// 264 integration tests, a thread per core, all spawning git — one round
/// trip takes ~2.5s: 実測, 20 seconds bought 8 of them, where the test on
/// its own finished all 12 in 4.7s. Raising the number until that fits
/// would hand every other wait in the suite the same head start before it
/// notices a hang.
///
/// Progress is what tells the two apart, so that is what the budget is
/// counted against: every event renews it, and only a session gone quiet
/// spends it. Same reading as 規約 §「もう起きない」を sleep で確かめない —
/// a stretch of clock is not a state. A hang still needs the same
/// [`QUIET_BUDGET`] of nothing to be called one; it is only the waits that
/// are demonstrably being answered that no longer pay for it.
pub struct Patience {
    started: Instant,
    quiet_since: Instant,
    seen: usize,
}

impl Patience {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            started: now,
            quiet_since: now,
            seen: 0,
        }
    }

    /// Renews the budget if the session has said anything since the last
    /// look.
    pub fn note(&mut self, events: usize) {
        if events != self.seen {
            self.seen = events;
            self.quiet_since = Instant::now();
        }
    }

    /// How long the session has been saying nothing.
    pub fn quiet_for(&self) -> Duration {
        self.quiet_since.elapsed()
    }

    /// Fails the test once the budget is spent. Call it with no lock held
    /// — the dump it prints takes one.
    pub fn check(&self, what: &str, events: &Mutex<Vec<SessionEvent>>) {
        let (quiet, whole) = (self.quiet_since.elapsed(), self.started.elapsed());
        assert!(
            quiet < QUIET_BUDGET && whole < OVERALL_BUDGET,
            "timed out waiting for {what} after {whole:?}, the last \
             {quiet:?} of it in silence; events so far: {:?}",
            events.lock().unwrap()
        );
    }
}

impl Default for Patience {
    fn default() -> Self {
        Self::new()
    }
}

/// How long nothing may happen before [`settled`] calls the session done.
///
/// Only the gap between one git command ending and the next one starting
/// has to fit in here — a command that is *running* is not silence but an
/// outstanding start (see [`settled`]) — so this is scheduling time, not
/// git time. 実測 under five copies of the suite at 32 threads each: the
/// opening's tag-inclusive pass took 639ms to get its `log -z` out of the
/// door after the tag-less one finished, on a machine where a single git
/// spawn was taking upwards of a second. Three times that, and still an
/// order under [`QUIET_BUDGET`], so a session that has genuinely hung is
/// still called one on the same terms as everywhere else.
const SETTLE_WINDOW: Duration = Duration::from_secs(2);

/// Whether every git command the session has announced has also reported
/// back. A command that is still running says nothing while it runs, and
/// on a loaded machine it says nothing for seconds (実測: 5830ms for a
/// two-commit walk) — long enough for a wait that only counts silence to
/// call the session finished in the middle of its opening.
///
/// Commands that are not being recorded report neither end, so the two
/// counts stay in step whatever `set_record_background` is set to; what
/// they cost is visibility, which is why [`settled`] is only as good as
/// how early recording was switched on.
fn nothing_running(events: &[SessionEvent]) -> bool {
    let (mut started, mut finished) = (0usize, 0usize);
    for event in events {
        match event {
            SessionEvent::CommandStarted { .. } => started += 1,
            SessionEvent::CommandFinished { .. } => finished += 1,
            _ => {}
        }
    }
    started == finished
}

/// Waits until the session has stopped doing things: nothing running, and
/// nothing said for a [`SETTLE_WINDOW`].
///
/// For taking a *baseline* — a count of what has happened so far, against
/// which whatever the test does next is measured. The trap that shape
/// walks into is that a session which has not been asked for anything for
/// a moment is not the same as one that has finished what it was already
/// doing: work set in motion by the opening lands whenever it lands, and
/// under load that is after a test on an idle machine would have finished
/// reading. Anything still in flight then gets counted as the reaction to
/// what the test did next.
///
/// Counted against silence rather than a fixed number of looks, for the
/// same reason [`Patience`] is (規約 §「もう起きない」を sleep で確かめない),
/// and it borrows `Patience` for the giving up: a command that never
/// reports back is a wait that never goes quiet, and gets the same
/// [`QUIET_BUDGET`] and dump as any other stuck wait.
pub async fn settled(events: &Mutex<Vec<SessionEvent>>) {
    let mut patience = Patience::new();
    loop {
        let (running, quiet) = {
            let evs = events.lock().unwrap();
            patience.note(evs.len());
            (!nothing_running(&evs), patience.quiet_for())
        };
        if !running && quiet >= SETTLE_WINDOW {
            return;
        }
        patience.check("the session to settle", events);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// Collects every [`CommandEnd`] the executor reports, for tests that read
/// exit codes off the command-log path (`GitExecutor::observed`).
#[derive(Default)]
pub struct Ends(pub Mutex<Vec<CommandEnd>>);

impl CommandObserver for Ends {
    fn records(&self, _user: bool) -> bool {
        true
    }
    fn started(&self, _display: &str, _full: &str, _user: bool) -> u64 {
        0
    }
    fn finished(&self, _id: u64, end: CommandEnd, _elapsed_ms: u64, _message: &str) {
        self.0.lock().unwrap().push(end);
    }
}
