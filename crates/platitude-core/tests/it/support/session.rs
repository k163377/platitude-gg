//! What the `RepoSession` tests open, and what they watch it with.

use std::sync::{Arc, Mutex};

use crate::support::{Patience, TestRepo};
use platitude_core::GitError;
use platitude_core::session::{PassHooks, PassStep, RepoSession, SessionEvent, SessionSink};

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
/// A test that waits for one shape is waiting on that race. measured: a
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
    changed: tokio::sync::watch::Sender<u64>,
}

impl CaptureSink {
    pub fn new() -> Arc<Self> {
        let (changed, _) = tokio::sync::watch::channel(0);
        Arc::new(Self {
            events: Mutex::new(Vec::new()),
            hook: Mutex::new(None),
            changed,
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
    ///
    /// And while parked, **only the test's own root future may wait for
    /// an event that has not been recorded yet** (the sink records before
    /// it runs the hook). The delivery that fires the hook wakes the
    /// waiters watching the sink, and a woken *spawned task* can land in
    /// the parked worker's LIFO slot — the one place stealing never
    /// reaches. Such a wait for a future event is then held captive by
    /// the very park it is supposed to release: no task runs, no timer
    /// serves the captive `Patience`, and the binary sits at 0% CPU until
    /// the CI kill. measured: a hook parked on the opening refs delivery
    /// plus a spawned wait for the tag-inclusive swap deadlocked exactly
    /// so in the container, deterministically, while passing on Windows.
    /// The root future is the one exception because its waker unparks the
    /// test thread directly instead of scheduling onto a worker — an
    /// implementation property of the runtime, so a wait that can be
    /// phrased over recorded events still should be.
    pub fn hook_once(
        &self,
        when: impl Fn(&SessionEvent) -> bool + Send + 'static,
        run: impl FnOnce() + Send + 'static,
    ) {
        // The parked worker is the premise, so it is asserted where it is
        // created: with a single worker nobody is left to drive the test,
        // and the failure would be a silent livelock instead of a name.
        // The default worker count is the machine's — the test declares
        // its own (`worker_threads = 2`).
        let workers = tokio::runtime::Handle::current().metrics().num_workers();
        assert!(
            workers >= 2,
            "hook_once parks a worker and needs at least 2, got {workers}: declare \
             #[tokio::test(flavor = \"multi_thread\", worker_threads = 2)]"
        );
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

    /// Waits for a pass ending with `total` rows (a `LogFinished`, or a
    /// `LogReplaced`) and returns it — the newest, when several landed
    /// that way, so the footer read off it belongs to the same pass the
    /// generation anchors. Callers that need a baseline must additionally
    /// await the operation that owns it; a quiet interval cannot establish
    /// that no later opening work exists.
    pub async fn settled_pass(&self, total: u32) -> Pass {
        self.wait_for(&format!("a {total}-row graph pass"), |evs| {
            evs.iter()
                .filter_map(pass_of)
                .filter(|pass| pass.total == total)
                .max_by_key(|pass| pass.generation)
        })
        .await
    }

    /// Establishes an opening graph baseline without a quiet window.
    ///
    /// The tracked poll owns current refs and status reads and includes the
    /// graph refresh either requested. The matching pass then proves that
    /// the expected graph is actually installed, irrespective of which
    /// opening task won the scheduler race.
    pub async fn opened_graph(&self, session: &Arc<RepoSession>, total: u32) -> Pass {
        self.opening_settled(session).await;
        let outcome = crate::support::wait::bounded(
            "the tracked opening graph refresh",
            session.refresh_log_tracked().outcome(),
        )
        .await;
        assert!(
            matches!(
                outcome,
                platitude_core::session::RefreshOutcome::Changed
                    | platitude_core::session::RefreshOutcome::Unchanged
            ),
            "the opening graph was available: {outcome:?}"
        );
        // The ask above took the stream over, and taking it over cancels
        // the pass that had it rather than ending it: the opening's
        // tag-inclusive pass stops when it next looks. A baseline read
        // before that is one the opening is still adding to — a walk it
        // spawns afterwards lands past the count and reads as work the
        // test's own subject asked for.
        crate::support::wait::bounded(
            "the graph passes the baseline displaced",
            session.wait_for_graph_passes(),
        )
        .await;
        self.settled_pass(total).await
    }

    /// Closes the opening baseline: both opening snapshots have landed
    /// *and* their readers have left the single flight. The snapshot
    /// events are sent from inside the readers, which then ask for the
    /// rebuild those answers imply, so the events alone leave the flight
    /// occupied and a count taken then still sees opening work
    /// (`wait_for_snapshot_reads` is the boundary).
    pub async fn opening_settled(&self, session: &Arc<RepoSession>) {
        self.opening_snapshots().await;
        crate::support::wait::bounded(
            "the opening snapshot reads",
            session.wait_for_snapshot_reads(),
        )
        .await;
    }

    /// Waits until the two repository snapshots started by `open` have
    /// landed. `Opened` only means the path was accepted; refs and status
    /// are deliberately started after that event, so it is not a safe
    /// baseline for a test that counts their work.
    pub async fn opening_snapshots(&self) {
        self.wait_for("the opening snapshots", |events| {
            let refs = events
                .iter()
                .any(|event| matches!(event, SessionEvent::RefsLoaded { .. }));
            let status = events
                .iter()
                .any(|event| matches!(event, SessionEvent::StatusLoaded { .. }));
            (refs && status).then_some(())
        })
        .await;
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

    /// Waits for `pred` over the event list. Event delivery wakes this
    /// waiter directly; the timeout budget remains only a failure backstop.
    pub async fn wait_for<T>(&self, what: &str, pred: impl Fn(&[SessionEvent]) -> Option<T>) -> T {
        let mut patience = Patience::new();
        let mut changed = self.changed.subscribe();
        loop {
            {
                let events = self.events.lock().unwrap();
                if let Some(v) = pred(&events) {
                    return v;
                }
                patience.note(events.len());
            }
            patience.check(what, &self.events);
            if tokio::time::timeout(patience.remaining(), changed.changed())
                .await
                .is_err()
            {
                patience.check(what, &self.events);
            }
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
        self.changed
            .send_modify(|generation| *generation = generation.wrapping_add(1));
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
    opened_with(repo, crate::support::exec::isolated()).await
}

/// [`opened`] without the wait — for a test whose first assertion is about
/// an event before, or instead of, `Opened`.
pub fn open_unawaited(repo: &TestRepo) -> (Arc<CaptureSink>, Arc<RepoSession>) {
    start(repo, crate::support::exec::isolated(), None)
}

/// [`open_unawaited`], with the doors into its graph passes in hand — for
/// the tests that end a pass from inside it.
pub fn open_with_doors(repo: &TestRepo) -> (Arc<CaptureSink>, Arc<RepoSession>, Arc<PassDoors>) {
    let doors = Arc::new(PassDoors::default());
    let hooks: Arc<dyn PassHooks> = Arc::clone(&doors) as _;
    let (sink, session) = start(repo, crate::support::exec::isolated(), Some(hooks));
    (sink, session, doors)
}

/// [`opened`] on an executor of the caller's own — for a session that has
/// to see the repository's private `--global` file
/// (`exec::isolated_global`), or one watched by an observer.
pub async fn opened_with(
    repo: &TestRepo,
    exec: platitude_core::process::GitExecutor,
) -> (Arc<CaptureSink>, Arc<RepoSession>) {
    let (sink, session) = start(repo, exec, None);
    sink.wait_for("Opened", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::Opened { .. }))
            .then_some(())
    })
    .await;
    (sink, session)
}

/// Opens `repo` on `exec`, watched by a fresh sink. `None` for the doors
/// is the shape the application opens in, and the one every test not
/// about the passes themselves opens in too.
fn start(
    repo: &TestRepo,
    exec: platitude_core::process::GitExecutor,
    doors: Option<Arc<dyn PassHooks>>,
) -> (Arc<CaptureSink>, Arc<RepoSession>) {
    let sink = CaptureSink::new();
    let session = RepoSession::open(
        exec,
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
        doors,
    );
    (sink, session)
}

/// The two doors into a graph pass the tests drive one through, let in
/// by the seam the session is opened with ([`PassHooks`]) —
/// [`open_with_doors`] hands them over.
#[derive(Default)]
pub struct PassDoors {
    /// What the next pass to reach a given step runs there.
    step: Mutex<Option<(PassStep, Then)>>,
    /// The step every pass fails at from now on.
    fault: Mutex<Option<PassStep>>,
}

impl PassDoors {
    /// Leaves `run` for the next graph pass to reach `at`, to be run
    /// there, on that pass's own task, once — the door a test ends a
    /// pass through (`PassHooks::before`). Taken by the pass that runs
    /// it, so exactly one falls over; a pass reaching a step nobody left
    /// anything at is a lock and a look.
    pub fn run_inside_next_pass(&self, at: PassStep, run: impl FnOnce() + Send + 'static) {
        *self.step.lock().unwrap() = Some((at, Box::new(run)));
    }

    /// Every graph pass that reaches `at` from here on fails there, in
    /// place of the walk it would have made — the door a screen is driven
    /// through (`PassHooks::fault`), held here so the arm it reaches the
    /// walk by is proved from this side too.
    pub fn fail_every_pass(&self, at: PassStep) {
        *self.fault.lock().unwrap() = Some(at);
    }
}

impl PassHooks for PassDoors {
    /// Taken out under the lock and run outside it: what it is here to do
    /// is unwind, and a guard held across that would poison the lock —
    /// which the pass behind this one would then take, run, and unwind
    /// through in turn.
    fn before(&self, at: PassStep) {
        let run = {
            let mut left = self.step.lock().unwrap();
            match left.take() {
                Some((step, run)) if step == at => Some(run),
                other => {
                    *left = other;
                    None
                }
            }
        };
        if let Some(run) = run {
            run();
        }
    }

    fn fault(&self, at: PassStep) -> Option<GitError> {
        let standing = *self.fault.lock().unwrap();
        (standing == Some(at)).then(|| GitError::UnexpectedOutput {
            command: "git log".to_string(),
            message: "the graph walk was made to fail".to_string(),
        })
    }
}

/// Whether the session said this write came to rest on a stop rather
/// than on a commit ([`SessionEvent::WriteStopped`]).
///
/// Read after [`write_result`], which is what does the waiting: the stop
/// is published between the write's start and its answer, so by the time
/// the answer has arrived this is settled.
pub fn write_stopped(sink: &CaptureSink, op: &'static str) -> bool {
    sink.events
        .lock()
        .unwrap()
        .iter()
        .any(|e| matches!(e, SessionEvent::WriteStopped { op: got } if *got == op))
}

/// Waits for the write named `op` to finish and returns git's error, if any.
pub async fn write_result(sink: &CaptureSink, op: &'static str) -> Option<String> {
    sink.wait_for(op, |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::WriteFinished { op: got, error, .. } if *got == op => Some(error.clone()),
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
/// another's exec — `pgg-todo-editor: Text file busy`, reported by the `sh`
/// git runs `GIT_SEQUENCE_EDITOR` through, on 6 runs out of 8 with a thread
/// per core (measured, 24 cores; 規約 §テストが差し込む実行ファイルは rename で置く).
pub fn install_todo_editor() {
    static INSTALLED: std::sync::Once = std::sync::Once::new();
    // Every caller waits for the one copy, so no test reaches git while it
    // is in flight; the rename covers the rest — another process sharing
    // this `target/` never sees a partly-written helper either.
    INSTALLED.call_once(|| {
        let built = std::path::PathBuf::from(env!("CARGO_BIN_EXE_pgg-todo-editor"));
        let Some(dir) = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
        else {
            return;
        };
        // A refused install must fail here, by name — swallowing it would
        // run whatever helper is already there under this build's
        // assertions, and the tests would fail somewhere unrelated later.
        publish_helper(&built, &dir).expect("publish the todo editor beside the test binary");
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
        // helper another run left behind. One holding this build's bytes
        // will do; anything else would run yesterday's helper under
        // today's assertions, and stopping here is what keeps that from
        // surfacing as an unrelated failure later. (A running exe stays
        // readable on Windows — only writing and renaming are refused.)
        // The refusal names its own condition: the raw rename error alone
        // reads as an unrelated permission problem.
        let _ = std::fs::remove_file(&staged);
        let refused = match (std::fs::read(built), std::fs::read(&beside)) {
            (Ok(want), Ok(have)) if have == want => None,
            (Ok(_), Ok(_)) => Some("holds different bytes than this build"),
            _ => Some("could not be read for comparison"),
        };
        if let Some(why) = refused {
            return Err(std::io::Error::other(format!(
                "the helper already at {} {why}, and replacing it failed: {error}",
                beside.display()
            )));
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
