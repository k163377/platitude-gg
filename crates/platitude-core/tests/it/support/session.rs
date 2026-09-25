//! What the `RepoSession` tests open, and what they watch it with.

use std::sync::{Arc, Mutex};

use crate::support::wait::bounded;
use crate::support::{Patience, TestRepo};
use platitude_core::session::{
    FollowUp, PassHooks, PassStep, RepoSession, SessionEvent, SessionSink,
};
use platitude_core::{GitError, OperationId, OperationKind};

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

/// Reads a landed pass out of the one event that ends it: a stream's
/// `LogFinished` or a rebuild's `LogReplaced`. Which shape carries a given
/// ask is a scheduling accident, so a wait must accept both
/// (rules-refs/core.md「`restart_log` の着地はストリームとは限らない」).
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
    /// middle of a read: a hook that parks holds the reader there while
    /// the test drives the rest.
    ///
    /// The hook runs under `block_in_place`; a bare park stalls the runtime
    /// on Linux (rules-refs/core.md
    /// 「hook で reader を停める配信は `block_in_place` の下で走らせる」).
    /// The sink records before it runs the hook, so a wait over recorded
    /// events sees the event that parked.
    pub fn hook_once(
        &self,
        when: impl Fn(&SessionEvent) -> bool + Send + 'static,
        run: impl FnOnce() + Send + 'static,
    ) {
        // Asserted where the hook is armed, so the failure has a name.
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

    /// Waits for a pass ending with `total` rows and returns the newest
    /// such, so its footer and generation belong to the same pass. A
    /// baseline also needs the operation that owns it awaited.
    pub async fn settled_pass(&self, total: u32) -> Pass {
        self.wait_for(&format!("a {total}-row graph pass"), |evs| {
            evs.iter()
                .filter_map(pass_of)
                .filter(|pass| pass.total == total)
                .max_by_key(|pass| pass.generation)
        })
        .await
    }

    /// Establishes the opening graph baseline by causal waits, in the order
    /// rules-refs/core.md「baseline は開始条件を列挙して待つ」 gives.
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
        crate::support::wait::bounded(
            "the graph passes the baseline displaced",
            session.wait_for_graph_passes(),
        )
        .await;
        self.settled_pass(total).await
    }

    /// Waits until both opening snapshots have landed *and* their readers
    /// have left the single flight — the events are sent from inside the
    /// readers, which then ask for a rebuild, so the events alone are not
    /// the boundary.
    pub async fn opening_settled(&self, session: &Arc<RepoSession>) {
        self.opening_snapshots().await;
        crate::support::wait::bounded(
            "the opening snapshot reads",
            session.wait_for_snapshot_reads(),
        )
        .await;
    }

    /// Waits until the refs and status snapshots `open` starts have landed
    /// (`Opened` only means the path was accepted).
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

    /// Waits for the first graph pass after generation `after` to land, in
    /// either shape ([`pass_of`]). A pass that finds rows and footer
    /// unchanged says nothing (`run_swap_pass`), so the first one after is
    /// the reaction to the test's last ask, even when `after` is older than
    /// the newest pass.
    ///
    /// The caller must ask for something the graph on screen differs from,
    /// in rows or footer: an unchanged ask overtaken by a rebuild announces
    /// nothing.
    pub async fn pass_after(&self, what: &str, after: u64) -> Pass {
        self.wait_for(what, |evs| {
            evs.iter()
                .filter_map(pass_of)
                .find(|p| p.generation > after)
        })
        .await
    }

    /// Waits for `pred` over the event list; each delivery wakes it, and
    /// the budget is only a failure backstop.
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

    /// [`Self::wait_for`] across one long silent step the test means to
    /// take (a `periodic` test's read of this machine): the silence budget
    /// would call it a hang, so only the overall one stands ([`bounded`]).
    pub async fn wait_through_silence<T>(
        &self,
        what: &str,
        pred: impl Fn(&[SessionEvent]) -> Option<T>,
    ) -> T {
        let mut changed = self.changed.subscribe();
        bounded(what, async {
            loop {
                if let Some(v) = pred(&self.events.lock().unwrap()) {
                    return v;
                }
                changed
                    .changed()
                    .await
                    .expect("the sink outlives every wait on it");
            }
        })
        .await
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
        // Outside both locks, or a parked hook blocks the events it waits
        // on; under `block_in_place` (`hook_once`).
        if let Some(run) = run {
            tokio::task::block_in_place(run);
        }
    }
}

/// main: root ─ main work ─ merge ← side work; tag v1 on the merge, one stash.
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

/// `None` doors is how the application opens.
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

/// The two doors into a graph pass ([`PassHooks`]) that tests drive one
/// through; [`open_with_doors`] hands them over.
#[derive(Default)]
pub struct PassDoors {
    /// What the next pass to reach a given step runs there.
    step: Mutex<Option<(PassStep, Then)>>,
    /// The step every pass fails at from now on.
    fault: Mutex<Option<PassStep>>,
}

impl PassDoors {
    /// Runs `run` once, on the task of the next graph pass to reach `at`
    /// (`PassHooks::before`) — how a test ends a pass. Taken by that pass,
    /// so exactly one falls over.
    pub fn run_inside_next_pass(&self, at: PassStep, run: impl FnOnce() + Send + 'static) {
        *self.step.lock().unwrap() = Some((at, Box::new(run)));
    }

    /// Every graph pass that reaches `at` from here on fails there instead
    /// of walking — the app harness's screen door (`PassHooks::fault`),
    /// proved from this side too.
    pub fn fail_every_pass(&self, at: PassStep) {
        *self.fault.lock().unwrap() = Some(at);
    }
}

impl PassHooks for PassDoors {
    /// Taken out under the lock and run outside it, as the trait requires.
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

/// Whether the session said this write came to rest on a stop
/// ([`SessionEvent::WriteStopped`]).
///
/// Read after [`write_result`]: the stop is published before the answer.
pub fn write_stopped(sink: &CaptureSink, kind: OperationKind) -> bool {
    sink.events
        .lock()
        .unwrap()
        .iter()
        .any(|e| matches!(e, SessionEvent::WriteStopped { kind: got, .. } if *got == kind))
}

/// Waits for the first write of `kind` to finish and returns git's error,
/// if any — for a test that made one write of it; writes that look alike
/// wait by id ([`write_answer`]).
pub async fn write_result(sink: &CaptureSink, kind: OperationKind) -> Option<String> {
    sink.wait_for(kind.label(), |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::WriteFinished {
                kind: got, error, ..
            } if *got == kind => Some(error.clone()),
            _ => None,
        })
    })
    .await
}

/// Waits for the write accepted under `id` to finish and returns git's
/// error, if any.
pub async fn write_answer(sink: &CaptureSink, id: OperationId) -> Option<String> {
    sink.wait_for("the write's own answer", |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::WriteFinished { id: got, error, .. } if *got == id => Some(error.clone()),
            _ => None,
        })
    })
    .await
}

/// Waits for the reads the write accepted under `id` invalidated to have
/// been made — its last boundary ([`SessionEvent::WriteSettled`]) — and
/// returns the ones that did not land, empty where every read did.
pub async fn write_settled(sink: &CaptureSink, id: OperationId) -> Vec<FollowUp> {
    sink.wait_for("the write settled", |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::WriteSettled {
                id: got, failed, ..
            } if *got == id => Some(failed.clone()),
            _ => None,
        })
    })
    .await
}

/// Puts the todo-editor helper where the session looks for it — beside the
/// running executable, here the test binary's directory.
///
/// Once per process and published by `rename`: copying straight onto the
/// path puts one test's write fd under another's exec (`ETXTBSY` on Linux
/// — rules-refs/core.md「テストが差し込む実行ファイルは rename で置く」).
pub fn install_todo_editor() {
    static INSTALLED: std::sync::Once = std::sync::Once::new();
    // Every caller waits for the one copy; the rename covers other
    // processes sharing this `target/`.
    INSTALLED.call_once(|| {
        let built = std::path::PathBuf::from(env!("CARGO_BIN_EXE_pgg-todo-editor"));
        let Some(dir) = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
        else {
            return;
        };
        // A refused install fails here, by name, not as an unrelated
        // failure later (`publish_helper`).
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
    // One staging name per process: `INSTALLED` runs one copy at a time.
    let staged = dir.join(format!("{name}.{}.staged", std::process::id()));
    let published = std::fs::copy(built, &staged).and_then(|_| std::fs::rename(&staged, &beside));
    if let Err(error) = published {
        // Windows locks a running executable (still readable), so the
        // rename can lose to a helper another run left behind. Only one
        // holding this build's bytes will do. The refusal names its
        // condition: the raw rename error reads as an unrelated permission
        // problem.
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
