//! Whose a seat is: the marks a session writes into `git worktree
//! lock`'s reason, and what those marks settle for whoever meets one.
//!
//! Apart from the roster because the two answer different questions. The
//! roster measures seats — what a letter's tree carries, and which letter
//! is free. This decides whose a lock is, which is a question about
//! processes and conversations and nothing about letters at all; it is
//! also the half read from outside the roster (the entry hooks, `land`,
//! the perf rig).

/// The mark a session's seat claim carries in `git worktree lock`'s
/// reason, followed by the session id, the Claude process the claim was
/// written from, when that process began and the program it was:
/// `claude-seat <session> pid <pid> born <when> as <image>`. The image
/// stays last because it is the only part that may hold a space
/// (`holder`). The entry hooks and the post-write re-claim
/// write it. Three things take it off, and none of them guesses: landing the
/// seat's branch (`land::release_claim`), `cargo xtask seat release`,
/// and the roster meeting a claim whose Claude process is gone
/// (`claim_is_dead`). A lock without this mark is a person's, and
/// nothing automatic touches it.
pub(crate) const SEAT_CLAIM: &str = "claude-seat";
/// This session, as a claim records it.
///
/// The session id says which conversation holds the seat, and is what
/// the session-end release matches on. The pid is the one mark that can
/// be put a question to: a claim whose Claude process is gone is litter,
/// and without asking, a still seat can only be guessed at from how long
/// it has been still — a guess that unlocks a live session's seat out
/// from under it. A number is not a name, though; the machine hands it
/// back out the moment its process ends, so the two marks recorded
/// beside it are what make it answer for one process, whoever holds
/// the number next.
pub(crate) struct Identity {
    pub session: String,
    pub pid: Option<u32>,
    /// The program `pid` was when the claim was written, as this machine
    /// spells it. Filled only when a claim is read back (`holder`): a
    /// session describing itself reads its own marks out of the
    /// environment, which says nothing about how the machine names the
    /// process, and the probe that answers for that runs where the claim
    /// is written (`Identity::reason`).
    pub image: Option<String>,
    /// When `pid`'s process began. The program name tells a git or a
    /// browser tab that inherited the number, but not the next Claude
    /// session handed it — every session on this machine is one
    /// `claude.exe`, so both read as the same program. This is the mark
    /// that tells those apart. Filled on the way back in, as `image` is.
    pub born: Option<String>,
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
        Self {
            session,
            pid,
            image: None,
            born: None,
        }
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
    ///
    /// A session's claim records the program behind its pid and when that
    /// process began, because a number alone is handed back out the moment
    /// its process is gone and how a session is installed or spelled is the
    /// machine's to say — an image name in this source is a guess that
    /// hands a live session's seat away the day it is wrong. Both marks go
    /// in because neither answers alone: the program tells a stranger of
    /// another kind that inherited the number, and the beginning tells the
    /// next `claude.exe`, which is what a session's number is usually
    /// handed to on a machine running several. The runner's own claims
    /// record nothing extra: `Held::ByRunner` asks after this very
    /// program, which it may name outright.
    ///
    /// Probed here because [`Identity::current`] runs on every hook
    /// event, and this runs once per claim taken.
    pub(crate) fn reason(&self, held: Held) -> String {
        let Some(pid) = self.pid else {
            return format!("{SEAT_CLAIM} {}", self.session);
        };
        let stem = format!("{SEAT_CLAIM} {} pid {pid}", self.session);
        match held {
            Held::ByRunner => stem,
            Held::BySession => {
                let born = crate::subprocess::born_of(pid)
                    .map(|born| format!(" born {born}"))
                    .unwrap_or_default();
                let image = crate::subprocess::image_of(pid)
                    .map(|image| format!(" as {image}"))
                    .unwrap_or_default();
                format!("{stem}{born}{image}")
            }
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

/// What kind of process a claim's pid names, which is the form its
/// liveness has to be asked in: the number alone is handed back out the
/// moment its process is gone, so only the image behind it tells a claim
/// somebody is working from litter a killed one left.
///
/// Two kinds write claims through this module, and a reader that asks in
/// the wrong one is wrong both ways — it breaks a live claim, or it keeps
/// a dead one — so every reader names which it expects.
#[derive(Clone, Copy)]
pub(crate) enum Held {
    /// By a Claude session, out of its `CLAUDE_PID`: every seat claim.
    /// What program that is, only the claim says.
    BySession,
    /// By this task runner, out of its own pid: the perf rig's claim,
    /// which no session writes and the roster never hands out
    /// (`perf::rig`).
    ByRunner,
}

impl Held {
    /// Whether the process a claim of this kind named is still there.
    /// `wrote` is what the claim recorded about it, and a session's is
    /// asked against that in the order the marks can answer: when the
    /// process began settles it outright, the program it was answers for
    /// the claims written before that mark existed, and a claim with
    /// neither can only be asked whether anybody at all is at the number
    /// — which answers alive for whoever inherited it. That is the safe
    /// way to be wrong, and it lasts until the seat is claimed again.
    fn still_running(self, pid: u32, wrote: &Identity) -> bool {
        if matches!(self, Self::ByRunner) {
            return crate::subprocess::task_runner_exists(pid);
        }
        let asked = wrote
            .born
            .as_deref()
            .and_then(|born| crate::subprocess::born_still_at(pid, born));
        match (asked, wrote.image.as_deref()) {
            (Some(answer), _) => answer,
            (None, Some(image)) => crate::subprocess::image_still_at(pid, image),
            (None, None) => crate::subprocess::process_exists(pid),
        }
    }
}

/// What a claim's own marks settle for whoever meets it in somebody
/// else's seat. A session cannot tell a real collision from a stale lock
/// by when the lock was written — seat e was shared for eight minutes
/// because a session read a matching mtime as proof the claim was its
/// own — so the answer is the claim's process, asked. Seat claims only,
/// whose pid is a session's; the rig says its own piece (`perf::rig`).
pub(crate) fn claim_liveness(reason: &str) -> &'static str {
    let Some(holder) = holder(reason) else {
        return "That lock carries no claim of ours, so a person wrote it.";
    };
    let names_its_process = holder.born.is_some() || holder.image.is_some();
    match holder.pid {
        Some(pid) if Held::BySession.still_running(pid, &holder) => {
            if names_its_process {
                "That session is running, so the other session is live and both of you are in one tree."
            } else {
                "A process holds that number, but the claim does not say which program it \
                 was — so it cannot be told from a stranger that inherited the number."
            }
        }
        Some(_) => {
            "No session is running under that number, so the claim is litter one left behind."
        }
        None => {
            "The claim names no process, so it predates the mark that would answer — \
             `cargo xtask seats` says whether the seat is still being worked."
        }
    }
}

/// Whether a lock is a claim nobody is behind any more. Only a claim that
/// names its process can answer, and only one that says when that process
/// began can tell every stranger that inherited the number from the
/// session that wrote it; anything else is left standing. Seat claims
/// only, as [`claim_liveness`] is.
pub(crate) fn claim_is_dead(reason: &str) -> bool {
    holder(reason).is_some_and(|holder| {
        holder
            .pid
            .is_some_and(|pid| !Held::BySession.still_running(pid, &holder))
    })
}

/// Who a claim names, read back out of a lock's reason. None when the
/// lock carries no claim of ours — a person's lock, which nothing
/// automatic touches.
///
/// Every part after the mark is optional, because a claim an older build
/// wrote has fewer of them, and a claim this build writes is read by
/// those older builds too. The image stays last and every mark added
/// after it goes in front of it, because the image is the one part that
/// may hold a space: a reader that stopped at the first space would read
/// a program this machine has no process for and call a live session
/// litter. What an older build loses instead is the pid — where it looks
/// for a number it now finds the marks it does not know — and a claim
/// whose pid cannot be read is one it leaves standing, which is the
/// harmless way to lose it.
fn holder(reason: &str) -> Option<Identity> {
    let rest = reason.strip_prefix(SEAT_CLAIM)?.trim_start();
    let Some((session, rest)) = rest.split_once(" pid ") else {
        return Some(Identity {
            session: rest.trim().to_string(),
            pid: None,
            image: None,
            born: None,
        });
    };
    // The image is cut off the end first, being the only part that may
    // hold a space; what is left is the marks, which may not.
    let (marks, image) = match rest.split_once(" as ") {
        Some((marks, image)) => (
            marks,
            Some(image.trim().to_string()).filter(|i| !i.is_empty()),
        ),
        None => (rest, None),
    };
    let (pid, born) = match marks.split_once(" born ") {
        Some((pid, born)) => (pid, Some(born.trim().to_string()).filter(|b| !b.is_empty())),
        None => (marks, None),
    };
    Some(Identity {
        session: session.trim().to_string(),
        pid: pid.trim().parse().ok(),
        image,
        born,
    })
}

/// Where a seat's lock stands relative to this session. Pure but for the
/// liveness probe, so the tests can ask.
pub(crate) enum Standing {
    /// No lock at all.
    Free,
    /// This session's claim.
    Ours,
    /// A claim the process it names is no longer behind: litter whoever
    /// meets it may clear.
    Stale(String),
    /// Somebody else's live claim, or a lock a person wrote by hand.
    Foreign(String),
}

/// Where a claim stands for this session, and nothing it cannot account
/// for is read as its own: a session that assumes an unaccountable lock
/// is its ends up sharing the tree with whoever wrote it. `held` is what
/// the claim's pid is asked after when the claim is not this session's.
pub(crate) fn standing(reason: Option<String>, me: &Identity, held: Held) -> Standing {
    let Some(reason) = reason else {
        return Standing::Free;
    };
    let Some(holder) = holder(&reason) else {
        return Standing::Foreign(reason);
    };
    if wrote_it(&holder, me) {
        return Standing::Ours;
    }
    match holder.pid {
        Some(pid) if !held.still_running(pid, &holder) => Standing::Stale(reason),
        _ => Standing::Foreign(reason),
    }
}

/// Whether `me` is the session that wrote `holder`: the session id when
/// there is one on both sides, and otherwise the number, but only from a
/// claim that says which process was behind it.
///
/// The number alone was proof once, and is not. It is handed back out the
/// moment its process ends, and on this machine it goes to another
/// `claude.exe` — so a claim a dead session left on this session's number
/// reads as this session's own, and the program name cannot say
/// otherwise, both sides of it being one program. A claim that predates
/// the mark is not this session's on the strength of a number: refusing
/// it costs the session another `cargo xtask seat`, and granting it puts
/// two sessions in one tree.
fn wrote_it(holder: &Identity, me: &Identity) -> bool {
    if !me.session.is_empty() && holder.session == me.session {
        return true;
    }
    match (me.pid, holder.born.as_deref()) {
        (Some(pid), Some(born)) if holder.pid == me.pid => {
            crate::subprocess::born_still_at(pid, born) == Some(true)
        }
        _ => false,
    }
}

/// Takes `seat_path` for this session when it is there to take: a free
/// seat is locked, a claim whose process is gone is lifted and locked
/// again, and a live claim somebody else holds is left where it is.
/// Answers where the seat stood once this was done — only `Ours` means
/// the session may work there.
pub(crate) fn take_seat(cwd: &str, seat_path: &str, me: &Identity, held: Held) -> Standing {
    if let Err(reason) = me.require_session() {
        return Standing::Foreign(reason);
    }
    match standing(lock_reason(seat_path), me, held) {
        Standing::Ours => Standing::Ours,
        Standing::Foreign(reason) => Standing::Foreign(reason),
        // Two sessions can meet one dead claim in the same moment, and
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
        .args(["worktree", "lock", "--reason", &me.reason(held), seat_path]);
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
    use super::{Identity, Standing};

    /// A session's marks, as a claim would record them. Nothing about the
    /// process: what a session knows of itself never carries it, and the
    /// claim it writes gets it from the machine (`Identity::reason`).
    fn me(session: &str, pid: Option<u32>) -> Identity {
        Identity {
            session: session.to_string(),
            pid,
            image: None,
            born: None,
        }
    }

    /// Every claim these tests stand is a seat's, and a seat's pid is a
    /// session's (`Held::BySession`).
    fn stood(reason: Option<String>, me: &Identity) -> Standing {
        super::standing(reason, me, super::Held::BySession)
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
            let error = crate::seats::release("not-a-repository", &anonymous)
                .expect_err("an anonymous release must fail before probing git");
            assert!(error.contains("session identity"), "{error}");
            assert!(matches!(
                super::take_seat(
                    "not-a-repository",
                    "no-seat",
                    &anonymous,
                    super::Held::BySession
                ),
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
    fn stands_a_lock_relative_to_the_session() {
        let mine = me("s1", Some(std::process::id()));
        assert!(matches!(stood(None, &mine), Standing::Free));
        // The session id alone still answers for a claim written before
        // the pid was recorded at all.
        assert!(matches!(
            stood(Some("claude-seat s1".into()), &mine),
            Standing::Ours
        ));
        assert!(matches!(
            stood(Some("parked by hand".into()), &mine),
            Standing::Foreign(_)
        ));
    }

    #[test]
    fn a_claim_whose_process_is_gone_is_litter() {
        let mine = me("s1", Some(std::process::id()));
        assert!(matches!(
            stood(Some(format!("claude-seat s2 pid {}", dead_pid())), &mine),
            Standing::Stale(_)
        ));
        // A claim from before the pid was recorded cannot be asked, so it
        // stays somebody's until its session says otherwise.
        assert!(matches!(
            stood(Some("claude-seat s2".into()), &mine),
            Standing::Foreign(_)
        ));
        // A session with no marks of its own reads a claim as somebody
        // else's: assuming it its own put two sessions in seat e.
        assert!(matches!(
            stood(Some("claude-seat s2".into()), &me("", None)),
            Standing::Foreign(_)
        ));
    }

    /// A number outlives the process it named, so the same claim on the
    /// same live number stands or falls by which program is asked after
    /// — and the two kinds ask differently. This process is the runner's
    /// own test binary, so the rig's kind finds what it expects at this
    /// number whatever a claim says; a session's kind believes the claim,
    /// which here names a program this number is not.
    #[test]
    fn a_claim_stands_only_in_the_kind_of_process_that_wrote_it() {
        let reason = format!(
            "claude-seat theirs pid {} as claude.exe",
            std::process::id()
        );
        let mine = me("mine", None);
        assert!(matches!(
            super::standing(Some(reason.clone()), &mine, super::Held::ByRunner),
            Standing::Foreign(_)
        ));
        assert!(matches!(
            super::standing(Some(reason), &mine, super::Held::BySession),
            Standing::Stale(_)
        ));
    }

    /// What a session's claim records about its own process, and what
    /// reading it back settles. Nothing here — and nothing in the source
    /// the claim is written by — knows what a session is installed as:
    /// the claim asks the machine at the moment it is written, and a
    /// reader asks the same machine the same way. A claim that named
    /// some other program is litter however alive its number is, which
    /// is the whole of the pid-reuse guard.
    #[test]
    fn a_session_claim_carries_the_program_its_number_was() {
        let pid = std::process::id();
        let image = crate::subprocess::image_of(pid).expect("this process is behind its own pid");
        let born = crate::subprocess::born_of(pid).expect("this process began at some point");
        let written = me("s1", Some(pid)).reason(super::Held::BySession);
        assert_eq!(
            written,
            format!("claude-seat s1 pid {pid} born {born} as {image}")
        );

        let read = super::holder(&written).expect("the claim carries our mark");
        assert_eq!(read.session, "s1");
        assert_eq!(read.pid, Some(pid));
        assert_eq!(read.image.as_deref(), Some(image.as_str()));
        assert_eq!(read.born.as_deref(), Some(born.as_str()));
        assert!(!super::claim_is_dead(&written));

        let stranger = format!("claude-seat s1 pid {pid} as claude.exe");
        assert!(super::claim_is_dead(&stranger));
        // Somebody else's, and unjudgeable: nothing but the number, so
        // whoever holds it now holds the seat too.
        let numberless = format!("claude-seat s1 pid {pid}");
        assert!(!super::claim_is_dead(&numberless));
    }

    /// The rig's claim keeps the spelling it had. Its number is this very
    /// program's, which `Held::ByRunner` names outright, and the builds
    /// in the other seats — each one its own — still read a pid there.
    #[test]
    fn the_runners_own_claim_records_no_program() {
        let pid = std::process::id();
        let written = me("perf-1", Some(pid)).reason(super::Held::ByRunner);
        assert_eq!(written, format!("claude-seat perf-1 pid {pid}"));
        assert!(matches!(
            super::standing(Some(written), &me("mine", None), super::Held::ByRunner),
            Standing::Foreign(_)
        ));
    }

    /// An image name may hold spaces, and a claim is one line of text: a
    /// reader that stopped at the first space would read a program this
    /// machine has no such process for, and call a live session litter.
    #[test]
    fn a_program_whose_name_holds_spaces_survives_the_round_trip() {
        let read = super::holder("claude-seat s1 pid 42 as My Claude.exe")
            .expect("the claim carries our mark");
        assert_eq!(read.pid, Some(42));
        assert_eq!(read.image.as_deref(), Some("My Claude.exe"));
        assert_eq!(read.born, None);
        // And the marks that may not hold one are read off the front,
        // which is why every mark after the image goes before it.
        let read = super::holder("claude-seat s1 pid 42 born 77 as My Claude.exe")
            .expect("the claim carries our mark");
        assert_eq!(read.pid, Some(42));
        assert_eq!(read.born.as_deref(), Some("77"));
        assert_eq!(read.image.as_deref(), Some("My Claude.exe"));
    }

    /// A number outlives the process it named and goes to the next
    /// `claude.exe`, so the marks a claim keeps have to tell this
    /// session's own claim from the one a dead session left on this
    /// session's number. The program name cannot: both are `claude.exe`.
    /// When the process began can, and a claim that predates the mark
    /// proves nothing by its number — which is what it used to be read
    /// as proving, and what walked a session into an occupied tree.
    #[test]
    fn a_claim_left_on_this_number_by_another_session_is_not_ours() {
        let pid = std::process::id();
        let image = crate::subprocess::image_of(pid).expect("this process is behind its own pid");
        let born = crate::subprocess::born_of(pid).expect("this process began at some point");
        let mine = me("s1", Some(pid));

        let predecessor = format!("claude-seat s9 pid {pid} born 1 as {image}");
        assert!(
            matches!(stood(Some(predecessor.clone()), &mine), Standing::Stale(_)),
            "a claim this number outlived is litter"
        );
        assert!(super::claim_is_dead(&predecessor));

        let undated = format!("claude-seat s9 pid {pid} as {image}");
        assert!(
            matches!(stood(Some(undated.clone()), &mine), Standing::Foreign(_)),
            "a claim that never dated its process is nobody's to inherit"
        );
        assert!(!super::claim_is_dead(&undated));

        // The same claim from the process that is still at the number:
        // another conversation's id, and this session's own to work in.
        let ours = format!("claude-seat s9 pid {pid} born {born} as {image}");
        assert!(matches!(stood(Some(ours.clone()), &mine), Standing::Ours));
        assert!(!super::claim_is_dead(&ours));
    }
}
