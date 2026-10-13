//! Whose a seat is: the marks a session writes into `git worktree
//! lock`'s reason, and what they settle. Apart from the roster, which
//! measures letters: this is about sessions, and is also read from
//! outside the roster (the entry hooks, `land`, the perf rig).
//!
//! A seat's claim is a conversation's, and no process is asked about it:
//! the Claude process ends whenever the app restarts while the
//! conversation goes on, so a claim judged by its process would hand the
//! letter to a stranger. Nor does the session id last as long as the
//! conversation in the desktop app: a prompt edited after an interrupt
//! goes on under a new session id in the same app session, so a claim
//! names the app session too, and either one settles it
//! (`Identity::host`). A `/clear` begins a new session id in the same app
//! session and the same tree as well; the claim stays the app session's,
//! and what the tree carries is said when the roster hands it back
//! (`seats::held_seat`). It moves three ways only — the branch lands
//! (`land::release_claim`), the session hands it back (`seats::release`),
//! or the user has it taken over (`seats::takeover`). The rig's claim is
//! the one that is a process's (`Held::ByRunner`).

use std::io::Write;

/// The mark a seat claim carries in `git worktree lock`'s reason:
/// `claude-seat <session> pid <pid> host <app session>`, the pid and the
/// host each where the environment names one. For a seat the pid only
/// names the tab that wrote it (`standing`); any other word after the
/// session is ignored (older builds wrote more). A lock without this mark
/// is a person's, and nothing automatic touches it.
pub(crate) const SEAT_CLAIM: &str = "claude-seat";

/// The environment's name for the desktop app's session.
const HOST_SESSION: &str = "CLAUDE_CODE_HOST_SESSION_ID";

/// This session, as a claim records it.
pub(crate) struct Identity {
    /// The conversation's id with Claude: a claim is matched on it, and
    /// it is the name of the transcript the work is in.
    pub session: String,
    /// The Claude process, when the environment names one — for the rig,
    /// the claim's whole liveness.
    pub pid: Option<u32>,
    /// The desktop app's session (`CLAUDE_CODE_HOST_SESSION_ID`): the
    /// conversation as the app lists it, which keeps this id while the
    /// session id changes under it. None outside the desktop app, and for
    /// the rig.
    pub host: Option<String>,
}

impl Identity {
    /// This session's marks: the hook payload's session id, else the
    /// environment's (Claude's, then Codex's task id); the pid is
    /// CLAUDE_PID's alone, and the host the desktop app's — for a Claude
    /// session only: a Codex task is a conversation of its own, whatever
    /// the shell it was started from carried.
    pub(crate) fn current(session: Option<&str>) -> Self {
        let codex = std::env::var("CODEX_THREAD_ID").ok();
        let session = session_id(
            session,
            std::env::var("CLAUDE_CODE_SESSION_ID").ok().as_deref(),
            codex.as_deref(),
        );
        let pid = std::env::var("CLAUDE_PID")
            .ok()
            .and_then(|pid| pid.trim().parse().ok());
        let host = app_session(
            std::env::var(HOST_SESSION).ok().as_deref(),
            codex.as_deref(),
            &session,
        );
        Self { session, pid, host }
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
        let mut reason = format!("{SEAT_CLAIM} {}", self.session);
        if let Some(pid) = self.pid {
            reason.push_str(&format!(" pid {pid}"));
        }
        if let Some(host) = &self.host {
            reason.push_str(&format!(" host {host}"));
        }
        reason
    }

    /// Whether `holder` is this conversation: the same session id, or the
    /// same app session under another one.
    fn is(&self, holder: &Identity) -> bool {
        (!self.session.is_empty() && holder.session == self.session)
            || self.host.is_some() && holder.host == self.host
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

/// The app session `session` belongs to: none for a Codex task, which
/// names itself by its task id.
fn app_session(host: Option<&str>, codex: Option<&str>, session: &str) -> Option<String> {
    if codex.map(str::trim) == Some(session) {
        return None;
    }
    host.and_then(one_word)
}

/// `text` as one word of a lock's reason, which is read back word by
/// word; None for anything else.
fn one_word(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'))
    .then(|| text.to_string())
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

/// Who a claim names, read back out of a lock's reason; None for a
/// person's lock. The mark ends at a word: `claude-seats are mine` is a
/// person's sentence, and reading it as a claim would hand their lock to
/// the first takeover that asked.
fn holder(reason: &str) -> Option<Identity> {
    let rest = reason.strip_prefix(SEAT_CLAIM)?;
    if !rest.is_empty() && !rest.starts_with(' ') {
        return None;
    }
    let mut words = rest.split_whitespace().peekable();
    // A claim written with no id goes straight on to its marks.
    let session = match words.peek() {
        Some(&word) if word != "pid" && word != "host" => words.next(),
        _ => None,
    };
    let (mut pid, mut host) = (None, None);
    while let Some(word) = words.next() {
        match word {
            "pid" => pid = words.next().and_then(|pid| pid.parse().ok()),
            "host" => host = words.next().and_then(one_word),
            _ => {}
        }
    }
    Some(Identity {
        session: session.unwrap_or_default().to_string(),
        pid,
        host,
    })
}

/// Whether a claim this session's conversation holds names an earlier
/// session id of it (`Identity::host`).
pub(crate) fn carried_over(reason: &str, me: &Identity) -> bool {
    holder(reason).is_some_and(|holder| me.is(&holder) && holder.session != me.session)
}

/// Whether a claim of this session's conversation lacks a name this
/// session would write: its session id, or its host — a claim written
/// before claims named one, which a later id of the conversation could
/// not find.
fn wants_renaming(reason: &str, me: &Identity) -> bool {
    holder(reason).is_some_and(|holder| {
        holder.session != me.session || (holder.host.is_none() && me.host.is_some())
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
    if me.is(&holder) {
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
/// stale rig claim lifted and replaced, anybody else's left; a claim of
/// this conversation's under an earlier session id is renamed for this
/// one. Only `Ours` means the session may work there.
pub(crate) fn take_seat(cwd: &str, seat_path: &str, me: &Identity, held: Held) -> Standing {
    if let Err(reason) = me.require_session() {
        return Standing::Foreign(reason);
    }
    let reason = lock_reason(seat_path);
    match standing(reason.clone(), me, held) {
        Standing::Ours => {
            if let Some(reason) = reason.filter(|reason| wants_renaming(reason, me)) {
                rename(seat_path, &reason, me);
            }
            Standing::Ours
        }
        Standing::Foreign(reason) => Standing::Foreign(reason),
        // Two runners can meet one dead claim at once; the claim's
        // creation refuses the second (`lock_or_read`).
        Standing::Stale(_) => {
            unlock_seat(cwd, seat_path);
            lock_or_read(seat_path, me, held)
        }
        Standing::Free => lock_or_read(seat_path, me, held),
    }
}

/// One claim, and what the seat looked like afterwards. The lock is git's
/// own file with git's own content (`git worktree lock --reason`), but
/// created only where none stands: git reads whether a lock stands and
/// then writes one, so two sessions asking at once can both pass the read
/// and both read their own claim back. Of two creations, one fails.
fn lock_or_read(seat_path: &str, me: &Identity, held: Held) -> Standing {
    if let Some(lock) = lock_file(seat_path)
        && let Ok(mut file) = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock)
        && file
            .write_all(format!("{}\n", me.reason()).as_bytes())
            .is_err()
    {
        // An empty lock would read as a person's, which nothing lifts.
        let _ = std::fs::remove_file(&lock);
    }
    // Not a claim until the seat, read back, names this session.
    standing(lock_reason(seat_path), me, held)
}

/// Writes this session's claim over `judged`, which names an earlier
/// session id of its conversation or no host, so the lock names the
/// transcript the work goes on in. Swapped in whole: an unlock and a lock
/// would leave a moment in which a stranger could take the seat, work and
/// all, and a write in place a moment in which it reads empty — a
/// person's lock. A lock that no longer says `judged` is left as it is;
/// one left unwritten is still this conversation's, by its host.
fn rename(seat_path: &str, judged: &str, me: &Identity) {
    let Some(lock) = lock_file(seat_path) else {
        return;
    };
    if !std::fs::read_to_string(&lock).is_ok_and(|reason| reason.trim() == judged) {
        return;
    }
    let renamed = lock.with_extension(format!("renamed-{}", std::process::id()));
    if std::fs::write(&renamed, format!("{}\n", me.reason())).is_err()
        || std::fs::rename(&renamed, &lock).is_err()
    {
        let _ = std::fs::remove_file(&renamed);
    }
}

/// Lifts whatever lock a seat carries. Callers check first whose it is.
pub(crate) fn unlock_seat(cwd: &str, seat_path: &str) -> bool {
    crate::subprocess::git_query(cwd, &["worktree", "unlock", seat_path]).is_some()
}

/// The reason on this worktree's own lock, if it is locked at all.
pub(crate) fn lock_reason(cwd: &str) -> Option<String> {
    let reason = std::fs::read_to_string(lock_file(cwd)?).ok()?;
    Some(reason.trim().to_string())
}

/// Where git keeps this worktree's lock: `locked` in its admin directory,
/// there or not.
fn lock_file(cwd: &str) -> Option<std::path::PathBuf> {
    let git_dir =
        crate::subprocess::git_query(cwd, &["rev-parse", "--path-format=absolute", "--git-dir"])?;
    Some(std::path::Path::new(&git_dir).join("locked"))
}

#[cfg(test)]
mod tests {
    use super::{Held, Identity, Standing};

    fn me(session: &str, pid: Option<u32>) -> Identity {
        Identity {
            session: session.to_string(),
            pid,
            host: None,
        }
    }

    /// A session of the desktop app's session `host`.
    fn in_app(session: &str, host: &str) -> Identity {
        Identity {
            session: session.to_string(),
            pid: Some(42),
            host: Some(host.to_string()),
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

    #[test]
    fn a_claim_names_the_app_session_and_reads_it_back() {
        let reason = in_app("s1", "local_a-1").reason();
        assert_eq!(reason, "claude-seat s1 pid 42 host local_a-1");
        let read = super::holder(&reason).expect("our mark");
        assert_eq!(read.session, "s1");
        assert_eq!(read.pid, Some(42));
        assert_eq!(read.host.as_deref(), Some("local_a-1"));
        let read = super::holder("claude-seat s1 host local_a").expect("our mark");
        assert_eq!((read.pid, read.host.as_deref()), (None, Some("local_a")));
        // No id of its own, and the host is still read.
        let read = super::holder("claude-seat host local_a").expect("our mark");
        assert_eq!(read.session, "");
        assert_eq!(read.host.as_deref(), Some("local_a"));
        assert_eq!(super::one_word(" local_a "), Some("local_a".into()));
        assert_eq!(super::one_word("two words"), None);
        assert_eq!(super::one_word(""), None);
    }

    #[test]
    fn a_codex_task_belongs_to_no_app_session() {
        let app = |host, codex, session| super::app_session(host, codex, session);
        assert_eq!(app(Some("local_a"), None, "s1"), Some("local_a".into()));
        assert_eq!(
            app(Some("local_a"), Some("t1"), "s1"),
            Some("local_a".into())
        );
        assert_eq!(
            app(Some("local_a"), Some("t1"), "t1"),
            None,
            "a task named by its own id, in a shell a Claude session opened"
        );
        assert_eq!(app(Some("two words"), None, "s1"), None);
        assert_eq!(app(None, None, "s1"), None);
    }

    /// The desktop app goes on with a conversation under a new session id
    /// when a prompt is edited after an interrupt, in the same app session.
    #[test]
    fn a_claim_is_its_app_sessions_under_any_session_id() {
        let claim = in_app("before", "local_a").reason();
        let after = in_app("after", "local_a");
        assert!(matches!(stood(&claim, &after), Standing::Ours));
        assert!(super::carried_over(&claim, &after));
        assert!(super::wants_renaming(&claim, &after));
        assert!(!super::carried_over(&claim, &in_app("before", "local_a")));
        assert!(!super::wants_renaming(&claim, &in_app("before", "local_a")));

        // Another app session — a session started from this one included.
        assert!(matches!(
            stood(&claim, &in_app("after", "local_b")),
            Standing::Foreign(_)
        ));
        // A session outside the app, and a claim that names no host, are
        // matched on the session id alone.
        assert!(matches!(
            stood(&claim, &me("after", None)),
            Standing::Foreign(_)
        ));
        assert!(matches!(
            stood("claude-seat before pid 7", &after),
            Standing::Foreign(_)
        ));
        // Written before claims named their host: its own session names it
        // again, so a later id of the conversation finds it.
        let unnamed = "claude-seat before pid 7";
        assert!(matches!(
            stood(unnamed, &in_app("before", "local_a")),
            Standing::Ours
        ));
        assert!(super::wants_renaming(unnamed, &in_app("before", "local_a")));
        assert!(!super::wants_renaming(unnamed, &me("before", None)));
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
