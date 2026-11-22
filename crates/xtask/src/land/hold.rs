//! `cargo xtask land hold` — one background task of this session kept
//! running while it stands in the line of landings: the hold.
//!
//! The desktop app stops a session's process under memory pressure once
//! it counts the session idle, and a session with no background task left
//! counts as idle until the model's first output of a turn reaches the
//! app: the turn a notice starts — a wait's end, a gate's, a landing's, a
//! subagent's report — spends its first seconds on the hooks and the first
//! answer, and a pressure sweep that falls in them stops it
//! (`%LOCALAPPDATA%\Claude\logs\main.log`: `pressure evicting`). The turn
//! is lost, the app's resume starts none for it, and only the user's next
//! message wakes the session — the message that closes the permit. The
//! app never picks a session with a local background task running, so a
//! landing keeps this one running from its `/land`.
//!
//! It ends without a turn of its own. Its file beside the permit names
//! its pid; the session's own next tool call after the session stops
//! standing takes the file away and waits for the hold to exit
//! ([`end_if_out`]), so the notice of its end joins that call's result —
//! a notice that arrives while a turn runs joins it at the next result,
//! and one that arrives after the turn's last starts another. It ends by
//! itself when a newer hold of the session writes the file, when the
//! Claude process it holds turns for is gone, and when the session's
//! entry is gone while its permit stays open — the conversation went on
//! under another session id (`/clear`), whose calls end no hold of this
//! one's. A SessionEnd ends nothing: the machine's sleep sends it to
//! every open conversation, and each goes on at the wake.
//!
//! Standing is read off the session's two files — the permit open, the
//! entry there and not left — since the line's own reading asks git and
//! writes, on every tool call. The two differ for a `/land` outside a seat
//! and a seat whose claim another conversation took: the hold then lasts
//! until the user's next message and a call after it, spending nothing.
//!
//! It lives as long as a landing waits, so it steps out of the build slot
//! it was started from: the gate and the landing rebuild the task runner
//! there, and a running image cannot be replaced
//! (`run::step_out_of_the_build_slot`). The next cargo run in the seat
//! builds the runner again. Not while a gate of that tree runs ([`Slot`]).

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::hook::permit::{self, COMMAND};

/// The hold's file beside the permit: its pid.
const KIND: &str = "hold";

/// How often the hold looks at its file — the session's call that ends it
/// waits on that look.
const LOOK: Duration = crate::wait::LOOK_AGAIN;

/// How many looks between asks after the Claude process and the session's
/// entry: the ask after a process starts one (`tasklist`, `ps`).
const ASK_EVERY: u32 = 50;

/// The name the Claude process runs under, compared with what stands at
/// its pid so that a pid handed out again ends the hold too.
const CLAUDE: &str = "claude";

/// How long a call that ends the hold waits for it to go — several of its
/// looks. A wedged hold is left to its own end, and the call goes on.
const GONE_WITHIN: Duration = Duration::from_secs(2);

/// `cargo xtask land hold`.
pub(super) fn run(args: &[String]) -> Result<(), String> {
    if args.len() > 1 {
        return Err(format!(
            "land hold takes nothing more (got {:?})",
            &args[1..]
        ));
    }
    let here = crate::seats::slashed(&crate::tree::workspace_root());
    let me = crate::seats::Identity::current(None);
    let claude = std::env::var("CLAUDE_PID")
        .ok()
        .and_then(|pid| pid.trim().parse::<u32>().ok())
        .ok_or("no Claude process to hold turns for: CLAUDE_PID names none")?;
    let primary = crate::tree::primary_root(&here)
        .ok_or_else(|| format!("{here} is in no checkout with a primary one"))?;
    let file = start(&primary, &me.session, std::process::id())?;
    println!(
        "holding this session's turns while it stands in the line of landings; the session's \
         own next tool call after that ends the hold, and the notice of its end needs no answer"
    );
    println!(
        "hold ended: {}",
        hold(
            &primary,
            &me.session,
            &file,
            (claude, CLAUDE),
            &mut Slot::of(&here)
        )
    );
    Ok(())
}

/// The build slot the hold was started from, until it has stepped out.
///
/// Not while a gate of the slot's tree runs: that gate's tests start the
/// task runner from the slot by name (`CARGO_BIN_EXE_xtask`), and the step
/// out takes every name of it away until a cargo builds it again — every
/// start in between is a runner not found. Nothing is lost by waiting: a
/// gate started as `cargo xtask gate` runs from the slot itself, so the
/// slot is not free before it ends, and `land` steps out by itself before
/// its gate rebuilds the runner (`run::prepare_gate`).
struct Slot {
    tree: String,
    /// Stepped out: nothing more to look at.
    out: bool,
    /// Said once that a gate keeps the hold in the slot.
    waiting: bool,
}

impl Slot {
    fn of(tree: &str) -> Self {
        Self {
            tree: tree.to_string(),
            out: false,
            waiting: false,
        }
    }

    /// One look: steps out of the slot unless a gate of its tree runs, and
    /// what to say of it — the step out, or, at the first look a gate runs,
    /// that the hold stays. A lock that cannot be looked at is no gate the
    /// hold knows of, and is said.
    fn look(&mut self) -> Option<String> {
        if self.out {
            return None;
        }
        let gate = crate::gate::running_in(Path::new(&self.tree));
        if gate == Ok(true) {
            if self.waiting {
                return None;
            }
            self.waiting = true;
            return Some(format!(
                "a gate is running in {}: this hold stays in the build slot until that gate is \
                 over — its tests start the task runner there by name",
                self.tree
            ));
        }
        self.out = true;
        let said = super::run::step_out_of_the_build_slot(&[self.tree.as_str()]);
        match gate {
            Err(why) => Some(format!(
                "{why} — stepped out as with no gate running{}",
                said.map(|word| format!("\n{word}")).unwrap_or_default()
            )),
            Ok(_) => said,
        }
    }
}

/// Writes this hold's file for `session`, which has to stand in the line,
/// and takes away the files of holds that are gone.
fn start(primary: &Path, session: &str, pid: u32) -> Result<PathBuf, String> {
    if session.is_empty() {
        return Err(
            "no session identity — the line is per session (CLAUDE_CODE_SESSION_ID)".into(),
        );
    }
    if !stands(primary, session) {
        return Err(format!(
            "this session stands in no line of landings — nothing to hold until the user's next \
             {COMMAND}"
        ));
    }
    let file = permit::kept_file_in(primary, session, KIND)
        .ok_or_else(|| format!("no file name for the session {session:?}"))?;
    sweep(file.parent().unwrap_or(primary));
    std::fs::write(&file, format!("{pid}\n"))
        .map_err(|e| format!("could not write {}: {e}", file.display()))?;
    Ok(file)
}

/// Looks until this hold has to end, and says why. `claude` is the Claude
/// process's pid and the name it runs under. Every look is also the
/// `slot`'s ([`Slot::look`]), the first before anything can end the hold.
fn hold(
    primary: &Path,
    session: &str,
    file: &Path,
    claude: (u32, &str),
    slot: &mut Slot,
) -> &'static str {
    let mine = std::process::id();
    let mut looks: u32 = 0;
    loop {
        if let Some(word) = slot.look() {
            println!("{word}");
        }
        // A file being written over — a newer hold's — reads as no pid for
        // the moment it is empty; only a file not there is gone.
        match held_by(file) {
            None if !file.exists() => {
                return "its file is gone — the session's own call found it out of the line";
            }
            Some(pid) if pid != mine => return "a newer hold of this session took its place",
            None | Some(_) => {}
        }
        if looks.is_multiple_of(ASK_EVERY) {
            let gone = if !crate::subprocess::image_still_at(claude.0, claude.1) {
                Some("the Claude process it held turns for is gone")
            } else if moved_on(primary, session) {
                Some("the conversation went on under another session id")
            } else {
                None
            };
            if let Some(why) = gone {
                if held_by(file) == Some(mine) {
                    let _gone = std::fs::remove_file(file);
                }
                return why;
            }
        }
        looks = looks.wrapping_add(1);
        // waits(paced): a look at the hold's file every `LOOK` — the call
        // that ends the hold waits on this look; the Claude process and the
        // session's entry are asked after every `ASK_EVERY` of them
        std::thread::sleep(LOOK);
    }
}

/// The session's own tool call, at its start or its end: the hold of a
/// session that no longer stands in the line ends, and the call waits for
/// it to exit so that the notice of its end joins this call's result. One
/// look for a file, for a session with no hold.
pub(crate) fn end_if_out(cwd: &str, session: &str) {
    let Some(primary) = crate::tree::primary_root(cwd) else {
        return;
    };
    let Some(file) = permit::kept_file_in(&primary, session, KIND) else {
        return;
    };
    let Some(pid) = held_by(&file) else {
        return;
    };
    if stands(&primary, session) || std::fs::remove_file(&file).is_err() {
        return;
    }
    let mut gone = crate::wait::Wait::new(
        "the ended hold",
        crate::wait::Budget::whole(GONE_WITHIN),
        crate::wait::LOOK_AGAIN,
    );
    while crate::subprocess::running_at(pid) {
        if gone.look_again("its exit").is_err() {
            return;
        }
    }
}

/// Whether `session` stands in the line as its files say: its permit open
/// and its entry there, not left.
fn stands(primary: &Path, session: &str) -> bool {
    permit::kept_file_in(primary, session, "land")
        .is_some_and(|permit| permit::open_at(&permit) && super::line::written_beside(&permit))
}

/// Whether the session's entry is gone while its permit stands open: the
/// first message of the conversation under its next session id took the
/// entry away (`line::enter` / `line::clear`), and the user's own next
/// message under this id would have closed the permit.
fn moved_on(primary: &Path, session: &str) -> bool {
    permit::kept_file_in(primary, session, "land")
        .is_some_and(|permit| permit::open_at(&permit) && !super::line::entry_beside(&permit))
}

/// The pid a hold's file names.
fn held_by(file: &Path) -> Option<u32> {
    std::fs::read_to_string(file).ok()?.trim().parse().ok()
}

/// Takes away the hold files that name a process no longer running — a
/// hold killed with its Claude process.
fn sweep(permits: &Path) {
    let Ok(entries) = std::fs::read_dir(permits) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|extension| extension == KIND)
            && held_by(&path).is_some_and(|pid| !crate::subprocess::process_exists(pid))
        {
            let _gone = std::fs::remove_file(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{KIND, Slot, end_if_out, held_by, hold, start};
    use crate::hook::permit;
    use std::path::{Path, PathBuf};

    const SESSION: &str = "hold-test";

    /// A checkout where `SESSION` stands in the line as the prompt hook
    /// leaves it after `/land` — its permit open, its entry written — the
    /// permit's path, and a pid no process has.
    fn standing() -> (crate::yard::Yard, String, PathBuf, u32) {
        let yard = crate::yard::Yard::new("land-hold");
        let root = yard.to_str().unwrap().replace('\\', "/");
        assert!(crate::subprocess::git_query(&root, &["init"]).is_some());
        let primary = crate::tree::primary_root(&root).unwrap();
        let permit = permit::kept_file_in(&primary, SESSION, "land").unwrap();
        std::fs::create_dir_all(permit.parent().unwrap()).unwrap();
        set(&permit, "open");
        crate::land::line::enter(&permit, &root, None, 1_000);
        (yard, root, permit, crate::subprocess::NO_SUCH_PID)
    }

    /// The permit as `hook::permit` writes it.
    fn set(permit: &Path, standing: &str) {
        std::fs::write(permit, format!("{standing}\t1\t/land\n")).unwrap();
    }

    /// Runs the hold loop of this process on `file` in a thread, with
    /// `claude` as its Claude process — this very process by its own name
    /// unless a pid nobody has — and hands back what it said, with its
    /// slot: the checkout's, which a test binary is never started from, so
    /// a step out moves nothing.
    fn holding(
        primary: &Path,
        file: &Path,
        claude: u32,
    ) -> std::sync::mpsc::Receiver<(&'static str, Slot)> {
        let me = std::process::id();
        std::fs::write(file, format!("{me}\n")).unwrap();
        let image = crate::subprocess::image_of(me).expect("this process is behind its own pid");
        let (primary, file) = (primary.to_path_buf(), file.to_path_buf());
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut slot = Slot::of(&primary.to_string_lossy());
            let why = hold(&primary, SESSION, &file, (claude, &image), &mut slot);
            let _sent = tx.send((why, slot));
        });
        rx
    }

    /// The checkout's gate, running: the lock its note stands beside
    /// (`gate::run::running_note`).
    fn gate_running(primary: &Path) -> crate::lanes::Sole {
        let note = primary.join("target").join("gate-running");
        crate::lanes::sole(&note, "gate").expect("the checkout's gate")
    }

    #[test]
    fn a_hold_stays_in_its_build_slot_while_its_trees_gate_runs() {
        let (yard, root, _permit, _gone) = standing();
        let primary = crate::tree::primary_root(&root).unwrap();
        let mut slot = Slot::of(&primary.to_string_lossy());
        let gate = gate_running(&primary);
        let said = slot.look().expect("a word that the hold stays");
        assert!(said.contains("a gate is running"), "{said}");
        assert_eq!(slot.look(), None, "said once");
        assert!(!slot.out, "stepped out from under the running gate");
        drop(gate);
        let _said = slot.look();
        assert!(slot.out, "the gate is over");
        drop(yard);
    }

    /// Every look of the loop asks the slot before anything can end it, so
    /// a hold ended mid-gate has looked while the gate ran, and one ended
    /// after has looked with none.
    #[test]
    fn the_hold_loop_steps_out_at_a_look_with_no_gate_running_and_at_no_other() {
        let (yard, root, _permit, _gone) = standing();
        let primary = crate::tree::primary_root(&root).unwrap();
        let file = permit::kept_file_in(&primary, SESSION, KIND).unwrap();
        let gate = gate_running(&primary);
        let said = holding(&primary, &file, std::process::id());
        std::fs::remove_file(&file).unwrap();
        let (why, slot) = crate::wait::heard("the hold", "its end", &said);
        assert!(why.contains("file is gone"), "{why}");
        assert!(
            !slot.out,
            "a hold ended in the middle of the gate never left the slot"
        );

        drop(gate);
        let said = holding(&primary, &file, std::process::id());
        std::fs::remove_file(&file).unwrap();
        let (why, slot) = crate::wait::heard("the hold", "its end", &said);
        assert!(why.contains("file is gone"), "{why}");
        assert!(slot.out, "a look with the gate over steps out");
        drop(yard);
    }

    #[test]
    fn a_hold_starts_only_for_a_session_in_the_line() {
        let (yard, root, permit, gone) = standing();
        let primary = crate::tree::primary_root(&root).unwrap();
        assert!(start(&primary, "someone-else", gone).is_err());
        let file = start(&primary, SESSION, gone).unwrap();
        assert_eq!(held_by(&file), Some(gone));
        set(&permit, "closed");
        assert!(
            start(&primary, SESSION, gone).is_err(),
            "the user's next message closed it"
        );
        drop(yard);
    }

    #[test]
    fn a_call_ends_the_hold_once_the_session_left_the_line() {
        let (yard, root, _permit, gone) = standing();
        let primary = crate::tree::primary_root(&root).unwrap();
        let file = start(&primary, SESSION, gone).unwrap();
        end_if_out(&root, SESSION);
        assert!(file.is_file(), "a session in the line keeps its hold");
        assert!(crate::land::line::leave(&root, SESSION, "test"));
        end_if_out(&root, SESSION);
        assert!(!file.is_file(), "a session that left ends it");
        drop(yard);
    }

    #[test]
    fn a_call_ends_the_hold_once_the_landing_spent_the_permit() {
        let (yard, root, permit, gone) = standing();
        let primary = crate::tree::primary_root(&root).unwrap();
        let file = start(&primary, SESSION, gone).unwrap();
        set(&permit, "spent");
        end_if_out(&root, SESSION);
        assert!(!file.is_file());
        drop(yard);
    }

    #[test]
    fn the_call_that_ends_a_hold_waits_for_its_process_to_go() {
        let (yard, root, permit, _gone) = standing();
        let primary = crate::tree::primary_root(&root).unwrap();
        // A stand-in for the hold: a process that ends when its input does,
        // which a thread closes once the call has taken the file away.
        let mut stand_in = std::process::Command::new("git")
            .args(["hash-object", "--stdin"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let file = start(&primary, SESSION, stand_in.id()).unwrap();
        let input = stand_in.stdin.take().unwrap();
        let watched = file.clone();
        let closer = std::thread::spawn(move || {
            crate::wait::until(
                "the hold's file taken away",
                || watched.is_file(),
                |on: &bool| !*on,
            );
            drop(input);
        });
        set(&permit, "spent");
        end_if_out(&root, SESSION);
        assert!(
            stand_in.try_wait().unwrap().is_some(),
            "the call came back before the hold had gone"
        );
        closer.join().unwrap();
        drop(yard);
    }

    #[test]
    fn a_hold_ends_by_itself_when_another_takes_its_place_or_claude_is_gone() {
        let (yard, root, _permit, gone) = standing();
        let primary = crate::tree::primary_root(&root).unwrap();
        let file = permit::kept_file_in(&primary, SESSION, KIND).unwrap();
        let claude = std::process::id();
        let said = holding(&primary, &file, claude);
        std::fs::write(&file, format!("{gone}\n")).unwrap();
        let (why, _slot) = crate::wait::heard("the hold", "its end", &said);
        assert!(why.contains("newer hold"), "{why}");
        assert!(file.is_file(), "the newer hold's file stays");

        let said = holding(&primary, &file, gone);
        let (why, _slot) = crate::wait::heard("the hold", "its end", &said);
        assert!(why.contains("Claude process"), "{why}");
        assert!(!file.is_file(), "it takes its own file away");
        drop(yard);
    }

    #[test]
    fn a_hold_ends_by_itself_when_the_conversation_moved_on() {
        let (yard, root, permit, _gone) = standing();
        let primary = crate::tree::primary_root(&root).unwrap();
        let file = permit::kept_file_in(&primary, SESSION, KIND).unwrap();
        crate::land::line::clear(&permit, None);
        let said = holding(&primary, &file, std::process::id());
        let (why, _slot) = crate::wait::heard("the hold", "its end", &said);
        assert!(why.contains("another session id"), "{why}");
        assert!(!file.is_file());
        drop(yard);
    }

    #[test]
    fn a_new_hold_takes_away_the_files_of_holds_gone() {
        let (yard, root, _permit, gone) = standing();
        let primary = crate::tree::primary_root(&root).unwrap();
        let other = permit::kept_file_in(&primary, "other-session", KIND).unwrap();
        std::fs::write(&other, format!("{gone}\n")).unwrap();
        let live = permit::kept_file_in(&primary, "live-session", KIND).unwrap();
        std::fs::write(&live, format!("{}\n", std::process::id())).unwrap();
        start(&primary, SESSION, gone).unwrap();
        assert!(!other.is_file(), "a hold gone");
        assert!(live.is_file(), "a hold still running");
        drop(yard);
    }
}
