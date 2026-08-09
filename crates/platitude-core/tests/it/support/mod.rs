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

use platitude_core::process::{CommandEnd, CommandObserver};
use platitude_core::session::{LogRow, RefLabel, SessionEvent};

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
