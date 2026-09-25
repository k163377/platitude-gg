//! Whose a seat is: the marks a session writes into `git worktree
//! lock`'s reason, and what they settle. Apart from the roster, which
//! measures letters: this is about sessions, and is also read from
//! outside the roster (the entry hooks, `land`, the perf rig).
//!
//! A seat's claim is a conversation's, and no process is asked about it:
//! the Claude process ends whenever the app restarts while the
//! conversation goes on, so a claim judged by its process would hand the
//! letter to a stranger. It moves three ways only — the branch lands
//! (`land::release_claim`), the session hands it back (`seats::release`),
//! or the user has it taken over (`seats::takeover`). The rig's claim is
//! the one that is a process's (`Held::ByRunner`).

/// The mark a seat claim carries in `git worktree lock`'s reason:
/// `claude-seat <session> pid <pid>`. For a seat the pid only names the tab
/// that wrote it (`standing`); anything after it is ignored (older builds
/// wrote more). A lock without this mark is a person's, and nothing
/// automatic touches it.
pub(crate) const SEAT_CLAIM: &str = "claude-seat";

/// This session, as a claim records it.
pub(crate) struct Identity {
    /// The conversation: what a claim is matched on, kept across every
    /// process it is run from.
    pub session: String,
    /// The Claude process, when the environment names one — for the rig,
    /// the claim's whole liveness.
    pub pid: Option<u32>,
}

impl Identity {
    /// This session's marks: the hook payload's session id, else the
    /// environment's (Claude's, then Codex's task id); the pid is
    /// CLAUDE_PID's alone.
    pub(crate) fn current(session: Option<&str>) -> Self {
        let session = session_id(
            session,
            std::env::var("CLAUDE_CODE_SESSION_ID").ok().as_deref(),
            std::env::var("CODEX_THREAD_ID").ok().as_deref(),
        );
        let pid = std::env::var("CLAUDE_PID")
            .ok()
            .and_then(|pid| pid.trim().parse().ok());
        Self { session, pid }
    }

    pub(super) fn require_session(&self) -> Result<(), String> {
        if self.session.trim().is_empty() {
            Err("missing session identity: supply the hook session_id, \
                 CLAUDE_CODE_SESSION_ID or CODEX_THREAD_ID before claiming or releasing a seat"
                .into())
        } else {
            Ok(())
        }
    }

    /// What this session writes into a lock's reason.
    pub(crate) fn reason(&self) -> String {
        match self.pid {
            Some(pid) => format!("{SEAT_CLAIM} {} pid {pid}", self.session),
            None => format!("{SEAT_CLAIM} {}", self.session),
        }
    }

    /// This session as a person reads it beside a lock's reason.
    pub(crate) fn mark(&self) -> String {
        match self.pid {
            Some(pid) => format!("session {} (pid {pid})", self.session),
            None => format!("session {}", self.session),
        }
    }
}

fn session_id(hook: Option<&str>, claude: Option<&str>, codex: Option<&str>) -> String {
    [hook, claude, codex]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|id| !id.is_empty())
        .unwrap_or_default()
        .to_string()
}

/// What kind of holder a claim's pid names, which decides whether the
/// number is asked about at all.
#[derive(Clone, Copy)]
pub(crate) enum Held {
    /// By a Claude session: every seat claim. The number is never asked
    /// (module doc).
    BySession,
    /// By this task runner, out of its own pid: the perf rig's claim
    /// (`perf::rig`). A measurement is one process, so a claim whose
    /// runner is gone is litter the next measurement may clear.
    ByRunner,
}

/// A claim as a person reads it: whose it is, by the marks it carries.
pub(crate) fn whose(reason: &str) -> String {
    match holder(reason) {
        // A claim written with no id: ours to lift (`is_a_seat_claim`),
        // but "held by session " would send a reader looking for nobody.
        Some(holder) if holder.session.is_empty() => {
            "held by a session that gave no id of its own".to_string()
        }
        Some(holder) => format!("held by {}", holder.mark()),
        None if reason.trim().is_empty() => "locked without a reason".to_string(),
        None => format!("locked by hand ({reason})"),
    }
}

/// Whether a lock's reason is one of ours at all — a claim, rather than
/// a lock a person wrote by hand.
pub(crate) fn is_a_seat_claim(reason: &str) -> bool {
    holder(reason).is_some()
}

/// Who a claim names (session and pid only), read back out of a lock's
/// reason; None for a person's lock. The mark ends at a word:
/// `claude-seats are mine` is a person's sentence, and reading it as a
/// claim would hand their lock to the first takeover that asked.
fn holder(reason: &str) -> Option<Identity> {
    let rest = reason.strip_prefix(SEAT_CLAIM)?;
    if !rest.is_empty() && !rest.starts_with(' ') {
        return None;
    }
    let rest = rest.trim_start();
    let (session, marks) = match rest.split_once(" pid ") {
        Some((session, marks)) => (session, Some(marks)),
        None => (rest, None),
    };
    let pid = marks
        .and_then(|marks| marks.split_whitespace().next())
        .and_then(|pid| pid.parse().ok());
    Some(Identity {
        session: session.trim().to_string(),
        pid,
    })
}

/// Where a seat's lock stands relative to this session. Pure but for the
/// rig's liveness probe, so the tests can ask.
pub(crate) enum Standing {
    /// No lock at all.
    Free,
    /// This session's claim.
    Ours,
    /// The rig's claim with its runner gone; never a seat's (`standing`).
    Stale(String),
    /// Somebody else's claim, or a lock a person wrote by hand.
    Foreign(String),
}

/// Where a claim stands for this session. Nothing unaccountable is read
/// as its own — the session would share the tree with whoever wrote it.
/// `held` says whether another's pid may be probed: the rig's may, a
/// seat's never.
pub(crate) fn standing(reason: Option<String>, me: &Identity, held: Held) -> Standing {
    let Some(reason) = reason else {
        return Standing::Free;
    };
    let Some(holder) = holder(&reason) else {
        return Standing::Foreign(reason);
    };
    if !me.session.is_empty() && holder.session == me.session {
        return Standing::Ours;
    }
    match (held, holder.pid) {
        (Held::ByRunner, Some(pid)) if !crate::subprocess::task_runner_exists(pid) => {
            Standing::Stale(reason)
        }
        _ => Standing::Foreign(reason),
    }
}

/// Takes `seat_path` for this session if it can: a free seat is locked, a
/// stale rig claim lifted and replaced, anybody else's left. Only `Ours`
/// means the session may work there.
pub(crate) fn take_seat(cwd: &str, seat_path: &str, me: &Identity, held: Held) -> Standing {
    if let Err(reason) = me.require_session() {
        return Standing::Foreign(reason);
    }
    match standing(lock_reason(seat_path), me, held) {
        Standing::Ours => Standing::Ours,
        Standing::Foreign(reason) => Standing::Foreign(reason),
        // Two runners can meet one dead claim at once; git's lock refuses
        // the second.
        Standing::Stale(_) => {
            unlock_seat(cwd, seat_path);
            lock_or_read(cwd, seat_path, me, held)
        }
        Standing::Free => lock_or_read(cwd, seat_path, me, held),
    }
}

/// One atomic claim, and what the seat looked like afterwards.
/// `git worktree lock` refuses a second lock, so the loser of a race is
/// told here.
fn lock_or_read(cwd: &str, seat_path: &str, me: &Identity, held: Held) -> Standing {
    let mut command = std::process::Command::new("git");
    command
        .arg("-C")
        .arg(cwd)
        // The refusal below is read out of git's message, and a
        // translated one would read as no refusal at all.
        .env("LC_ALL", "C")
        .args(["worktree", "lock", "--reason", &me.reason(), seat_path]);
    if let Ok(output) = crate::subprocess::run_captured(&mut command)
        && !output.status.success()
        && let Some(rest) = String::from_utf8_lossy(&output.stderr)
            .split("already locked")
            .nth(1)
    {
        let reason = rest
            .split_once("reason:")
            .map(|(_, reason)| reason.trim().to_string())
            .unwrap_or_default();
        return standing(Some(reason), me, held);
    }
    // Not a claim until the seat, read back, names this session.
    standing(lock_reason(seat_path), me, held)
}

/// Lifts whatever lock a seat carries. Callers check first whose it is.
pub(crate) fn unlock_seat(cwd: &str, seat_path: &str) -> bool {
    crate::subprocess::git_query(cwd, &["worktree", "unlock", seat_path]).is_some()
}

/// The reason on this worktree's own lock, if it is locked at all.
pub(crate) fn lock_reason(cwd: &str) -> Option<String> {
    let git_dir =
        crate::subprocess::git_query(cwd, &["rev-parse", "--path-format=absolute", "--git-dir"])?;
    let reason = std::fs::read_to_string(format!("{git_dir}/locked")).ok()?;
    Some(reason.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::{Held, Identity, Standing};

    fn me(session: &str, pid: Option<u32>) -> Identity {
        Identity {
            session: session.to_string(),
            pid,
        }
    }

    /// Every claim these tests stand is a seat's unless it says otherwise.
    fn stood(reason: &str, me: &Identity) -> Standing {
        super::standing(Some(reason.to_string()), me, Held::BySession)
    }

    /// A pid that names no process: a reaped child's can be handed out
    /// again before the assertion runs.
    fn dead_pid() -> u32 {
        crate::subprocess::NO_SUCH_PID
    }

    #[test]
    fn an_anonymous_session_cannot_start_a_claim() {
        for session in ["", "   "] {
            let anonymous = me(session, None);
            let error = crate::seats::assign("not-a-repository", &anonymous)
                .err()
                .expect("no identity must fail before probing git");
            assert!(error.contains("session identity"), "{error}");
            let error = crate::seats::release("not-a-repository", &anonymous, None)
                .expect_err("an anonymous release must fail before probing git");
            assert!(error.contains("session identity"), "{error}");
            assert!(matches!(
                super::take_seat("not-a-repository", "no-seat", &anonymous, Held::BySession),
                Standing::Foreign(_)
            ));
        }
    }

    #[test]
    fn session_identity_uses_the_first_nonblank_source() {
        assert_eq!(
            super::session_id(Some("hook"), Some("claude"), Some("codex")),
            "hook"
        );
        assert_eq!(
            super::session_id(Some(" "), Some("claude"), Some("codex")),
            "claude"
        );
        assert_eq!(super::session_id(None, Some(" "), Some("codex")), "codex");
        assert_eq!(super::session_id(None, None, Some("codex")), "codex");
        assert_eq!(super::session_id(None, None, None), "");
    }

    #[test]
    fn a_claim_records_the_session_and_the_process_that_wrote_it() {
        assert_eq!(me("s1", Some(42)).reason(), "claude-seat s1 pid 42");
        assert_eq!(me("s1", None).reason(), "claude-seat s1");
        assert_eq!(me("s1", Some(42)).mark(), "session s1 (pid 42)");
        let read = super::holder("claude-seat s1 pid 42").expect("the claim carries our mark");
        assert_eq!(read.session, "s1");
        assert_eq!(read.pid, Some(42));
        assert!(super::is_a_seat_claim("claude-seat s1 pid 42"));
        assert!(!super::is_a_seat_claim("parked by hand"));
        // The mark ends at a word (`holder`).
        assert!(!super::is_a_seat_claim("claude-seats are mine"));
        assert_eq!(
            super::whose("claude-seats are mine"),
            "locked by hand (claude-seats are mine)"
        );
        // Written with no id (`whose`).
        assert!(super::is_a_seat_claim("claude-seat"));
        assert_eq!(
            super::whose("claude-seat"),
            "held by a session that gave no id of its own"
        );
        assert_eq!(
            super::whose("claude-seat s1 pid 42"),
            "held by session s1 (pid 42)"
        );
        assert_eq!(
            super::whose("parked by hand"),
            "locked by hand (parked by hand)"
        );
    }

    /// Older builds wrote the process's start and program after the pid.
    #[test]
    fn marks_an_older_build_wrote_after_the_number_are_passed_over() {
        let read = super::holder("claude-seat s1 pid 42 born 77 as My Claude.exe")
            .expect("the claim carries our mark");
        assert_eq!(read.session, "s1");
        assert_eq!(read.pid, Some(42));
        let read = super::holder("claude-seat s1 pid 42 as claude.exe").expect("our mark");
        assert_eq!(read.pid, Some(42));
        // A claim written before the pid was recorded at all, and one a
        // caller wrote with no identity to put in it.
        let read = super::holder("claude-seat s1").expect("our mark");
        assert_eq!(read.pid, None);
        let read = super::holder("claude-seat").expect("our mark");
        assert_eq!(read.session, "");
        assert_eq!(super::holder("parked by hand").map(|h| h.session), None);
    }

    #[test]
    fn stands_a_lock_relative_to_the_session() {
        let mine = me("s1", Some(std::process::id()));
        assert!(matches!(
            super::standing(None, &mine, Held::BySession),
            Standing::Free
        ));
        assert!(matches!(stood("claude-seat s1", &mine), Standing::Ours));
        assert!(matches!(
            stood("claude-seat s1 pid 7", &mine),
            Standing::Ours
        ));
        assert!(matches!(
            stood("claude-seat s2 pid 7", &mine),
            Standing::Foreign(_)
        ));
        assert!(matches!(
            stood("parked by hand", &mine),
            Standing::Foreign(_)
        ));
        // A session with no marks reads every claim as somebody else's,
        // and an anonymous claim is nobody's, not everybody's.
        assert!(matches!(
            stood("claude-seat s2", &me("", None)),
            Standing::Foreign(_)
        ));
        assert!(matches!(
            stood("claude-seat", &me("", None)),
            Standing::Foreign(_)
        ));
    }

    /// The app restarts under a conversation, and the conversation comes
    /// back to its letter.
    #[test]
    fn a_seat_claim_stands_whatever_became_of_its_process() {
        let mine = me("s1", Some(std::process::id()));
        let gone = format!("claude-seat s2 pid {}", dead_pid());
        assert!(matches!(stood(&gone, &mine), Standing::Foreign(_)));
        let gone_with_marks = format!("claude-seat s2 pid {} born 1 as claude.exe", dead_pid());
        assert!(matches!(
            stood(&gone_with_marks, &mine),
            Standing::Foreign(_)
        ));
        // The same conversation, back from a restart under a new number:
        // its own seat still.
        let back = me("s2", Some(std::process::id()));
        assert!(matches!(stood(&gone, &back), Standing::Ours));
    }

    /// The one claim a pid settles.
    #[test]
    fn a_rig_claim_whose_runner_is_gone_is_litter() {
        let mine = me("mine", None);
        let gone = format!("claude-seat perf-1 pid {}", dead_pid());
        assert!(matches!(
            super::standing(Some(gone.clone()), &mine, Held::ByRunner),
            Standing::Stale(_)
        ));
        // This process is the runner's own test binary: a claim on its
        // number is a live runner's, whatever session it names.
        let live = format!("claude-seat perf-2 pid {}", std::process::id());
        assert!(matches!(
            super::standing(Some(live), &mine, Held::ByRunner),
            Standing::Foreign(_)
        ));
        // A rig claim with no number cannot be asked, and stays.
        assert!(matches!(
            super::standing(Some("claude-seat perf-3".into()), &mine, Held::ByRunner),
            Standing::Foreign(_)
        ));
    }
}
