//! The ledger the rule reads: where a machine's tickets stand, who is
//! holding them, and the locks that say so.
//!
//! Every read and every write of it happens under one lock
//! ([`Pool::decide`]), so nobody reads a ticket half written and no two
//! units are handed the same room. Liveness is the lock beside each
//! ticket; the rule those tickets are answered by is [`super::queue`],
//! which touches no file at all.

use std::fs::{File, TryLockError};
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::queue::{self, Rank, Ticket};
use super::unit::{Admitted, Ask, COMPILE, HELD, demand};
use crate::locks::Locked;
use crate::wait::{Budget, LOOK_AGAIN, TRY_AGAIN, Wait};

/// The ledger, beside `.git`.
pub(super) const DIR: &str = "pgg-budget";

/// The lock every read and write of the ledger happens under.
const ADMIT: &str = "admit.lock";

/// The arrival counter's file.
pub(super) const SEQ: &str = "seq";

/// Where the seats' running totals stand — what each has been handed, by
/// rank ([`queue::Served`]). Beside the tickets, because it has to
/// outlive the units it counts: a seat that was just served has nothing
/// left in the ledger to say so, and a fairness read off the live
/// tickets alone hands that seat the machine again.
const SERVED: &str = "served";

/// The extension of a lock file; a ticket's own file has no extension
/// beyond its name, so the two never collide in the directory.
const LOCK: &str = "lock";

/// How long the queue may stand entirely still before a wait calls it
/// hung. Silence is what is measured: a queue that keeps handing the
/// machine on is working however long this unit stands in it, and what
/// this catches is a ledger nothing moves — every holder gone without
/// its ticket coming down, which the sweep would have answered, or a
/// machine where nothing finishes at all.
pub(super) const QUIET_CEILING: Duration = if cfg!(test) {
    Duration::from_secs(20)
} else {
    Duration::from_secs(30 * 60)
};

/// The backstop under it, for a queue that keeps moving and never
/// reaches this unit. Longer than one gate of every seat on the machine,
/// because that is what a unit at the back of a full queue is waiting
/// for.
const WHOLE_CEILING: Duration = if cfg!(test) {
    Duration::from_secs(60)
} else {
    Duration::from_secs(4 * 60 * 60)
};

/// How long a leftover may run before it is worth reporting
/// ([`Pool::leftover`]), counted **from the second it was handed the
/// machine** ([`queue::Ticket::ran_since`]): the longest a step may run
/// at all, read off the ceiling that says so, because a leftover *is* a
/// step that was running when its runner died.
///
/// **It reports and holds on.** A step past this is still a step
/// on the machine, and the room it holds is room it is using; what
/// crossing this says is that nothing but a person will end it now
/// ([`queue::Ticket::overdue`]).
pub(super) const LEFTOVER_CEILING: u64 = crate::check::STEP_CEILING.as_secs();

/// The machine's budget, as one process sees it.
pub(crate) struct Pool {
    dir: PathBuf,
    budget: u32,
    /// This whole process runs under a ticket its parent took, so every
    /// unit of it is admitted outright ([`HELD`]).
    pub(super) carried: bool,
}

impl Pool {
    /// The pool of the repository `dir` belongs to, asking for the
    /// machine's `jobs` verbs a side.
    ///
    /// **The one shape that reads the environment.** A pool named
    /// outright ([`Pool::at`]) is the tests' and answers to nothing but
    /// its arguments: the runner's own suite runs as a step of a gate,
    /// which marks every child as carried, and a pool that read that
    /// mark would hand out passes to the very tests watching it queue.
    pub(crate) fn of(dir: &Path, jobs: usize) -> Result<Pool, String> {
        let common = crate::subprocess::common_git_dir(&dir.display().to_string())
            .ok_or_else(|| format!("{} is not a git repository", dir.display()))?;
        Ok(Pool {
            carried: std::env::var_os(HELD).is_some(),
            // At least one unit's weight: a budget too small for the
            // heaviest unit is one nothing ever fits in.
            ..Pool::at(Path::new(&common), demand(jobs).max(COMPILE))
        })
    }

    /// The same, beside a `.git` named outright and with the budget
    /// spelled out — the tests' own, answering to its arguments
    /// alone.
    pub(crate) fn at(common: &Path, budget: u32) -> Pool {
        Pool {
            dir: common.join(DIR),
            budget,
            carried: false,
        }
    }

    /// Takes a ticket for one unit, waiting for the machine to have room
    /// for it.
    pub(crate) fn admit(&self, ask: &Ask<'_>) -> Result<Admitted, String> {
        self.admit_polled(ask, &|| {})
    }

    /// The same, once no measurement is holding the machine still
    /// (`still::until_free`). Every unit that is about to run
    /// something goes through here: a ticket taken ahead of the
    /// measurement would sit in the ledger as a unit that is running
    /// when it cannot start, and a reader could not tell the queue
    /// from a stall.
    pub(crate) fn admit_once_the_machine_is_free(&self, ask: &Ask<'_>) -> Result<Admitted, String> {
        // Off the ledger's own directory: this is asked once per unit,
        // and reading it from a tree would spend a `git rev-parse` on
        // every one of a gate's seven hundred.
        if let Some(common) = self.dir.parent() {
            crate::still::until_free_in(common, ask.what)?;
        }
        self.admit(ask)
    }

    /// The same, given up the moment `stop` says so while the unit still
    /// waits — for a measurement to let the machine go, or for room:
    /// `Ok(None)` is a unit that was never admitted, and its ticket goes
    /// with it. A gate that has gone red takes nothing more of the
    /// machine (`gate::halt`) — least of all the room another seat is
    /// waiting for.
    pub(crate) fn admit_unless(
        &self,
        ask: &Ask<'_>,
        stop: &dyn Fn() -> bool,
    ) -> Result<Option<Admitted>, String> {
        if let Some(common) = self.dir.parent()
            && !crate::still::until_free_in_unless(common, ask.what, stop)?
        {
            return Ok(None);
        }
        if self.carried {
            return Ok(Some(Admitted::carried()));
        }
        self.queued_unless(ask, false, &|| {}, &|| {}, stop)
    }

    /// Takes this landing's turn, waiting for the landings that arrived
    /// before it. Outside the budget: a turn asks for none of the
    /// machine, and the landing's own units ask under [`Rank::Landing`].
    pub(crate) fn turn(&self, seat: &str, what: &str) -> Result<Admitted, String> {
        self.turn_polled(seat, what, &|| {})
    }

    /// The same, saying so the instant this landing is in the ledger.
    pub(crate) fn turn_arriving(
        &self,
        seat: &str,
        what: &str,
        arrived: &dyn Fn(),
    ) -> Result<Admitted, String> {
        self.queued(&Ask::turn(seat, what), true, arrived, &|| {})
    }

    pub(super) fn turn_polled(
        &self,
        seat: &str,
        what: &str,
        polled: &dyn Fn(),
    ) -> Result<Admitted, String> {
        self.queued(&Ask::turn(seat, what), true, &|| {}, polled)
    }

    /// Takes a ticket, saying so the instant this unit is in the ledger
    /// — before it is admitted, and whether or not it ever is.
    ///
    /// **What arrival is, and why it is said by the unit itself.** A
    /// process that wants to know another has joined the queue — the
    /// suite, driving several holders against one budget — can only read
    /// that off the ledger's own report, and the report says what the
    /// machine is doing in words a person reads: an empty ledger still
    /// says "0 landing(s) in line". A reader matching a word in it is
    /// answered by a unit that has not started, and lets go of the room
    /// it was holding for one that is not in the queue yet — which is
    /// admission in the wrong order, and then a deadlock for as long as
    /// the test's ceiling (`tests/gate/budget.rs`). So the arrival is a
    /// word from the unit, said once, at the one instant its ticket is
    /// readable by everybody.
    pub(crate) fn admit_arriving(
        &self,
        ask: &Ask<'_>,
        arrived: &dyn Fn(),
    ) -> Result<Admitted, String> {
        if self.carried {
            // Nothing joins the queue, so the arrival is the whole of
            // it: this unit is under its parent's ticket and running.
            arrived();
            return Ok(Admitted::carried());
        }
        self.queued(ask, false, arrived, &|| {})
    }

    pub(super) fn admit_polled(
        &self,
        ask: &Ask<'_>,
        polled: &dyn Fn(),
    ) -> Result<Admitted, String> {
        if self.carried {
            return Ok(Admitted::carried());
        }
        self.queued(ask, false, &|| {}, polled)
    }

    /// The one loop both a unit's ticket and a landing's turn go round:
    /// register, then look at the ledger under its lock until the rule
    /// hands this one the machine.
    fn queued(
        &self,
        ask: &Ask<'_>,
        turn: bool,
        arrived: &dyn Fn(),
        polled: &dyn Fn(),
    ) -> Result<Admitted, String> {
        self.queued_unless(ask, turn, arrived, polled, &|| false)?
            .ok_or_else(|| format!("{}: the wait for room was given up", ask.what))
    }

    /// [`Self::queued`], looking at `stop` between looks at the ledger: a
    /// wait it ends comes back `None`, the ticket dropped with it.
    fn queued_unless(
        &self,
        ask: &Ask<'_>,
        turn: bool,
        arrived: &dyn Fn(),
        polled: &dyn Fn(),
        stop: &dyn Fn() -> bool,
    ) -> Result<Option<Admitted>, String> {
        std::fs::create_dir_all(&self.dir)
            .map_err(|e| format!("could not make {}: {e}", self.dir.display()))?;
        let (seq, held, paths) = self.register(ask, turn)?;
        // Held from here on whichever way this goes: a ticket dropped on
        // the way out of a failed wait takes its own files with it.
        let mut mine = Admitted {
            mine: Some(paths),
            _lock: Some(held),
            waited: Duration::ZERO,
        };
        // Said after `register` has let the ledger's lock go, so that
        // whoever hears it can read the queue this unit is standing in.
        arrived();
        let mut wait = Wait::new(
            format!("the machine's budget for {}", ask.what),
            Budget::of(QUIET_CEILING, WHOLE_CEILING),
            LOOK_AGAIN,
        );
        loop {
            let (admitted, standing) = self.look(ask, seq, turn)?;
            if admitted {
                mine.waited = if wait.looks() > 0 {
                    wait.elapsed()
                } else {
                    Duration::ZERO
                };
                return Ok(Some(mine));
            }
            if stop() {
                return Ok(None);
            }
            // The queue moving is what renews the silence budget, so a
            // wait behind a hundred units is not read as a hang and a
            // ledger nothing moves at all is.
            wait.saw(standing);
            polled();
            wait.look_again("room on the machine").map_err(|expired| {
                format!(
                    "{expired} — `cargo xtask budget` says what is holding it, and `cargo xtask \
                     still` what else is under way"
                )
            })?;
        }
    }

    /// Puts this unit in the ledger as waiting, under the ledger's lock:
    /// the arrival number, its own lock taken before its name exists (so
    /// a ticket that can be read is one whose lock is held), and the
    /// file the others read.
    fn register(
        &self,
        ask: &Ask<'_>,
        turn: bool,
    ) -> Result<(u64, Locked, (PathBuf, PathBuf)), String> {
        let _decision = self.decide()?;
        // The dead swept before a name is taken, so that a number this
        // one is about to write at is never one another process can
        // still read a gone unit's weight from.
        let live = self.read()?;
        // Where this seat stands, written the moment it joins the
        // queue. Arrival and first service are two moments: a seat
        // that queued at the start and has been served nothing must
        // stand *below* one that has been served, while a seat
        // arriving into somebody else's hour must stand *level* with
        // it. Only a row made on arrival can tell those apart —
        // without one, both look alike and the older unit wins, which
        // is the seat that was already served (`queue::order`).
        if !turn {
            let served = self.served(&live);
            if !served.contains_key(&(ask.rank, ask.seat.to_string())) {
                let floor = queue::floor_of(&live, &served);
                self.serve(&served, floor, ask.rank, ask.seat, 0);
            }
        }
        let mut seq = self.next_seq()?;
        loop {
            let lock_path = self.dir.join(format!("t-{seq}.{LOCK}"));
            let lock = open_lock(&lock_path)?;
            match lock.try_lock() {
                Ok(()) => {
                    let held = Locked::new(lock);
                    let ticket = self.dir.join(format!("t-{seq}"));
                    let text = render(&Ticket {
                        seq,
                        pid: std::process::id(),
                        weight: ask.weight,
                        budget: self.budget,
                        rank: ask.rank,
                        turn,
                        seat: ask.seat.to_string(),
                        what: ask.what.to_string(),
                        // Nothing started yet, and nothing asked after:
                        // both are the running unit's to fill in, as is
                        // the second it is handed the machine at.
                        child: 0,
                        child_name: String::new(),
                        probed: 0,
                        told: false,
                        orphaned: false,
                        overdue: false,
                        running: false,
                        ran_since: 0,
                    });
                    std::fs::write(&ticket, text)
                        .map_err(|e| format!("could not write {}: {e}", ticket.display()))?;
                    return Ok((seq, held, (ticket, lock_path)));
                }
                // Somebody holds that number: the counter was lost and
                // started again over live tickets. Take the next.
                Err(TryLockError::WouldBlock) => seq += 1,
                Err(TryLockError::Error(error)) => {
                    return Err(format!(
                        "could not open the ticket {}: {error}",
                        lock_path.display()
                    ));
                }
            }
        }
    }

    /// One look at the ledger under its lock: the dead swept, the rule
    /// asked, and this unit written as running when it is handed the
    /// machine. Comes back with what the queue looked like, for the wait
    /// to tell a queue that moves from one that does not.
    fn look(&self, ask: &Ask<'_>, seq: u64, turn: bool) -> Result<(bool, String), String> {
        let _decision = self.decide()?;
        let mine = self.read()?;
        let budget = queue::budget_of(&mine, self.budget);
        let served = self.served(&mine);
        let standing = format!(
            "{} of {budget} held, {} waiting",
            queue::used(&mine),
            queue::order(&mine, &served).len()
        );
        let admitted = if turn {
            queue::turn_is(&mine, seq)
        } else {
            queue::admits(&mine, budget, seq, &served)
        };
        if !admitted {
            return Ok((false, standing));
        }
        let ticket = self.dir.join(format!("t-{seq}"));
        let text = std::fs::read_to_string(&ticket)
            .map_err(|e| format!("could not read back {}: {e}", ticket.display()))?;
        let mut held = parse(&text).ok_or_else(|| {
            format!(
                "this unit's own ticket at {} cannot be read back",
                ticket.display()
            )
        })?;
        held.running = true;
        // The unit starts here, and this is the second every ceiling on
        // it is measured from ([`Ticket::ran_since`]).
        held.ran_since = now();
        std::fs::write(&ticket, render(&held))
            .map_err(|e| format!("could not write {}: {e}", ticket.display()))?;
        // The seat's total moves with the unit it was just handed, and a
        // turn moves nothing: it asks for none of the machine, so it is
        // not a share of it to be counted against the seat.
        if !turn {
            let floor = queue::floor_of(&mine, &served);
            let at = queue::stood_at(&served, ask.rank, ask.seat, floor);
            self.serve(&served, at, ask.rank, ask.seat, ask.weight);
        }
        Ok((true, standing))
    }

    /// The ledger as it stands, the tickets of processes that are gone
    /// taken away on the way past. Held under [`Pool::decide`] by every
    /// caller.
    fn read(&self) -> Result<Vec<Ticket>, String> {
        let entries = std::fs::read_dir(&self.dir)
            .map_err(|e| format!("could not read {}: {e}", self.dir.display()))?;
        let mut tickets = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy().to_string();
            if !name.starts_with("t-") {
                continue;
            }
            // A lock with no ticket beside it is either a unit between
            // its own two removals — which holds it, and is left alone —
            // or a register that died between taking the name and
            // writing at it. The first is microseconds; the second would
            // otherwise stay for good, the names never being reused.
            if let Some(stem) = name.strip_suffix(&format!(".{LOCK}")) {
                let orphan = self.dir.join(stem);
                if !orphan.exists() {
                    let path = entry.path();
                    if let Ok(lock) = File::options().read(true).write(true).open(&path)
                        && lock.try_lock().is_ok()
                    {
                        let lock = Locked::new(lock);
                        let _ = std::fs::remove_file(&path);
                        drop(lock);
                    }
                }
                continue;
            }
            let path = entry.path();
            let lock_path = self.dir.join(format!("{name}.{LOCK}"));
            let Ok(lock) = File::options().read(true).write(true).open(&lock_path) else {
                // A name with no lock beside it is half of a ticket a
                // crash left; the reader that meets it takes it away.
                let _ = std::fs::remove_file(&path);
                continue;
            };
            match lock.try_lock() {
                // Nobody holds it: the unit's process is gone. Whether
                // its *work* is gone is another question, and the wrong
                // answer here is over-subscription nobody is left to
                // notice ([`leftover`]).
                Ok(()) => {
                    let lock = Locked::new(lock);
                    if let Some(leftover) = self.leftover(&path)? {
                        drop(lock);
                        tickets.push(leftover);
                        continue;
                    }
                    let _ = std::fs::remove_file(&path);
                    let _ = std::fs::remove_file(&lock_path);
                    drop(lock);
                }
                Err(TryLockError::WouldBlock) => {
                    if let Ok(text) = std::fs::read_to_string(&path)
                        && let Some(ticket) = parse(&text)
                    {
                        tickets.push(ticket);
                    }
                }
                Err(TryLockError::Error(error)) => {
                    return Err(format!(
                        "could not probe the ticket {}: {error}",
                        lock_path.display()
                    ));
                }
            }
        }
        Ok(tickets)
    }

    /// What a ticket whose owner is gone is still holding the machine
    /// for, or `None` when the room is free to hand out.
    ///
    /// **A lock that frees says the unit's own process ended; it says
    /// nothing about what that process started.** A gate killed mid-step
    /// leaves its cargo, its rustc, its container or its app running —
    /// on unix in a process group of its own, on Windows with a parent
    /// that is simply gone (`crate::reap`) — and that load is on the
    /// machine whether or not anybody is left waiting on it. Handing the
    /// room out then is the one thing the whole budget exists to stop,
    /// and nothing would ever report it.
    ///
    /// So the room is held while the unit's child is still *running* —
    /// a process that has exited and not been waited on holds nothing,
    /// and inside a container nobody ever waits on it (`subprocess`) —
    /// and `cargo xtask budget` says whose it was and what number to
    /// look at. Ending it is a person's: what a killed gate leaves is a
    /// cargo, a container or an app, and `cargo xtask kill` reaps only
    /// the last of those.
    ///
    /// **Which is asked of the number and the name together.** A pid is
    /// handed out again the moment its process is gone, so "something is
    /// running there" would hold the room for whatever inherited it —
    /// a git, a browser tab — for that stranger's whole life. The unit
    /// wrote down what it started ([`Ticket::child_name`]) at the same
    /// instant it wrote the number, which costs nothing, and the two are
    /// compared as one answer (`subprocess::image_still_at`). A ticket
    /// that recorded no name is asked only about the number.
    ///
    /// **The probe is the whole of the decision.** A step that has
    /// run past the longest a step may run ([`LEFTOVER_CEILING`]) is
    /// a thing to report: the load behind it is on the machine
    /// whether or not it is late, and admitting somebody into a room
    /// whose occupant is still compiling is the over-subscription
    /// this exists to stop — for four weight, with nobody left to
    /// notice. So the ceiling turns the leftover into an anomaly that
    /// names itself ([`Ticket::overdue`], `queue::standing`) and
    /// holds on, and the same is true where the probe could not be
    /// asked at all: an unanswerable question holds the room as
    /// well.
    ///
    /// What that costs is a room a person may have to end by hand: a
    /// stranger that inherited both the number and the name, or a
    /// machine where the probe is refused for good, holds it until it
    /// exits or the ticket is taken away. That cost is deliberate,
    /// because the other way round is a machine that quietly runs at
    /// twice its budget, and it is the one direction nothing downstream
    /// can detect. Every waiting unit's wait says what it is behind, and
    /// `cargo xtask budget` says which number to look at.
    ///
    /// The ceiling is measured from the second the unit was handed
    /// the machine ([`Ticket::ran_since`]), so a step that queued
    /// out a busy machine is not reported as late for having
    /// waited.
    ///
    /// A unit that has started nothing yet (`child` zero — every waiting
    /// unit, and every one that takes no child at all) is let go of at
    /// once.
    fn leftover(&self, path: &Path) -> Result<Option<Ticket>, String> {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Ok(None);
        };
        let Some(mut ticket) = parse(&text) else {
            return Ok(None);
        };
        if ticket.child == 0 || !ticket.running {
            return Ok(None);
        }
        let now = now();
        // Asked at most once a second for the whole machine, because on
        // Windows the ask is a process of its own and every waiter looks
        // ten times a second. Between asks the last answer stands, which
        // is what `probed` marks — a pace, being how often the ledger
        // pays for the question.
        if ticket.probed == now {
            return Ok(Some(held(&ticket, now)));
        }
        if !still_at(ticket.child, &ticket.child_name) {
            return Ok(None);
        }
        ticket.probed = now;
        // Said once for the whole machine: the ledger remembers that
        // somebody has been told, and the standing goes on naming it
        // for anyone who asks later.
        if overdue(&ticket, now) && !ticket.told {
            ticket.told = true;
            println!(
                "  budget: {} has held {} weight past the longest a step may run, and pid {} \
                 ({}) is still there. The room is held until it goes — `cargo xtask budget` \
                 says where it stands, and ending it is a person's.",
                ticket.what, ticket.weight, ticket.child, ticket.child_name
            );
        }
        std::fs::write(path, render(&ticket))
            .map_err(|e| format!("could not write {}: {e}", path.display()))?;
        Ok(Some(held(&ticket, now)))
    }

    /// The ledger's own lock, held for the length of one decision: every
    /// read and every write of it happens under this, so nobody reads a
    /// ticket half written and no two units are handed the same room.
    fn decide(&self) -> Result<Locked, String> {
        decide_in(&self.dir)
    }

    /// What each seat has been handed so far, by rank. Held under
    /// [`Pool::decide`] by every caller, as the tickets are.
    ///
    /// Rows for seats with nothing live here are dropped on the way
    /// past — such a seat starts again from the floor whatever its row
    /// said (`queue::stood_at`), so the row is nothing but a name the
    /// file would keep for good.
    fn served(&self, tickets: &[Ticket]) -> queue::Served {
        let text = std::fs::read_to_string(self.dir.join(SERVED)).unwrap_or_default();
        let mut served = queue::Served::new();
        for line in text.lines() {
            let mut fields = line.split_whitespace();
            let (Some(rank), Some(seat), Some(weight)) =
                (fields.next(), fields.next(), fields.next())
            else {
                continue;
            };
            let Ok(weight) = weight.parse::<u64>() else {
                continue;
            };
            let key = (Rank::parse(rank), seat.to_string());
            if tickets
                .iter()
                .any(|t| !t.turn && t.rank == key.0 && t.seat == key.1)
            {
                served.insert(key, weight);
            }
        }
        served
    }

    /// Writes the seats' totals back, `seat` advanced by `weight` from
    /// where it was standing — the unit it has just been handed.
    fn serve(&self, served: &queue::Served, at: u64, rank: Rank, seat: &str, weight: u32) {
        let mut served = served.clone();
        served.insert((rank, seat.to_string()), at + u64::from(weight));
        let text: String = served
            .iter()
            .map(|((rank, seat), weight)| format!("{} {seat} {weight}\n", rank.word()))
            .collect();
        let _ = std::fs::write(self.dir.join(SERVED), text);
    }

    /// The next arrival number. Held under [`Pool::decide`] by its one
    /// caller, so the read and the write are one step.
    fn next_seq(&self) -> Result<u64, String> {
        let path = self.dir.join(SEQ);
        let seq = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| text.trim().parse::<u64>().ok())
            .unwrap_or(0);
        std::fs::write(&path, format!("{}\n", seq + 1))
            .map_err(|e| format!("could not write {}: {e}", path.display()))?;
        Ok(seq)
    }

    /// What the machine is doing, for `cargo xtask budget` and for a
    /// report that has to say whether a unit was queued or wedged.
    pub(crate) fn standing(&self) -> Result<String, String> {
        let _decision = self.decide()?;
        let tickets = self.read()?;
        let budget = queue::budget_of(&tickets, self.budget);
        let served = self.served(&tickets);
        Ok(queue::standing(&tickets, budget, &served))
    }
}

/// What one ticket file says, for a report that reads the ledger without
/// touching it (`verify::wedge`): the weight, whether its unit is
/// running, and whether it is a landing's turn. `None` where the file
/// cannot be read at all — which is not the same as a ticket that says
/// nothing, and the caller has to say so.
pub(crate) fn probe(ticket: &Path) -> Option<(u32, bool, bool)> {
    let text = std::fs::read_to_string(ticket).ok()?;
    let ticket = parse(&text)?;
    Some((ticket.weight, ticket.running, ticket.turn))
}

/// The ledger's own lock at `dir`, held for the length of one decision.
/// A free function because a ticket in hand knows where its ledger is
/// without a pool to ask (`Admitted::started`).
pub(super) fn decide_in(dir: &Path) -> Result<Locked, String> {
    let path = dir.join(ADMIT);
    let lock = open_lock(&path)?;
    // Microseconds of file reading, so the tries come close together;
    // the ceiling is a backstop against a process killed between the
    // lock and the unlock, which the operating system undoes for us
    // anyway.
    let mut tries = Wait::new("the ledger's lock", Budget::whole(QUIET_CEILING), TRY_AGAIN);
    loop {
        match lock.try_lock() {
            Ok(()) => return Ok(Locked::new(lock)),
            Err(TryLockError::WouldBlock) => tries.saw("another process deciding"),
            Err(TryLockError::Error(error)) => {
                return Err(format!(
                    "could not probe the ledger's lock at {}: {error}",
                    path.display()
                ));
            }
        }
        tries
            .look_again("the ledger")
            .map_err(|expired| format!("{expired} — the ledger at {} is wedged", dir.display()))?;
    }
}

fn open_lock(path: &Path) -> Result<File, String> {
    File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|e| format!("could not open the lock at {}: {e}", path.display()))
}

pub(super) fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// Whether the work a leftover started is still the thing running at
/// `child` — the number and the name asked as one question, so that a
/// pid the system has handed out again is not read as the work
/// ([`Pool::leftover`]).
///
/// A ticket that recorded no name is asked only whether anything is
/// there: a name is written at the same instant as the number, so a
/// nameless ticket is one written by a build that writes no name, or
/// by hand.
fn still_at(child: u32, name: &str) -> bool {
    if name.is_empty() {
        return crate::subprocess::running_at(child);
    }
    crate::subprocess::image_still_at(child, name)
}

/// Whether a running leftover has been on the machine longer than a step
/// is allowed to run ([`LEFTOVER_CEILING`]) — which says it is worth
/// reporting, its room still held ([`Pool::leftover`]).
fn overdue(ticket: &Ticket, now: u64) -> bool {
    now.saturating_sub(ticket.ran_since) > LEFTOVER_CEILING
}

/// A leftover as the rule and the standing meet it: still holding its
/// room, marked as nobody's, and marked late where it is.
fn held(ticket: &Ticket, now: u64) -> Ticket {
    Ticket {
        orphaned: true,
        overdue: overdue(ticket, now),
        ..ticket.clone()
    }
}

/// A ticket as its file holds it: one `<key> <value>` a line, and the
/// two that can carry spaces — the program a unit started and what the
/// unit is — each the whole of its own line.
pub(super) fn render(ticket: &Ticket) -> String {
    format!(
        "seq {}\npid {}\nweight {}\nbudget {}\nrank {}\nturn {}\nrunning {}\nran-since {}\n\
         child {}\nprobed {}\ntold {}\nseat {}\nchild-name {}\nwhat {}\n",
        ticket.seq,
        ticket.pid,
        ticket.weight,
        ticket.budget,
        ticket.rank.word(),
        u8::from(ticket.turn),
        u8::from(ticket.running),
        ticket.ran_since,
        ticket.child,
        ticket.probed,
        u8::from(ticket.told),
        ticket.seat,
        ticket.child_name,
        ticket.what,
    )
}

pub(super) fn parse(text: &str) -> Option<Ticket> {
    let mut ticket = Ticket {
        seq: u64::MAX,
        pid: 0,
        weight: 0,
        budget: 0,
        rank: Rank::Normal,
        turn: false,
        seat: String::new(),
        what: String::new(),
        running: false,
        ran_since: 0,
        child: 0,
        child_name: String::new(),
        probed: 0,
        told: false,
        orphaned: false,
        overdue: false,
    };
    for line in text.lines() {
        let (key, value) = line.split_once(' ').unwrap_or((line, ""));
        match key {
            "seq" => ticket.seq = value.parse().ok()?,
            "pid" => ticket.pid = value.parse().ok()?,
            "weight" => ticket.weight = value.parse().ok()?,
            "budget" => ticket.budget = value.parse().ok()?,
            "rank" => ticket.rank = Rank::parse(value),
            "turn" => ticket.turn = value == "1",
            "running" => ticket.running = value == "1",
            "ran-since" => ticket.ran_since = value.parse().ok()?,
            "child" => ticket.child = value.parse().ok()?,
            "child-name" => ticket.child_name = value.to_string(),
            "probed" => ticket.probed = value.parse().ok()?,
            "told" => ticket.told = value == "1",
            "seat" => ticket.seat = value.to_string(),
            "what" => ticket.what = value.to_string(),
            _ => {}
        }
    }
    (ticket.seq != u64::MAX).then_some(ticket)
}
