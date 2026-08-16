//! What the `RepoSession` tests open, and what they watch it with.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::support::{Patience, TestRepo};
use platitude_core::GitExecutor;
use platitude_core::session::{RepoSession, SessionEvent, SessionSink};

/// Picks the event a [`CaptureSink`] hook fires on.
type When = Box<dyn Fn(&SessionEvent) -> bool + Send>;
/// What runs inside that event's delivery.
type Then = Box<dyn FnOnce() + Send>;

/// What a graph pass reports when it lands, whichever of the two shapes
/// it landed in.
#[derive(Debug, Clone, Copy)]
pub struct Pass {
    pub generation: u64,
    /// Rows the pass put on screen (`LogFinished`'s `total`).
    pub total: u32,
    pub walked: u32,
    pub truncated: bool,
}

/// Reads a landed pass out of the one event that ends it.
///
/// A stream (`LogStarted` → chunks → `LogFinished`) and an atomic
/// replacement (`LogReplaced`) are two shapes of the same thing, and
/// which one carries a given ask is a scheduling accident. `restart_log`
/// asks for a stream, but that pass drops without a word the moment a
/// background rebuild is asked for over it — `run_direct_pass` returns on
/// a cancelled token, and a walk cancelled mid-stream reports neither
/// `LogFinished` nor `LogFailed` — and the rebuild behind it replaces
/// instead, carrying the very options the ask just changed (both entry
/// points read them after taking the token, so the winner is always the
/// one holding the new ones).
///
/// A test that waits for one shape is waiting on that race. 実測: a
/// window change on a loaded machine landed as `LogReplaced { generation:
/// 4 }` — the open sequence's dirty-flip `refresh_log` overtook the
/// stream at generation 3 — and the wait sat out its whole budget.
pub fn pass_of(event: &SessionEvent) -> Option<Pass> {
    match event {
        SessionEvent::LogFinished {
            generation,
            total,
            walked,
            truncated,
            ..
        } => Some(Pass {
            generation: *generation,
            total: *total,
            walked: *walked,
            truncated: *truncated,
        }),
        SessionEvent::LogReplaced {
            generation,
            rows,
            walked,
            truncated,
            ..
        } => Some(Pass {
            generation: *generation,
            total: rows.len() as u32,
            walked: *walked,
            truncated: *truncated,
        }),
        _ => None,
    }
}

pub struct CaptureSink {
    pub events: Mutex<Vec<SessionEvent>>,
    hook: Mutex<Option<(When, Then)>>,
}

impl CaptureSink {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            events: Mutex::new(Vec::new()),
            hook: Mutex::new(None),
        })
    }

    /// Runs `run` once, from inside the sink call that delivers the first
    /// event `when` accepts — the only place a test can stand in the
    /// middle of a read. Everything the session sends comes through here,
    /// so a hook that parks holds the reader there while the test drives
    /// the rest.
    ///
    /// A parked hook blocks the worker thread its reader runs on, and
    /// tokio leaves a task queued there queued: whatever has to run
    /// meanwhile must be started from the test's own thread, not from
    /// inside the hook.
    pub fn hook_once(
        &self,
        when: impl Fn(&SessionEvent) -> bool + Send + 'static,
        run: impl FnOnce() + Send + 'static,
    ) {
        *self.hook.lock().unwrap() = Some((Box::new(when), Box::new(run)));
    }

    pub fn count(&self, pred: impl Fn(&SessionEvent) -> bool) -> usize {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| pred(e))
            .count()
    }

    /// Waits until log streaming settles: a pass ending with `total`
    /// rows (a `LogFinished`, or a `LogReplaced` carrying that many
    /// rows) exists and no further stream event arrives for a beat,
    /// then returns the newest matching generation. Acting on the
    /// *first* matching pass instead would race the passes still in
    /// flight (the tag swap, the dirty-flip replacement), which finish
    /// afterwards with higher generations and would be mistaken for the
    /// reaction to whatever the test does next.
    pub async fn settled_stream_gen(&self, total: u32) -> u64 {
        fn stream_events(evs: &[SessionEvent]) -> usize {
            evs.iter().filter(|e| is_stream_event(e)).count()
        }
        let what = format!("the stream to settle at {total} rows");
        let mut patience = Patience::new();
        loop {
            let (newest, seen) = {
                let evs = self.events.lock().unwrap();
                let newest = evs
                    .iter()
                    .filter_map(pass_of)
                    .filter(|p| p.total == total)
                    .map(|p| p.generation)
                    .max();
                patience.note(evs.len());
                (newest, stream_events(&evs))
            };
            if let Some(g) = newest {
                tokio::time::sleep(Duration::from_millis(400)).await;
                if stream_events(&self.events.lock().unwrap()) == seen {
                    return g;
                }
            } else {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            patience.check(&what, &self.events);
        }
    }

    /// Waits for the first graph pass after generation `after` to land,
    /// in whichever shape it landed in (see [`pass_of`]).
    ///
    /// "The first one after" is the reaction to whatever the test asked
    /// for last, and nothing else: a pass that finds the graph unchanged
    /// swaps nothing and says nothing (`run_swap_pass`), so the only
    /// passes that speak are the ones an ask produced. That also makes it
    /// safe for `after` to be older than the newest settled pass — a
    /// duplicate of a graph already on screen could not have spoken.
    ///
    /// Which means the caller owes one thing: **ask for something the
    /// graph on screen differs from** — in its rows, or in the footer
    /// under them (`run_swap_pass` compares both, so a window that only
    /// moves `walked`/`truncated` does speak). A change that leaves the
    /// two exactly as they are has nothing to announce if a rebuild
    /// overtakes the stream, and no wait can conjure an event nobody sent.
    pub async fn pass_after(&self, what: &str, after: u64) -> Pass {
        self.wait_for(what, |evs| {
            evs.iter()
                .filter_map(pass_of)
                .find(|p| p.generation > after)
        })
        .await
    }

    /// Waits until the session has finished what it already had going
    /// (see [`crate::support::settled`]).
    pub async fn settled(&self) {
        crate::support::settled(&self.events).await;
    }

    /// Polls until `pred` over the event list returns `Some`, giving up
    /// only once the session has gone quiet on it (see [`Patience`]).
    pub async fn wait_for<T>(&self, what: &str, pred: impl Fn(&[SessionEvent]) -> Option<T>) -> T {
        let mut patience = Patience::new();
        loop {
            {
                let events = self.events.lock().unwrap();
                if let Some(v) = pred(&events) {
                    return v;
                }
                patience.note(events.len());
            }
            patience.check(what, &self.events);
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}

impl SessionSink for CaptureSink {
    fn event(&self, event: SessionEvent) {
        let run = {
            let mut slot = self.hook.lock().unwrap();
            let fires = slot.as_ref().is_some_and(|(when, _)| when(&event));
            fires.then(|| slot.take().map(|(_, run)| run)).flatten()
        };
        self.events.lock().unwrap().push(event);
        // Outside both locks: a parked hook must not hold the recording
        // shut, or the events it is waiting on could never be written.
        if let Some(run) = run {
            run();
        }
    }
}

/// main: root ─ a ─ merge ← side, tag v1 on merge target, one stash.
pub fn scenario() -> (TestRepo, String) {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    repo.git(&["checkout", "-b", "side"]);
    repo.commit_file("g.txt", "s\n", "side work");
    repo.git(&["checkout", "main"]);
    repo.commit_file("f.txt", "1\n", "main work");
    repo.git(&["merge", "--no-ff", "-m", "merge side", "side"]);
    repo.git(&["tag", "v1"]);
    repo.write_file("f.txt", "wip\n");
    repo.git(&["stash", "push", "-m", "wip stash"]);
    let head = repo.git(&["rev-parse", "HEAD"]);
    (repo, head)
}

pub async fn opened(repo: &TestRepo) -> (Arc<CaptureSink>, Arc<RepoSession>) {
    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );
    sink.wait_for("Opened", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::Opened { .. }))
            .then_some(())
    })
    .await;
    (sink, session)
}

/// Waits for the write named `op` to finish and returns git's error, if any.
pub async fn write_result(sink: &CaptureSink, op: &'static str) -> Option<String> {
    sink.wait_for(op, |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::WriteFinished { op: got, error } if *got == op => Some(error.clone()),
            _ => None,
        })
    })
    .await
}

/// Puts the todo-editor helper where the session looks for it — beside the
/// running executable, which for a test is the test binary's own directory.
/// Packaging carries the same obligation for the application.
///
/// Once per process, and the file is published by `rename` rather than
/// written where it stands. Both halves are about the same thing: on Linux
/// a file somebody holds open for writing cannot be executed at all
/// (`ETXTBSY`), and all five replaying tests call this and then hand the
/// path to git. Copying straight onto it put one test's write fd under
/// another's exec — `pg-todo-editor: Text file busy`, reported by the `sh`
/// git runs `GIT_SEQUENCE_EDITOR` through, on 6 runs out of 8 with a thread
/// per core (実測 24 cores; 規約 §テストが差し込む実行ファイルは rename で置く).
pub fn install_todo_editor() {
    static INSTALLED: std::sync::Once = std::sync::Once::new();
    // Every caller waits for the one copy, so no test reaches git while it
    // is in flight; the rename covers the rest — another process sharing
    // this `target/` never sees a partly-written helper either.
    INSTALLED.call_once(|| {
        let built = std::path::PathBuf::from(env!("CARGO_BIN_EXE_pg-todo-editor"));
        let Some(dir) = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
        else {
            return;
        };
        let _ = publish_helper(&built, &dir);
    });
}

/// Copies `built` into `dir` under the name the application looks for,
/// through a staging name of its own so the live path is never opened for
/// writing. Returns where it put it.
pub fn publish_helper(
    built: &std::path::Path,
    dir: &std::path::Path,
) -> std::io::Result<std::path::PathBuf> {
    let name = format!(
        "{}{}",
        platitude_core::sequencer::HELPER_NAME,
        std::env::consts::EXE_SUFFIX
    );
    let beside = dir.join(&name);
    if beside == built {
        return Ok(beside);
    }
    // One staging name per process is enough: `INSTALLED` means one copy
    // runs at a time, and a second process gets a name of its own.
    let staged = dir.join(format!("{name}.{}.staged", std::process::id()));
    let published = std::fs::copy(built, &staged).and_then(|_| std::fs::rename(&staged, &beside));
    if let Err(error) = published {
        // Windows locks a running executable, so the rename can lose to a
        // helper another run left behind — the one that is there will do.
        let _ = std::fs::remove_file(&staged);
        if !beside.is_file() {
            return Err(error);
        }
    }
    Ok(beside)
}

/// Every event a log stream can emit — what "the graph stayed silent"
/// counts.
pub fn is_stream_event(e: &SessionEvent) -> bool {
    matches!(
        e,
        SessionEvent::LogStarted { .. }
            | SessionEvent::LogChunk { .. }
            | SessionEvent::LogFinished { .. }
            | SessionEvent::LogReplaced { .. }
            | SessionEvent::LogFailed { .. }
    )
}
