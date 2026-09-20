//! Whose a seat is: the marks a session writes into `git worktree
//! lock`'s reason, and what those marks settle for whoever meets one.
//!
//! Apart from the roster because the two answer different questions. The
//! roster measures seats — what a letter's tree carries, and which letter
//! is free. This decides whose a lock is, which is a question about
//! sessions and nothing about letters at all; it is also the half read
//! from outside the roster (the entry hooks, `land`, the perf rig).
//!
//! A seat's claim is a conversation's, and no process is asked about
//! it: the Claude process behind a conversation ends whenever the app
//! restarts, and the conversation goes on without it, so a claim judged
//! by its process is lifted from under a session that is merely between
//! processes — its letter handed to a stranger and its pictures swept
//! off the board. A seat claim moves three ways and no other: the branch
//! lands (`land::release_claim`), the session hands the letter back
//! (`seats::release`), or the user has another session take it over
//! (`seats::takeover`). The rig's claim is the one that is a process's —
//! this program's own — and it is asked after as such
//! (`Held::ByRunner`).

/// The mark a seat claim carries in `git worktree lock`'s reason,
/// followed by the session id and the Claude process the claim was
/// written from: `claude-seat <session> pid <pid>`. The pid is for the
/// reader — it names the tab that wrote the claim — and settles nothing
/// (`standing`). Anything after it is ignored, so a claim an older build
/// wrote with more marks reads the same. A lock without this mark is a
/// person's, and nothing automatic touches it.
pub(crate) const SEAT_CLAIM: &str = "claude-seat";

/// This session, as a claim records it.
pub(crate) struct Identity {
    /// The conversation. This is what a claim is matched on, and the one
    /// mark a session keeps across every process it is run from.
    pub session: String,
    /// The Claude process the claim was written from, when the
    /// environment names one. For a seat it only tells a reader which
    /// tab; for the rig it is the claim's whole liveness.
    pub pid: Option<u32>,
}

impl Identity {
    /// This session's marks: the session id a hook payload carries, or
    /// the environment's for a command run outside one, and the Claude
    /// process both run under. Codex's task id supplies the session when
    /// no Claude identity is present; the pid is CLAUDE_PID's alone.
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
    /// — the conversation outlives its processes — so a seat claim is
    /// somebody's until it is landed, released or taken over.
    BySession,
    /// By this task runner, out of its own pid: the perf rig's claim,
    /// which no session writes and the roster never hands out
    /// (`perf::rig`). A measurement is one process, so a claim whose
    /// runner is gone is litter the next measurement may clear.
    ByRunner,
}

/// A claim as a person reads it: whose it is, by the marks it carries.
pub(crate) fn whose(reason: &str) -> String {
    match holder(reason) {
        // A claim written by a session that had no id to give: nobody's
        // by name, and nobody's to match, but ours to lift all the same
        // (`is_a_seat_claim`). A reader told "held by session " would go
        // looking for a session called nothing.
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

/// Who a claim names, read back out of a lock's reason. None when the
/// lock carries no claim of ours — a person's lock, which nothing
/// automatic touches.
///
/// Only the session and the number are read; whatever an older build
/// wrote after the number is passed over, so every claim on a machine
/// reads under one rule whichever build wrote it.
///
/// The mark ends at a word: `claude-seats are mine` is a sentence a
/// person wrote, and reading it as a claim would hand their lock to the
/// first takeover that asked for the letter.
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
    /// The rig's claim with its runner gone: litter the next measurement
    /// may clear. A seat claim is never this (`standing`).
    Stale(String),
    /// Somebody else's claim, or a lock a person wrote by hand.
    Foreign(String),
}

/// Where a claim stands for this session, and nothing it cannot account
/// for is read as its own: a session that assumes an unaccountable lock
/// is its ends up sharing the tree with whoever wrote it. `held` says
/// whether the claim's number may be asked after when the claim is not
/// this session's — the rig's may, a seat's never.
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

/// Takes `seat_path` for this session when it is there to take: a free
/// seat is locked, a rig claim whose runner is gone is lifted and locked
/// again, and a claim somebody else holds is left where it is. Answers
/// where the seat stood once this was done — only `Ours` means the
/// session may work there.
pub(crate) fn take_seat(cwd: &str, seat_path: &str, me: &Identity, held: Held) -> Standing {
    if let Err(reason) = me.require_session() {
        return Standing::Foreign(reason);
    }
    match standing(lock_reason(seat_path), me, held) {
        Standing::Ours => Standing::Ours,
        Standing::Foreign(reason) => Standing::Foreign(reason),
        // Two runners can meet one dead claim in the same moment, and
        // the lock below is what decides between them: git refuses
        // the second one.
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
        // The refusal below is read out of git's own message, and a
        // translated one would read as no refusal at all. Pin the locale
        // so the claim keeps its teeth.
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
    // A lock git reported nothing about is not a claim yet: read the seat
    // back, so that only a reason naming this session counts as one.
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

    /// A pid that certainly names no process — a reaped child's is
    /// the kernel's to give out again before the assertion runs
    /// (`subprocess::NO_SUCH_PID`).
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
        // The mark ends at a word: a sentence that merely starts with it
        // is a person's lock, and lifting one is nobody's business.
        assert!(!super::is_a_seat_claim("claude-seats are mine"));
        assert_eq!(
            super::whose("claude-seats are mine"),
            "locked by hand (claude-seats are mine)"
        );
        // A session that had no id to give wrote this one. It is ours to
        // lift, and naming it "session " would send a reader looking.
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

    /// Claims older builds wrote carry marks after the number — when the
    /// process began, and the program it was. They read under this
    /// build's rule like any other, with those marks passed over.
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
        // A session with no marks of its own reads every claim as
        // somebody else's: assuming one its own put two sessions in seat
        // e. And an anonymous claim is nobody's, not everybody's.
        assert!(matches!(
            stood("claude-seat s2", &me("", None)),
            Standing::Foreign(_)
        ));
        assert!(matches!(
            stood("claude-seat", &me("", None)),
            Standing::Foreign(_)
        ));
    }

    /// The rule the whole roster now hangs on: a seat claim whose
    /// process is gone is still its session's. The app restarts under a
    /// conversation, and the conversation comes back to its letter.
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

    /// The rig's claim is this program's own, and a runner that is gone
    /// leaves litter: the one claim a number settles.
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
