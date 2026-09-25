//! The ledger the rule ([`super::queue`]) reads: the machine's tickets,
//! who holds them, and the locks that say so.
//!
//! Every read and write happens under one lock ([`Pool::decide`]), so
//! nobody reads a ticket half written and no two units are handed the
//! same room.

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

/// Each seat's running total, by rank ([`queue::Served`]). A file of its
/// own because it must outlive the units it counts: read off the live
/// tickets alone, a seat just served would be handed the machine again.
const SERVED: &str = "served";

/// A lock file's extension; a ticket's own file has none.
const LOCK: &str = "lock";

/// How long the queue may stand entirely still before a wait calls it
/// hung. A queue that keeps moving is working however long this unit
/// waits; what this catches is a ledger nothing moves.
pub(super) const QUIET_CEILING: Duration = if cfg!(test) {
    Duration::from_secs(20)
} else {
    Duration::from_secs(30 * 60)
};

/// The backstop for a queue that keeps moving and never reaches this
/// unit: longer than one gate of every seat, which is what the back of a
/// full queue waits for.
const WHOLE_CEILING: Duration = if cfg!(test) {
    Duration::from_secs(60)
} else {
    Duration::from_secs(4 * 60 * 60)
};

/// How long a leftover may run, from the second it was handed the
/// machine ([`queue::Ticket::ran_since`]), before it is reported
/// ([`Pool::leftover`]): the longest a step may run, since a leftover is
/// a step whose runner died.
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
    /// The pool of the repository `dir` belongs to, for the machine's
    /// `jobs` verbs a side.
    ///
    /// **The one constructor that reads the environment.** [`Pool::at`]
    /// must not: the runner's own suite runs as a gate's step, which
    /// marks every child as carried, and a pool reading that mark would
    /// pass the very tests watching it queue.
    pub(crate) fn of(dir: &Path, jobs: usize) -> Result<Pool, String> {
        let common = crate::subprocess::common_git_dir(&dir.display().to_string())
            .ok_or_else(|| format!("{} is not a git repository", dir.display()))?;
        Ok(Pool {
            carried: std::env::var_os(HELD).is_some(),
            // A budget below the heaviest unit is one nothing fits in.
            ..Pool::at(Path::new(&common), demand(jobs).max(COMPILE))
        })
    }

    /// The same, beside a `.git` named outright and with the budget
    /// spelled out — the tests' own.
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

    /// The same, once no measurement holds the machine still
    /// (`still::until_free_in`) — waited for first, because a ticket
    /// taken ahead of it reads as a running unit that cannot start, and
    /// the queue could not be told from a stall.
    pub(crate) fn admit_once_the_machine_is_free(&self, ask: &Ask<'_>) -> Result<Admitted, String> {
        // Off the ledger's own directory: from a tree it would cost a
        // `git rev-parse` per unit.
        if let Some(common) = self.dir.parent() {
            crate::still::until_free_in(common, ask.what)?;
        }
        self.admit(ask)
    }

    /// The same, given up the moment `stop` says so while the unit still
    /// waits, for a measurement or for room: `Ok(None)` is a unit never
    /// admitted, its ticket gone with it. A red gate takes nothing more
    /// of the machine (`gate::halt`).
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

    /// Takes a ticket, calling `arrived` the instant this unit is in the
    /// ledger — before it is admitted, and whether or not it ever is.
    /// `tests/gate/budget.rs` waits on this and not on the standing,
    /// whose words ("0 landing(s) in line") are true before the unit
    /// has queued.
    pub(crate) fn admit_arriving(
        &self,
        ask: &Ask<'_>,
        arrived: &dyn Fn(),
    ) -> Result<Admitted, String> {
        if self.carried {
            // Under its parent's ticket: nothing joins the queue.
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
    /// register, then look under the ledger's lock until the rule hands
    /// this one the machine.
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
        // Held from here on: a failed wait drops the ticket and its files.
        let mut mine = Admitted {
            mine: Some(paths),
            _lock: Some(held),
            waited: Duration::ZERO,
        };
        // After `register` has let the ledger's lock go, so whoever hears
        // it can read the queue.
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
            // The queue moving renews the silence budget (`QUIET_CEILING`).
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
    /// the arrival number, its own lock taken before its ticket exists
    /// (so a readable ticket is one whose lock is held), and the ticket.
    fn register(
        &self,
        ask: &Ask<'_>,
        turn: bool,
    ) -> Result<(u64, Locked, (PathBuf, PathBuf)), String> {
        let _decision = self.decide()?;
        // The dead swept first, so the number this one writes at never
        // still shows a gone unit's weight.
        let live = self.read()?;
        // The seat's row is made on arrival, not on first service: a seat
        // queued from the start and served nothing must stand below a
        // served one, while a latecomer stands level with the others.
        // Without the row both look alike and the older unit — the served
        // seat's — wins (`queue::order`).
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
                        // Filled in once the unit runs.
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
                // The counter was lost and restarted over live tickets.
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

    /// One look under the ledger's lock: the dead swept, the rule asked,
    /// and this unit written as running if admitted. Also returns the
    /// standing, so the wait can tell a moving queue from a still one.
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
        // Every ceiling on the unit is measured from here
        // ([`Ticket::ran_since`]).
        held.ran_since = now();
        std::fs::write(&ticket, render(&held))
            .map_err(|e| format!("could not write {}: {e}", ticket.display()))?;
        // A turn asks for none of the machine, so it is not counted
        // against the seat.
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
            // A lock with no ticket: a unit between its own two removals
            // (it holds the lock, so it is left alone), or a register that
            // died before writing the ticket, which would otherwise stay
            // for good.
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
                // A ticket with no lock is half of what a crash left.
                let _ = std::fs::remove_file(&path);
                continue;
            };
            match lock.try_lock() {
                // Nobody holds it: the process is gone, but its work may
                // not be ([`leftover`]).
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
    /// **A freed lock says the owner ended, not what it started**: a
    /// killed gate's cargo, container or app keeps running (`crate::reap`).
    /// So the room is held while the unit's child is still *running* — an
    /// exited child nobody waited on holds nothing (`subprocess`) — asked
    /// by number and name together ([`Ticket::child_name`],
    /// `subprocess::image_still_at`), since a pid is handed out again.
    ///
    /// **The probe is the whole of the decision.** Past
    /// [`LEFTOVER_CEILING`] the leftover names itself ([`Ticket::overdue`],
    /// `queue::standing`) and still holds on, and a probe that cannot be
    /// asked holds too. The cost is a room a person may have to end by
    /// hand; the other way round is a machine quietly at twice its budget,
    /// which nothing downstream can detect.
    ///
    /// A unit that has started nothing (`child` zero) is let go at once.
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
        // At most once a second machine-wide: on Windows the ask is a
        // process of its own, and every waiter looks many times a second.
        // Between asks the last answer stands.
        if ticket.probed == now {
            return Ok(Some(held(&ticket, now)));
        }
        if !still_at(ticket.child, &ticket.child_name) {
            return Ok(None);
        }
        ticket.probed = now;
        // Said once machine-wide; the standing goes on naming it.
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

    fn decide(&self) -> Result<Locked, String> {
        decide_in(&self.dir)
    }

    /// What each seat has been handed so far, by rank; held under
    /// [`Pool::decide`] by every caller. Rows of seats with nothing live
    /// are dropped: such a seat restarts from the floor whatever its row
    /// said (`queue::stood_at`).
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

/// One ticket file's (weight, running, turn), for a report that reads the
/// ledger without touching it (`verify::wedge`). `None` where the file
/// cannot be read — not the same as a ticket that says nothing, and the
/// caller has to say so.
pub(crate) fn probe(ticket: &Path) -> Option<(u32, bool, bool)> {
    let text = std::fs::read_to_string(ticket).ok()?;
    let ticket = parse(&text)?;
    Some((ticket.weight, ticket.running, ticket.turn))
}

/// The ledger's own lock at `dir`, held for the length of one decision.
/// A free function because a ticket in hand knows its ledger without a
/// pool (`Admitted::started`).
pub(super) fn decide_in(dir: &Path) -> Result<Locked, String> {
    let path = dir.join(ADMIT);
    let lock = open_lock(&path)?;
    // Held for microseconds, so the tries come close together; the
    // ceiling is only a backstop (the OS releases a killed holder's lock).
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

/// Whether the work a leftover started is still at `child`
/// ([`Pool::leftover`]). A nameless ticket (from a build that writes no
/// name, or by hand) is asked by number only.
fn still_at(child: u32, name: &str) -> bool {
    if name.is_empty() {
        return crate::subprocess::running_at(child);
    }
    crate::subprocess::image_still_at(child, name)
}

fn overdue(ticket: &Ticket, now: u64) -> bool {
    now.saturating_sub(ticket.ran_since) > LEFTOVER_CEILING
}

/// A leftover as the rule and the standing meet it: still holding its
/// room.
fn held(ticket: &Ticket, now: u64) -> Ticket {
    Ticket {
        orphaned: true,
        overdue: overdue(ticket, now),
        ..ticket.clone()
    }
}

/// One `<key> <value>` a line; `child-name` and `what` may carry spaces
/// and run to the end of their line.
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
