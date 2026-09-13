//! The machine's budget between real processes: what one holds another
//! waits for, whose turn comes first, and what a holder that is killed
//! gives back.
//!
//! Threads answer the rule (`budget::queue`) and the ledger's own files
//! (`budget::tests`); what only processes can answer is the part the
//! operating system owns — a lock released because a process died, which
//! is `flock` on Linux and `LockFileEx` on Windows and is the whole of
//! how a killed gate gives the machine back. So every holder here is a
//! `cargo xtask budget hold` of its own, and the suite drives every one
//! of its edges: it says one word when it has joined the queue
//! (`--queued`), one when it is admitted (`--say`), and holds until a
//! file appears (`--until`). No clock decides anything, and nothing here
//! is read out of the ledger's own report: a holder says where it stands
//! itself, so that letting a holder go can never happen while the one
//! meant to take the room has not arrived ([`Holder::wait_queued`]).

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use crate::support::{EXE, Sandbox};
use crate::wait::{Budget, LOOK_AGAIN, Wait};

/// One holder: the process, the word it says when it has joined the
/// queue, the word it says when it is admitted, and the word that lets
/// it go.
struct Holder {
    child: Child,
    queued: PathBuf,
    said: PathBuf,
    release: PathBuf,
    what: String,
}

impl Holder {
    /// Whether it has been admitted — the word it writes the instant it
    /// has the machine.
    fn admitted(&self) -> bool {
        self.said.exists()
    }

    /// Waits until this holder is standing in the queue — the word it
    /// writes the instant its ticket is in the ledger.
    ///
    /// **Said by the holder rather than read out of the ledger's
    /// report.** A test that let a holder go while the next one had not
    /// arrived would watch the room go to whoever *had* — and then wait
    /// out its own ceiling for the one it meant to admit, which is a
    /// deadlock the report cannot be read for: it says "0 landing(s) in
    /// line" on an empty ledger, so a word looked for in it answers
    /// before anything has arrived (`budget::ledger::admit_arriving`).
    fn wait_queued(&mut self) {
        self.wait_for(
            &self.queued.clone(),
            "queued",
            "the word that it had joined the queue",
        );
    }

    /// Waits until it has been admitted.
    fn wait_admitted(&mut self) {
        self.wait_for(
            &self.said.clone(),
            "admitted",
            "the word that it was admitted",
        );
    }

    /// Waits for one of this holder's words, failing by name if the
    /// holder ends without saying it.
    fn wait_for(&mut self, word: &Path, stage: &str, what_for: &str) {
        let mut wait = Wait::new(
            format!("the holder {}", self.what),
            Budget::SUITE,
            LOOK_AGAIN,
        );
        while !word.exists() {
            if let Some(status) = self.child.try_wait().expect("ask after the holder") {
                panic!(
                    "the holder {} ended before it was {stage} ({status})",
                    self.what
                );
            }
            wait.saw(format!("not {stage} yet"));
            wait.look_again(what_for)
                .unwrap_or_else(|expired| panic!("{expired}"));
        }
    }

    /// Lets it go and waits for it to end.
    fn let_go(mut self) {
        std::fs::write(&self.release, b"go\n").expect("the word to let go");
        let status = self.child.wait().expect("the holder");
        assert!(status.success(), "the holder {} ended red", self.what);
    }

    /// Ends it where it stands, holding its ticket: what a gate killed
    /// mid-run leaves behind.
    fn killed(mut self) {
        self.child.kill().expect("kill the holder");
        self.child.wait().expect("the killed holder");
    }
}

impl Drop for Holder {
    /// A holder whose word never came — a test that failed before it
    /// said it — is not left on the machine: its own ceiling would keep
    /// it there for half an hour, holding room every later run waits for.
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A repository for the holders to share a budget beside. The sandbox's
/// own, so the git configuration is the suite's rather than the user's.
fn sandbox(name: &str) -> (Sandbox, PathBuf) {
    let sandbox = Sandbox::new(name);
    let repo = sandbox.repo.clone();
    (sandbox, repo)
}

/// Starts one holder against `repo`, its seat named after it.
fn hold(sandbox: &Sandbox, repo: &Path, what: &str, args: &[&str]) -> Holder {
    held(sandbox, repo, what, &["--seat", what], args)
}

/// The same, for a test that puts several units in one seat.
fn held(sandbox: &Sandbox, repo: &Path, what: &str, seat: &[&str], args: &[&str]) -> Holder {
    let beside = repo.parent().unwrap_or(repo);
    let (queued, said, release) = (
        beside.join(format!("queued-{what}")),
        beside.join(format!("said-{what}")),
        beside.join(format!("go-{what}")),
    );
    for word in [&queued, &said, &release] {
        let _ = std::fs::remove_file(word);
    }
    let mut command = Command::new(EXE);
    command
        .arg("budget")
        .arg("hold")
        .arg("--dir")
        .arg(repo)
        .arg("--what")
        .arg(what)
        .args(seat)
        .arg("--queued")
        .arg(&queued)
        .arg("--say")
        .arg(&said)
        .arg("--until")
        .arg(&release)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    sandbox.env(&mut command);
    let child = command.spawn().expect("spawn a holder");
    Holder {
        child,
        queued,
        said,
        release,
        what: what.to_string(),
    }
}

/// What the machine says it is doing, as `cargo xtask budget` prints it.
fn standing(sandbox: &Sandbox, repo: &Path) -> String {
    let mut command = Command::new(EXE);
    command.arg("budget").arg("--dir").arg(repo);
    sandbox.env(&mut command);
    let output = command.output().expect("spawn budget");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// The budget is one machine's, not one process's: what the first holds,
/// the second waits for, across processes that never met.
#[test]
fn a_second_process_waits_for_the_room_the_first_is_holding() {
    let (sandbox, repo) = sandbox("budget-two");
    let mut first = hold(&sandbox, &repo, "first", &["--jobs", "1", "--weight", "6"]);
    first.wait_admitted();
    let mut second = hold(&sandbox, &repo, "second", &["--jobs", "1", "--weight", "6"]);
    second.wait_queued();
    assert!(
        !second.admitted(),
        "two units of six ran on a budget of ten:\n{}",
        standing(&sandbox, &repo)
    );
    first.let_go();
    second.wait_admitted();
    second.let_go();
}

/// A landing is handed the room a gate's unit was already waiting for,
/// and ordinary work is not admitted into what the landing has not
/// finished with.
#[test]
fn a_landing_takes_the_room_ahead_of_a_gate_that_was_waiting_first() {
    let (sandbox, repo) = sandbox("budget-landing");
    let mut holder = hold(&sandbox, &repo, "holder", &["--jobs", "1", "--weight", "6"]);
    holder.wait_admitted();
    let mut ordinary = hold(
        &sandbox,
        &repo,
        "ordinary",
        &["--jobs", "1", "--weight", "6"],
    );
    ordinary.wait_queued();
    let mut landing = hold(
        &sandbox,
        &repo,
        "landing",
        &["--jobs", "1", "--weight", "6", "--landing"],
    );
    landing.wait_queued();
    holder.let_go();
    landing.wait_admitted();
    assert!(
        !ordinary.admitted(),
        "the gate's unit was handed the room ahead of the landing's:\n{}",
        standing(&sandbox, &repo)
    );
    landing.let_go();
    ordinary.wait_admitted();
    ordinary.let_go();
}

/// The three ranks in one line, between processes: what moves main goes
/// first, the window somebody is waiting at next, and the tests last.
#[test]
fn the_room_goes_to_the_landing_then_the_launch_then_the_test() {
    let (sandbox, repo) = sandbox("budget-ranks");
    let mut holder = hold(
        &sandbox,
        &repo,
        "holder",
        &["--jobs", "1", "--weight", "10"],
    );
    holder.wait_admitted();
    // Queued in the order that would be wrong if the ranks were not read.
    let mut test = hold(&sandbox, &repo, "test", &["--jobs", "1", "--weight", "10"]);
    test.wait_queued();
    let mut launch = hold(
        &sandbox,
        &repo,
        "launch",
        &["--jobs", "1", "--weight", "10", "--launch"],
    );
    launch.wait_queued();
    let mut landing = hold(
        &sandbox,
        &repo,
        "landing",
        &["--jobs", "1", "--weight", "10", "--landing"],
    );
    landing.wait_queued();
    holder.let_go();
    landing.wait_admitted();
    assert!(!launch.admitted(), "the launch went ahead of the landing");
    assert!(!test.admitted(), "a test went ahead of the landing");
    landing.let_go();
    launch.wait_admitted();
    assert!(
        !test.admitted(),
        "a test was handed the room the launch was waiting for:\n{}",
        standing(&sandbox, &repo)
    );
    launch.let_go();
    test.wait_admitted();
    test.let_go();
}

/// A holder killed where it stands gives the machine back: the lock is
/// the operating system's to release, and the ticket beside it is litter
/// the next reader takes away. This is the shape a killed gate leaves,
/// and the one place the two systems answer for themselves.
#[test]
fn a_holder_killed_where_it_stands_gives_the_machine_back() {
    let (sandbox, repo) = sandbox("budget-killed");
    let mut first = hold(&sandbox, &repo, "killed", &["--jobs", "1", "--weight", "6"]);
    first.wait_admitted();
    let mut second = hold(&sandbox, &repo, "after", &["--jobs", "1", "--weight", "6"]);
    second.wait_queued();
    assert!(!second.admitted(), "the second ran beside the first");
    first.killed();
    second.wait_admitted();
    let seen = standing(&sandbox, &repo);
    assert!(
        !seen.contains("killed"),
        "the killed holder's ticket is still counted:\n{seen}"
    );
    second.let_go();
}

/// Two seats, two units and one: a, then b, then a — and the middle
/// step is the one that is easy to get wrong, because by then a's first
/// unit has **finished** and there is nothing among the tickets to say a
/// was served at all.
#[test]
fn the_seats_take_turns_across_units_that_have_already_finished() {
    let (sandbox, repo) = sandbox("budget-turns-served");
    // Ten is the whole budget at --jobs 1, so exactly one unit runs.
    let whole = ["--jobs", "1", "--weight", "10"];
    let mut holder = hold(&sandbox, &repo, "holder", &whole);
    holder.wait_admitted();
    let mut first = held(&sandbox, &repo, "a-one", &["--seat", "a"], &whole);
    first.wait_queued();
    let mut second = held(&sandbox, &repo, "a-two", &["--seat", "a"], &whole);
    second.wait_queued();
    let mut third = held(&sandbox, &repo, "b-one", &["--seat", "b"], &whole);
    third.wait_queued();

    holder.let_go();
    first.wait_admitted();
    assert!(!second.admitted() && !third.admitted(), "one at a time");
    // a's first is not merely done with the room — it is gone, ticket
    // and all. The next room is b's all the same.
    first.let_go();
    third.wait_admitted();
    assert!(
        !second.admitted(),
        "the seat that was already served took the next room too:\n{}",
        standing(&sandbox, &repo)
    );
    third.let_go();
    second.wait_admitted();
    second.let_go();
}

/// A killed holder gives the machine back **only once what it started
/// has ended**. Its own lock frees the instant it dies, but the cargo,
/// the container or the app under it is still on the machine, and
/// handing that room out is the over-subscription the whole budget is
/// there to stop — with nobody left to notice it.
#[test]
fn a_killed_holder_keeps_its_room_while_what_it_started_runs() {
    let (sandbox, repo) = sandbox("budget-leftover");
    let goes = repo.parent().unwrap_or(&repo).join("go-leftover-child");
    let _ = std::fs::remove_file(&goes);
    let mut first = hold(
        &sandbox,
        &repo,
        "killed",
        &[
            "--jobs",
            "1",
            "--weight",
            "6",
            "--child-until",
            &goes.display().to_string(),
        ],
    );
    first.wait_admitted();
    let mut second = hold(&sandbox, &repo, "after", &["--jobs", "1", "--weight", "6"]);
    second.wait_queued();
    first.killed();
    // The ledger has to say what it is holding the room for, or nobody
    // could act on it.
    let mut wait = Wait::new("the ledger", Budget::SUITE, LOOK_AGAIN);
    loop {
        let seen = standing(&sandbox, &repo);
        if seen.contains("LEFTOVER") {
            break;
        }
        wait.saw(seen);
        wait.look_again("the leftover named")
            .unwrap_or_else(|expired| panic!("{expired}"));
    }
    assert!(
        !second.admitted(),
        "the room came back while the killed holder's child was still running:\n{}",
        standing(&sandbox, &repo)
    );
    // The child ends: now there is nothing left of that unit on the
    // machine, and the room is everybody's again.
    std::fs::write(&goes, b"go\n").expect("the word to let the child go");
    second.wait_admitted();
    second.let_go();
}

/// Landings go one at a time, in the order they arrived, and a turn asks
/// for none of the budget — the landing's own units do that.
#[test]
fn landings_take_their_turn_one_at_a_time() {
    let (sandbox, repo) = sandbox("budget-turns");
    let mut first = hold(&sandbox, &repo, "land-first", &["--turn"]);
    first.wait_admitted();
    let mut second = hold(&sandbox, &repo, "land-second", &["--turn"]);
    second.wait_queued();
    assert!(!second.admitted(), "two landings were under way at once");
    // The turn holds none of the machine: a unit of the whole budget
    // runs beside it.
    let mut beside = hold(
        &sandbox,
        &repo,
        "beside",
        &["--jobs", "1", "--weight", "10"],
    );
    beside.wait_admitted();
    beside.let_go();
    first.let_go();
    second.wait_admitted();
    second.let_go();
}

/// A child of an admitted unit runs under its parent's ticket: it is
/// admitted though the machine is full, because the room its parent took
/// is the room it is running in.
#[test]
fn a_child_under_its_parent_s_ticket_is_admitted_though_the_machine_is_full() {
    let (sandbox, repo) = sandbox("budget-carried");
    let mut whole = hold(&sandbox, &repo, "whole", &["--jobs", "1", "--weight", "10"]);
    whole.wait_admitted();
    let beside = repo.parent().unwrap_or(&repo);
    let (queued, said, release) = (
        beside.join("queued-child"),
        beside.join("said-child"),
        beside.join("go-child"),
    );
    for word in [&queued, &said, &release] {
        let _ = std::fs::remove_file(word);
    }
    let mut command = Command::new(EXE);
    command
        .args(["budget", "hold", "--jobs", "1", "--weight", "6", "--dir"])
        .arg(&repo)
        .arg("--queued")
        .arg(&queued)
        .arg("--say")
        .arg(&said)
        .arg("--until")
        .arg(&release)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    sandbox.env(&mut command);
    // After the sandbox's own environment, which clears this very mark:
    // what is under test here is a child that carries it.
    command.env("PGG_BUDGET_HELD", "1");
    let mut child = Holder {
        child: command.spawn().expect("spawn the child"),
        queued,
        said,
        release,
        what: "child".to_string(),
    };
    // A carried unit joins no queue, and says so all the same: its
    // arrival is the decision that it needs no ticket.
    child.wait_queued();
    child.wait_admitted();
    child.let_go();
    whole.let_go();
}
