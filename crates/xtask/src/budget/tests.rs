//! The ledger between threads: what a unit waits for, what it is handed
//! and what a holder that is gone gives back.
//!
//! Every wait here is driven by the thing it waits for — a ticket
//! dropped, a look taken and said on a channel
//! (.claude/rules/core.md §非同期・並行テスト). The rule that decides the
//! order is arithmetic and is tested as such in [`super::queue`]; what
//! these watch is the ledger under it: the files, the locks, the sweep,
//! and the loop that reads them.
//!
//! Between processes — a holder killed outright, a landing's turn
//! against another program's — is `tests/gate/budget.rs`, where the
//! runner can be started as itself.

use std::path::Path;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use super::ledger::{self, DIR, Pool, SEQ, now};
use super::unit;
use super::{Admitted, Ask, LIGHT, Rank, demand, under, weight_of};
use crate::yard::Yard;

/// A `.git`-shaped directory of this test's own, gone when the test is.
fn common(name: &str) -> Yard {
    Yard::new(&format!("budget-{name}"))
}

/// The waits under test say every look they take on a channel, so a look
/// is proved by the word of it.
fn polls() -> (Receiver<()>, impl Fn()) {
    let (said, looks) = std::sync::mpsc::channel();
    (looks, move || {
        let _ = said.send(());
    })
}

/// Waits for the first look: a wait that has looked once is one that
/// found the machine full.
fn until_polled(looks: &Receiver<()>) {
    crate::wait::heard("the wait under test", "a look", looks);
}

/// Waits for a look taken after this moment — the ones already said
/// drained first, so what it proves is that the waiter looked again
/// under the state the test has just made.
fn until_polled_again(looks: &Receiver<()>) {
    while looks.try_recv().is_ok() {}
    crate::wait::heard("the wait under test", "another look", looks);
}

fn ask<'a>(what: &'a str, weight: u32, rank: Rank, seat: &'a str) -> Ask<'a> {
    Ask {
        weight,
        rank,
        seat,
        what,
    }
}

/// One unit's whole life on a thread: it says when it was admitted and
/// holds until the test says to let go.
struct Held {
    admitted: Receiver<Duration>,
    release: std::sync::mpsc::Sender<()>,
    thread: std::thread::JoinHandle<()>,
    looks: Receiver<()>,
}

impl Held {
    /// Starts a unit that asks for `weight` at `rank` for `seat`.
    fn start(dir: &Path, what: &str, weight: u32, rank: Rank, seat: &str, budget: u32) -> Held {
        let (looks, polled) = polls();
        let (said, admitted) = std::sync::mpsc::channel();
        let (release, released) = std::sync::mpsc::channel::<()>();
        let (dir, what, seat) = (dir.to_path_buf(), what.to_string(), seat.to_string());
        let thread = std::thread::spawn(move || {
            let pool = Pool::at(&dir, budget);
            let held = pool
                .admit_polled(&ask(&what, weight, rank, &seat), &polled)
                .expect("a ticket");
            said.send(held.waited).expect("say it was admitted");
            crate::wait::heard("the unit", "the word to let go", &released);
            drop(held);
        });
        Held {
            admitted,
            release,
            thread,
            looks,
        }
    }

    /// Waits until this unit has the machine, and says how long it stood.
    fn taken(&self) -> Duration {
        crate::wait::heard("the test", "the unit admitted", &self.admitted)
    }

    fn let_go(self) {
        self.release.send(()).expect("let the unit go");
        self.thread.join().expect("the unit's thread");
    }
}

/// A unit that does not fit waits, and takes the room the one before it
/// gives back.
#[test]
fn a_unit_takes_the_room_the_one_before_it_gives_back() {
    let dir = common("room");
    let first = Held::start(&dir, "clippy", 4, Rank::Normal, "a", 4);
    assert_eq!(first.taken(), Duration::ZERO, "an empty machine is no wait");
    let second = Held::start(&dir, "test", 4, Rank::Normal, "b", 4);
    until_polled(&second.looks);
    assert!(
        second.admitted.try_recv().is_err(),
        "two four-weight units ran on a budget of four"
    );
    first.let_go();
    assert!(second.taken() > Duration::ZERO, "the wait is reported");
    second.let_go();
}

/// The room a unit gives back goes to the landing that was waiting for
/// it, and the gate's unit that was waiting first goes on waiting.
#[test]
fn a_landing_is_handed_the_room_a_gate_s_unit_was_waiting_for() {
    let dir = common("landing-first");
    let holder = Held::start(&dir, "held", 4, Rank::Normal, "a", 4);
    holder.taken();
    let ordinary = Held::start(&dir, "verb", 4, Rank::Normal, "b", 4);
    until_polled(&ordinary.looks);
    let landing = Held::start(&dir, "land's clippy", 4, Rank::Landing, "c", 4);
    until_polled(&landing.looks);
    holder.let_go();
    assert!(landing.taken() > Duration::ZERO, "the landing's wait");
    until_polled_again(&ordinary.looks);
    assert!(
        ordinary.admitted.try_recv().is_err(),
        "the gate's unit was handed the room ahead of the landing's"
    );
    landing.let_go();
    ordinary.taken();
    ordinary.let_go();
}

/// The admission stop: while a landing's unit is short of room, the room
/// a finishing unit gives back is not handed to ordinary work that
/// would fit in it — otherwise a stream of small units would keep the
/// landing short for as long as the stream lasted.
#[test]
fn ordinary_work_is_not_admitted_into_room_a_waiting_landing_is_short_of() {
    let dir = common("admission-stop");
    let holder = Held::start(&dir, "held", 3, Rank::Normal, "a", 4);
    holder.taken();
    let landing = Held::start(&dir, "land's test", 4, Rank::Landing, "b", 4);
    until_polled(&landing.looks);
    // One weight is free and this unit asks for exactly that.
    let small = Held::start(&dir, "verb", 1, Rank::Normal, "c", 4);
    until_polled(&small.looks);
    assert!(
        small.admitted.try_recv().is_err(),
        "the one free weight went to ordinary work the landing is short of"
    );
    holder.let_go();
    landing.taken();
    until_polled_again(&small.looks);
    assert!(
        small.admitted.try_recv().is_err(),
        "ordinary work ran beside the landing that took the whole machine"
    );
    landing.let_go();
    small.taken();
    small.let_go();
}

/// What is left over after the landing's units fit is handed on.
#[test]
fn room_a_landing_will_not_use_goes_on_down_the_queue() {
    let dir = common("leftover");
    let landing = Held::start(&dir, "land's verb", 1, Rank::Landing, "a", 4);
    landing.taken();
    let ordinary = Held::start(&dir, "verb", 1, Rank::Normal, "b", 4);
    assert_eq!(
        ordinary.taken(),
        Duration::ZERO,
        "three weights were free and ordinary work stood still"
    );
    ordinary.let_go();
    landing.let_go();
}

/// Landings go one at a time in the order they arrived, and the turn
/// asks for none of the budget.
#[test]
fn landings_take_their_turn_in_the_order_they_arrived() {
    let dir = common("turns");
    let (first_looks, first_polled) = polls();
    let pool = Pool::at(&dir, 4);
    let first = pool
        .turn_polled("a", "land a", &first_polled)
        .expect("the first landing's turn");
    assert!(
        first_looks.try_recv().is_err(),
        "an empty ledger cost the first landing a look"
    );
    let (second_looks, second_polled) = polls();
    let waiting_in = dir.to_path_buf();
    let second = std::thread::spawn(move || {
        let pool = Pool::at(&waiting_in, 4);
        pool.turn_polled("b", "land b", &second_polled)
            .map(|turn| turn.waited)
    });
    until_polled(&second_looks);
    assert!(!second.is_finished(), "two landings ran at once");
    // A turn takes none of the machine: the landing's own units do.
    let unit = Held::start(&dir, "land's clippy", 4, Rank::Landing, "a", 4);
    unit.taken();
    unit.let_go();
    drop(first);
    let waited = second
        .join()
        .expect("the second landing's thread")
        .expect("the turn, once the first landing was done");
    assert!(waited > Duration::ZERO, "the wait is reported");
}

/// A ticket whose holder is gone holds nothing: the next process to read
/// the ledger takes it away and the machine is whole again. This is the
/// shape a killed gate leaves — the lock is the operating system's to
/// release, and the file it names is not.
#[test]
fn a_ticket_nobody_holds_the_lock_beside_gives_the_machine_back() {
    let dir = common("dead-ticket");
    let ledger = dir.join(DIR);
    std::fs::create_dir_all(&ledger).expect("the ledger");
    std::fs::write(
        ledger.join("t-0"),
        "seq 0\npid 1\nweight 4\nbudget 4\nrank normal\nturn 0\nrunning 1\nsince 0\nseat z\n\
         side host\nwhat a gate that was killed\n",
    )
    .expect("a dead gate's ticket");
    std::fs::write(ledger.join("t-0.lock"), b"").expect("its lock, held by nobody");
    // The counter past it, so that what the sweep takes away is told
    // apart from the name this unit writes at.
    std::fs::write(ledger.join(SEQ), "1\n").expect("the counter");
    // A lock a register died before writing a ticket at, which nothing
    // else would ever look at again.
    std::fs::write(ledger.join("t-9.lock"), b"").expect("a name nobody wrote at");
    let pool = Pool::at(&dir, 4);
    let mine = pool
        .admit(&ask("clippy", 4, Rank::Normal, "a"))
        .expect("the machine, the dead ticket being nobody's");
    assert_eq!(mine.waited, Duration::ZERO, "the dead ticket cost a wait");
    assert!(!ledger.join("t-0").exists(), "the dead ticket stands");
    assert!(!ledger.join("t-0.lock").exists(), "its lock file stands");
    assert!(!ledger.join("t-9.lock").exists(), "the empty name stands");
    drop(mine);
}

/// A ticket a killed unit left, with the process it had started still
/// on the machine: the room stays held, and the ledger says what it is
/// being held for.
#[test]
fn a_killed_unit_s_room_is_held_while_what_it_started_runs() {
    let dir = common("left-behind");
    let ledger = dir.join(DIR);
    std::fs::create_dir_all(&ledger).expect("the ledger");
    // This very process stands in for the cargo the unit started: it is
    // certainly running, and its number is certainly not one the system
    // has handed out to somebody else.
    dead_ticket(&ledger, "t-0", std::process::id(), &my_name(), now());
    let pool = Pool::at(&dir, 4);
    let text = pool.standing().expect("the standing");
    assert!(
        text.contains("budget 4/4"),
        "the room was handed out: {text}"
    );
    assert!(text.contains("LEFTOVER"), "{text}");
    assert!(ledger.join("t-0").exists(), "the leftover was swept");
}

/// And it is held for the work itself. A pid is handed out again
/// the moment its process is gone, so the unit wrote down what it
/// started beside where; a live number carrying somebody else's
/// program is not the work, and the room goes back at once, at the
/// first look.
#[test]
fn a_leftover_lets_the_room_go_when_the_number_carries_a_stranger() {
    let dir = common("left-behind-stranger");
    let ledger = dir.join(DIR);
    std::fs::create_dir_all(&ledger).expect("the ledger");
    // This process's own number again — certainly live, so the name is
    // the only thing that can answer — recorded as a program it is not.
    dead_ticket(
        &ledger,
        "t-0",
        std::process::id(),
        "not-this-program",
        now(),
    );
    let pool = Pool::at(&dir, 4);
    let text = pool.standing().expect("the standing");
    assert!(
        text.contains("budget 0/4"),
        "a stranger at the number held the room: {text}"
    );
    assert!(!ledger.join("t-0").exists(), "the leftover stands");
}

/// And a leftover that has run past the longest a step may run is
/// **reported, its room held**. The load behind it is on the machine
/// whether or not it is late, so letting the room go on elapsed time
/// alone is the over-subscription this whole thing exists to stop —
/// admitting four weight into a room whose occupant is still
/// compiling, with nobody left to notice. Crossing the ceiling says
/// only that nothing but a person will end it.
#[test]
fn a_leftover_past_the_ceiling_is_reported_and_its_room_is_not_handed_out() {
    let dir = common("left-behind-ceiling");
    let ledger = dir.join(DIR);
    std::fs::create_dir_all(&ledger).expect("the ledger");
    // The same live number and the same name as the leftover above —
    // only the second it started running is moved past the ceiling.
    let long_ago = now() - ledger::LEFTOVER_CEILING - 1;
    dead_ticket(&ledger, "t-0", std::process::id(), &my_name(), long_ago);
    let pool = Pool::at(&dir, 4);
    let text = pool.standing().expect("the standing");
    assert!(
        text.contains("budget 4/4"),
        "a live child's room was handed out on elapsed time: {text}"
    );
    assert!(text.contains("LEFTOVER OVERDUE"), "{text}");
    assert!(ledger.join("t-0").exists(), "the leftover was swept");
    assert!(
        ticket_at(&ledger.join("t-0")).told,
        "the ledger did not remember that it had said so"
    );
}

/// A leftover past the ceiling whose child has gone is nobody's: the
/// probe is the whole of the decision, so the room comes back at once
/// and the ceiling changes nothing about it.
#[test]
fn a_leftover_past_the_ceiling_gives_the_room_back_when_its_child_has_gone() {
    let dir = common("left-behind-ceiling-gone");
    let ledger = dir.join(DIR);
    std::fs::create_dir_all(&ledger).expect("the ledger");
    let long_ago = now() - ledger::LEFTOVER_CEILING - 1;
    let nobody = crate::subprocess::NO_SUCH_PID;
    dead_ticket(&ledger, "t-0", nobody, &my_name(), long_ago);
    let pool = Pool::at(&dir, 4);
    let text = pool.standing().expect("the standing");
    assert!(
        text.contains("budget 0/4"),
        "the room is still held for a child that is gone: {text}"
    );
    assert!(!ledger.join("t-0").exists(), "the leftover stands");
}

/// That ceiling is measured from the second the unit was handed the
/// machine, which is why a ticket carries no date at all until then. A
/// date written at the registration would put a unit that queued out a
/// busy machine past its ceiling in the first second it ran, and the
/// room it was still using would be handed to somebody else.
#[test]
fn a_ticket_is_dated_from_the_run_and_not_from_the_registration() {
    let dir = common("dated-from-the-run");
    let ledger = dir.join(DIR);
    let holder = Held::start(&dir, "held", 4, Rank::Normal, "a", 4);
    holder.taken();
    let queued = Held::start(&dir, "queued", 4, Rank::Normal, "b", 4);
    until_polled(&queued.looks);
    let waiting = ticket_at(&ledger.join("t-1"));
    assert!(!waiting.running, "the second unit was admitted");
    assert_eq!(
        waiting.ran_since, 0,
        "a unit that has not been handed the machine is dated as running"
    );
    holder.let_go();
    queued.taken();
    let run = ticket_at(&ledger.join("t-1"));
    assert!(run.running, "the second unit was not handed the machine");
    assert_ne!(
        run.ran_since, 0,
        "the date was not written when the unit was handed the machine"
    );
    queued.let_go();
}

/// One ticket file as the ledger holds it.
fn ticket_at(path: &Path) -> super::queue::Ticket {
    ledger::parse(&std::fs::read_to_string(path).expect("the ticket file")).expect("a ticket")
}

/// A ticket its owner is no longer there to give back: the file with
/// nobody holding the lock beside it, naming `child` and `program` as
/// the work it started and when it started running.
fn dead_ticket(ledger: &Path, name: &str, child: u32, program: &str, ran_since: u64) {
    std::fs::write(
        ledger.join(name),
        format!(
            "seq 0\npid 1\nweight 4\nbudget 4\nrank normal\nturn 0\nrunning 1\n\
             ran-since {ran_since}\nchild {child}\nprobed 0\ntold 0\nseat z\nside host\n\
             child-name {program}\nwhat a gate that was killed\n"
        ),
    )
    .expect("a killed unit's ticket");
    std::fs::write(ledger.join(format!("{name}.lock")), b"").expect("its lock, held by nobody");
}

/// The name this process would be found under, which is what a unit
/// that started it would have recorded (`subprocess::as_probed`).
fn my_name() -> String {
    crate::subprocess::as_probed(
        &std::env::current_exe()
            .expect("this process's own program")
            .display()
            .to_string(),
    )
}

/// A ticket comes down with its holder, files and all: nothing collects
/// in the ledger and nothing is counted twice.
#[test]
fn a_ticket_takes_its_own_files_with_it() {
    let dir = common("clean-up");
    let pool = Pool::at(&dir, 4);
    let mine = pool
        .admit(&ask("verb", 1, Rank::Normal, "a"))
        .expect("a ticket");
    let ledger = dir.join(DIR);
    let files = || {
        std::fs::read_dir(&ledger)
            .expect("the ledger")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .filter(|name| name.starts_with("t-"))
            .count()
    };
    assert_eq!(files(), 2, "a ticket is its file and its lock");
    drop(mine);
    assert_eq!(files(), 0, "the ticket outlived its holder");
}

/// A wait for room given up (`admit_unless`, a gate gone red) comes back
/// with no ticket and leaves none behind: the room it stood in line for
/// is not held for a run that has nothing left to start.
#[test]
fn a_wait_given_up_leaves_no_ticket() {
    let dir = common("given-up");
    let pool = Pool::at(&dir, 4);
    let holder = pool
        .admit(&ask("clippy", 4, Rank::Normal, "a"))
        .expect("a ticket");
    // Asked between looks at a ledger that had no room: the first ask is
    // the word that the unit is in line, and the answer is the test's.
    let (said, asked) = std::sync::mpsc::channel();
    let given_up = std::sync::atomic::AtomicBool::new(false);
    let result = std::thread::scope(|scope| {
        let waiter = scope.spawn(|| {
            pool.admit_unless(&ask("verb", 1, Rank::Normal, "b"), &|| {
                let _ = said.send(());
                given_up.load(std::sync::atomic::Ordering::SeqCst)
            })
        });
        crate::wait::heard("the test", "the unit in line", &asked);
        given_up.store(true, std::sync::atomic::Ordering::SeqCst);
        waiter.join().expect("the waiter's thread")
    });
    assert!(
        result.expect("no error").is_none(),
        "a wait that was given up came back admitted"
    );
    let tickets = std::fs::read_dir(dir.join(DIR))
        .expect("the ledger")
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("t-"))
        .count();
    assert_eq!(tickets, 2, "only the holder's ticket and its lock stand");
    drop(holder);
}

/// A gate whose ticket is refused by nothing still says what it is
/// waiting behind: what `cargo xtask budget` prints, and what a wait
/// that ran out has to name.
#[test]
fn the_standing_names_what_is_holding_the_machine() {
    let dir = common("standing");
    let pool = Pool::at(&dir, 24);
    let held = pool
        .admit(&ask("test platitude-core", 4, Rank::Normal, "d"))
        .expect("a ticket");
    let turn = pool.turn("e", "land worktree-e").expect("a landing's turn");
    let text = pool.standing().expect("the standing");
    for word in [
        "budget 4/24",
        "1 landing(s) in line",
        "landing  landing weight 0 — land worktree-e",
        "running",
        "test platitude-core",
        "seat d",
    ] {
        assert!(text.contains(word), "{word:?} is not in {text}");
    }
    drop(turn);
    drop(held);
}

/// A pool named outright answers to its arguments and to nothing in the
/// environment. The runner's own suite runs as a step of a gate, which
/// marks every child of a step as carried — a pool that read that mark
/// would hand a pass to the very tests above, which watch units queue.
#[test]
fn a_pool_named_outright_carries_nothing_from_the_environment() {
    let dir = common("no-ambient");
    let pool = Pool::at(&dir, 4);
    assert!(!pool.carried, "a named pool read the environment's mark");
}

/// The gate suite cannot `use` this crate's modules, so it spells the
/// name out; drift between the spellings would put that suite back under
/// a gate's own mark without saying so.
#[test]
fn the_gate_suite_clears_the_name_this_module_spells() {
    let support = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("gate")
        .join("support.rs");
    let text = std::fs::read_to_string(&support).expect("the gate suite's sandbox");
    let name = crate::budget::HELD;
    assert!(
        text.contains(&format!("env_remove(\"{name}\")")),
        "{} does not clear {name}",
        support.display()
    );
}

/// A standalone command that carries its parent's mark answers before it
/// goes looking for a ledger. That order is the whole of how a container
/// run works: the ticket was taken on the host, where the ledger is, and
/// from inside the mount there may be no road to it at all — a `.git`
/// that is a file naming a directory outside the container
/// (`verify::wedge`). A pass that needed a repository would turn every
/// such run into an error.
#[test]
fn a_carried_command_is_admitted_without_a_ledger_to_ask() {
    let nowhere = std::env::temp_dir().join("pgg-budget-no-repository-here");
    let carried = unit::marked(&nowhere, LIGHT, Rank::Normal, "linux test", true)
        .expect("a carried command needs no ledger");
    assert!(carried.mine.is_none(), "a carried command took a ticket");
    let refused = unit::marked(&nowhere, LIGHT, Rank::Normal, "linux test", false)
        .expect_err("an uncarried command with no ledger to ask");
    assert!(refused.contains("not a git repository"), "{refused}");
}

/// A child of an admitted unit runs under its parent's ticket: the mark
/// goes on the command, and a nested ask is answered with a pass.
#[test]
fn a_child_is_marked_as_running_under_its_parent_s_ticket() {
    let mut command = std::process::Command::new("git");
    under(&mut command);
    assert!(
        command
            .get_envs()
            .any(|(key, value)| key == crate::budget::HELD && value == Some("1".as_ref())),
        "a child of a unit is not marked as being under it"
    );
}

/// What a unit takes is read off the words it is spelled with, whether
/// the plan's spelling or the copy the gate starts it from.
#[test]
fn the_weight_is_read_off_the_command() {
    let words = |line: &[&str]| line.iter().map(|w| (*w).to_string()).collect::<Vec<_>>();
    let compiles = [
        words(&["cargo", "clippy", "-p", "xtask", "--all-targets"]),
        words(&["cargo", "test", "-p", "platitude-core", "--test", "it"]),
        words(&["cargo", "run", "-p", "xtask", "--", "shipped"]),
        words(&["cargo", "run", "-p", "xtask", "--", "linux", "bare"]),
        words(&[
            "cargo", "run", "-p", "xtask", "--", "linux", "test", "-p", "xtask",
        ]),
        words(&["C:/x/target/gate-logs/xtask-runner-7.exe", "shipped"]),
    ];
    for command in compiles {
        assert_eq!(
            weight_of(&command, false),
            crate::budget::COMPILE,
            "{command:?} starts a compiler"
        );
    }
    let reads = [
        words(&["cargo", "fmt", "--all", "--", "--check"]),
        words(&["cargo", "run", "-p", "xtask", "--", "structure"]),
        words(&["cargo", "run", "-p", "xtask", "--", "waits"]),
        words(&["cargo", "run", "-p", "xtask", "--", "deny"]),
        words(&["cargo", "run", "-p", "xtask", "--", "linux", "qmltest"]),
        words(&["C:/x/target/gate-logs/xtask-runner-7.exe", "docs"]),
        // The alias, which is how the container's own command line is
        // built and how a person types it.
        words(&["cargo", "xtask", "qmltest"]),
        words(&["cargo", "xtask", "linux", "docs"]),
    ];
    for command in reads {
        assert_eq!(
            weight_of(&command, false),
            LIGHT,
            "{command:?} starts no compiler"
        );
    }
    // The one verb of a side that builds the release is a build; the
    // rest reuse it.
    let verb = words(&["cargo", "run", "-p", "xtask", "--", "verify-ui", "wip"]);
    assert_eq!(weight_of(&verb, false), crate::budget::COMPILE);
    assert_eq!(weight_of(&verb, true), LIGHT);
}

/// The budget is one gate's widest moment, so a gate running alone never
/// waits for itself.
#[test]
fn the_budget_is_one_gate_s_worth_of_machine() {
    // Both sides at once: a compiling checks chain beside eight verbs.
    assert_eq!(demand(8), 2 * (crate::budget::COMPILE + 8 * LIGHT));
    assert_eq!(demand(8), 24);
    assert!(
        demand(1) >= 2 * crate::budget::COMPILE + 2 * LIGHT,
        "a machine with one verb a side still fits both sides' chains"
    );
}

/// A unit that asks for nothing and holds nothing: what a child under
/// its parent's ticket gets back.
#[test]
fn a_carried_pass_holds_no_files() {
    let carried = Admitted::carried();
    assert_eq!(carried.waited, Duration::ZERO);
    assert!(carried.mine.is_none(), "a carried pass took a ticket");
}
