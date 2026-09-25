//! The machine's budget between real processes: what one holds another
//! waits for, whose turn comes first, and what a killed holder gives back.
//!
//! Only processes can answer the part the OS owns — the lock released when
//! a holder dies (`flock` / `LockFileEx`); the rule and the ledger are
//! `budget::queue` and `budget::tests`. Each holder is a
//! `cargo xtask budget hold` driven by its own edges (`--queued`, `--say`,
//! `--until`): no clock decides anything, and where a holder stands is
//! never read out of the ledger's report ([`Holder::wait_queued`]).

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use crate::support::{EXE, Sandbox};
use crate::wait::{Budget, LOOK_AGAIN, Wait};

/// One holder process and its three words: joined the queue, admitted,
/// let go.
struct Holder {
    child: Child,
    queued: PathBuf,
    said: PathBuf,
    release: PathBuf,
    what: String,
}

impl Holder {
    fn admitted(&self) -> bool {
        self.said.exists()
    }

    /// Waits until this holder says its ticket is in the ledger. Letting
    /// a holder go before the next has queued hands the room to whoever
    /// had queued, and the test waits out its ceiling; the report cannot
    /// stand in, as "0 landing(s) in line" is true before anything has
    /// queued (`budget::ledger::admit_arriving`).
    fn wait_queued(&mut self) {
        self.wait_for(
            &self.queued.clone(),
            "queued",
            "the word that it had joined the queue",
        );
    }

    fn wait_admitted(&mut self) {
        self.wait_for(
            &self.said.clone(),
            "admitted",
            "the word that it was admitted",
        );
    }

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

    fn let_go(mut self) {
        std::fs::write(&self.release, b"go\n").expect("the word to let go");
        let status = self.child.wait().expect("the holder");
        assert!(status.success(), "the holder {} ended red", self.what);
    }

    /// Kills it holding its ticket — what a gate killed mid-run leaves.
    fn killed(mut self) {
        self.child.kill().expect("kill the holder");
        self.child.wait().expect("the killed holder");
    }
}

impl Drop for Holder {
    /// A test that failed mid-way must not leave its holder on the
    /// machine: its ceiling would keep it holding room later runs wait for.
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn sandbox(name: &str) -> (Sandbox, PathBuf) {
    let sandbox = Sandbox::new(name);
    let repo = sandbox.repo.clone();
    (sandbox, repo)
}

fn hold(sandbox: &Sandbox, repo: &Path, what: &str, args: &[&str]) -> Holder {
    held(sandbox, repo, what, &["--seat", what], args)
}

/// `hold` with the seat given, for several units in one seat.
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

/// The report `cargo xtask budget` prints.
fn standing(sandbox: &Sandbox, repo: &Path) -> String {
    let mut command = Command::new(EXE);
    command.arg("budget").arg("--dir").arg(repo);
    sandbox.env(&mut command);
    let output = command.output().expect("spawn budget");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// The budget is the machine's, across processes that never met.
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

/// Ordinary work stays out until the landing lets go.
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

/// The lock is the OS's to release, and the ticket beside it is litter
/// the next reader takes away.
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

/// a, then b, then a: by the middle step a's first unit has finished, so
/// no ticket is left to say a was served.
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

/// Its lock frees the instant it dies, but the cargo, container or app
/// under it still runs: handing that room out is the over-subscription
/// the budget exists to stop.
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
    // The ledger has to name what it holds the room for.
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
    std::fs::write(&goes, b"go\n").expect("the word to let the child go");
    second.wait_admitted();
    second.let_go();
}

/// A turn asks for none of the budget — the landing's own units do.
#[test]
fn landings_take_their_turn_one_at_a_time() {
    let (sandbox, repo) = sandbox("budget-turns");
    let mut first = hold(&sandbox, &repo, "land-first", &["--turn"]);
    first.wait_admitted();
    let mut second = hold(&sandbox, &repo, "land-second", &["--turn"]);
    second.wait_queued();
    assert!(!second.admitted(), "two landings were under way at once");
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

/// The child runs in the room its parent took.
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
    // After `sandbox.env`, which clears this mark.
    command.env("PGG_BUDGET_HELD", "1");
    let mut child = Holder {
        child: command.spawn().expect("spawn the child"),
        queued,
        said,
        release,
        what: "child".to_string(),
    };
    // A carried unit joins no queue but still says it has arrived.
    child.wait_queued();
    child.wait_admitted();
    child.let_go();
    whole.let_go();
}
