//! The pace of the reads a page makes on its own: when this window's tree
//! is read again, and when each other worktree is. Rules only — no
//! clock and no I/O; the driver is `session::pacer`, and every moment here
//! is a [`Duration`] since the driver's origin.
//!
//! What sets the pace is how heavy a read was: the lives of the git
//! children it spawned (`process::Meter` — a proxy for the work, not a CPU
//! reading). A heavy read pushes the next one out at once; light reads
//! pull it back in a step at a time ([`Estimate`]). Nothing here reads a
//! clock to decay, so a late timer, a suspended machine or a hidden window
//! leaves the pace where the reads put it.
//!
//! The ranges are a person's to set ([`PaceBounds`]): a lower floor reads
//! changes sooner and spends more of the machine, a higher ceiling spends
//! less and shows changes later. They are aims, not promises: a read that
//! alone takes longer than the ceiling is followed by the next as soon as
//! it ends and [`OWN_REST`] has passed.
//!
//! This tree comes first. The other worktrees are read one at a time, in
//! the gaps between this tree's reads; a worktree that does not fit a gap
//! waits for the end of this tree's next read once, then goes whether it
//! fits or not ([`Pacer::starts`]), so a worktree slower than the gap is
//! still read.

use std::time::Duration;

/// A moment, as the time since the driver's origin.
pub(super) type At = Duration;

/// This tree's default floor: not read again sooner than this after its
/// last read began.
pub const OWN_FLOOR: Duration = Duration::from_secs(5);
/// Its default ceiling: nor later than this, where one read alone fits in
/// it.
pub const OWN_CEILING: Duration = Duration::from_secs(15);
/// What a read longer than the gap leaves before the next one: without it,
/// a read that takes longer than the ceiling would be followed by the next
/// with no pause at all. Never more than the floor: a floor set under it
/// asks for reads that close together.
pub const OWN_REST: Duration = Duration::from_secs(3);

/// The other worktrees' default floor: none is read again sooner than this
/// after its last read began.
pub const WORKTREE_FLOOR: Duration = Duration::from_secs(5);
/// Their default ceiling: nor later than this, where the reads of all of
/// them fit in it.
pub const WORKTREE_CEILING: Duration = Duration::from_secs(45);

/// The shortest floor a setting can ask for.
pub const PACE_MIN_SECS: u32 = 1;
/// The longest ceiling — an hour, the way the auto-fetch ceiling is one.
pub const PACE_MAX_SECS: u32 = 3600;

/// The floor and the ceiling that will run for the pair asked, in seconds:
/// each in range, and the ceiling never under the floor. The one place the
/// range is applied, as with `auto_fetch_minutes`: the settings screen and
/// a hand-written `settings.toml` write the same fields.
#[must_use]
pub fn pace_bounds_secs(floor: u32, ceiling: u32) -> (u32, u32) {
    let floor = floor.clamp(PACE_MIN_SECS, PACE_MAX_SECS);
    (floor, ceiling.clamp(floor, PACE_MAX_SECS))
}

/// The floors and ceilings the pace keeps to — the settings'
/// (`settings::Defaults::pace_bounds`), each pair through
/// [`pace_bounds_secs`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaceBounds {
    pub own_floor: Duration,
    pub own_ceiling: Duration,
    pub worktree_floor: Duration,
    pub worktree_ceiling: Duration,
}

impl Default for PaceBounds {
    fn default() -> Self {
        Self {
            own_floor: OWN_FLOOR,
            own_ceiling: OWN_CEILING,
            worktree_floor: WORKTREE_FLOOR,
            worktree_ceiling: WORKTREE_CEILING,
        }
    }
}

/// The share of the time this tree's reads may keep children alive: the
/// interval is a read's weight over this. A read under
/// `OWN_FLOOR × OWN_SHARE` is read at the floor. Set so a small tree sits
/// at the floor with room to spare on a slower machine and a tree of the
/// performance budget's size reaches the ceiling
/// (ci/baseline/poll-cost-windows-x64.md).
pub const OWN_SHARE: f64 = 0.05;
/// The share all the other worktrees' reads may keep children alive,
/// divided between them evenly: each worktree's interval is its weight
/// times the count, over this. Set so the budget-sized tree with a few
/// worktrees costs what it did before the worktrees were paced, and a small
/// one reads its worktrees at the floor (same record).
pub const WORKTREE_SHARE: f64 = 0.06;
/// What a light read takes off the weight held: the weight after it is the
/// larger of the read and this much of the weight before. Three light
/// reads in a row bring a weight down to a third.
pub const RELEASE: f64 = 0.7;

/// How heavy reads of one thing are, as the reads have said: up at once
/// with a heavy one, down by [`RELEASE`] with each lighter one.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct Estimate(Option<Duration>);

impl Estimate {
    pub(super) fn observe(&mut self, weight: Duration) {
        self.0 = Some(match self.0 {
            None => weight,
            Some(held) => weight.max(held.mul_f64(RELEASE)),
        });
    }

    pub(super) fn weight(self) -> Option<Duration> {
        self.0
    }
}

/// How the other worktrees are read (`settings::WorktreesReading`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorktreesPace {
    Off,
    /// Every worktree at this interval, whatever its weight.
    Fixed(Duration),
    /// Each worktree at an interval its weight sets ([`WORKTREE_SHARE`]).
    Auto,
}

/// What to start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Start {
    /// This tree's read; `with_stashes` when the window came back since
    /// a read last listed them — no other paced read does.
    Own { with_stashes: bool },
    /// One other worktree's, by its key (`joins::same_path_key`).
    Worktree(String),
}

/// One other worktree, as the pace sees it.
#[derive(Debug, Clone)]
struct OtherWorktree {
    key: String,
    estimate: Estimate,
    /// When its last read began; `None` before the first.
    started: Option<At>,
    due: At,
    /// Turned away once for not fitting before this tree's next read: it
    /// goes when that read ends.
    held: bool,
    /// That read has ended: it goes next, fitting or not.
    forced: bool,
    /// When it was last asked for at once (its reading is behind the
    /// listing, the window came back to its pane, or the worktrees were turned
    /// back on): a new pace does not push it out, and an ask its own read
    /// began before is still owed when that read ends.
    asked: Option<At>,
}

#[derive(Debug, Clone, Default)]
struct Own {
    estimate: Estimate,
    started: Option<At>,
    due: At,
    /// The end of the rest after the last read, which a new pace keeps to.
    rested: At,
    running: bool,
    /// Asked for while a read ran: the next starts as that one ends.
    again: bool,
    /// Asked for at once (the window came back): a new pace does not push
    /// it out.
    asked: bool,
    /// The window came back since a read last listed the stashes: the next
    /// read lists them.
    stashes: bool,
}

/// The pace of one page's reads.
#[derive(Debug, Clone)]
pub(super) struct Pacer {
    /// The page is on screen to be read for.
    active: bool,
    own: Own,
    /// In the listing's order, which breaks ties between worktrees due at
    /// the same moment.
    worktrees: Vec<OtherWorktree>,
    /// The worktree being read; one at a time.
    reading: Option<String>,
    /// A pass over every worktree is out (`RepoSession::refresh_carried`), and
    /// starts none of its own.
    passing: bool,
    pace: WorktreesPace,
    bounds: PaceBounds,
}

impl Pacer {
    pub(super) fn new(pace: WorktreesPace) -> Self {
        Self {
            active: false,
            own: Own::default(),
            worktrees: Vec::new(),
            reading: None,
            passing: false,
            pace,
            bounds: PaceBounds::default(),
        }
    }

    /// New floors and ceilings. The waits already set are planned again
    /// from where their reads began, so a shorter pace set during a long
    /// wait is kept now, not after it; a read under way plans its next as
    /// it ends.
    pub(super) fn set_bounds(&mut self, bounds: PaceBounds) {
        self.bounds = bounds;
        self.plan_own_again();
        self.plan_worktrees_again();
    }

    /// The opening read everything at `at`: this tree's next read is an
    /// interval away, not due the moment the page shows.
    pub(super) fn opened(&mut self, at: At) {
        if self.own.started.is_none() && !self.own.running {
            self.own.started = Some(at);
            if !self.own.asked {
                self.own.due = at + self.own_interval();
            }
        }
    }

    /// The page came on screen or went off it. Coming back after its next
    /// read fell due reads at once; nothing held is forgotten either way.
    pub(super) fn set_active(&mut self, active: bool, at: At) {
        if active && !self.active && self.own.started.is_none() && !self.own.running {
            self.own.due = at;
        }
        self.active = active;
    }

    /// Something likely moved outside (the window came back): read this
    /// tree now, or as soon as the read under way ends — with the stashes.
    pub(super) fn ask_now(&mut self, at: At) {
        self.own.stashes = true;
        if self.own.running {
            self.own.again = true;
        } else {
            self.own.due = self.own.due.min(at);
            self.own.asked = true;
        }
    }

    /// How the worktrees are read, with the waits already set planned again
    /// as [`Self::set_bounds`] plans them. Turned back on, every worktree is
    /// due at once: the off took their rows down. The session's pass usually
    /// reads them first (`RepoSession::set_worktrees_pace`); this reads them
    /// where that pass could not begin.
    pub(super) fn set_pace(&mut self, pace: WorktreesPace, at: At) {
        let was_off = self.pace == WorktreesPace::Off;
        self.pace = pace;
        if was_off && pace != WorktreesPace::Off {
            for worktree in &mut self.worktrees {
                worktree.due = worktree.due.min(at);
                worktree.asked = Some(at);
                worktree.held = false;
            }
        }
        self.plan_worktrees_again();
    }

    /// Whether the page is on screen to be read for.
    pub(super) fn active(&self) -> bool {
        self.active
    }

    /// The worktrees a listing named, by key, in its order. A worktree new
    /// to the pace is due at once; one no longer listed is forgotten.
    pub(super) fn list(&mut self, keys: &[String], at: At) {
        let mut kept = Vec::with_capacity(keys.len());
        for key in keys {
            let worktree = match self.worktrees.iter().position(|c| &c.key == key) {
                Some(found) => self.worktrees.swap_remove(found),
                None => OtherWorktree {
                    key: key.clone(),
                    estimate: Estimate::default(),
                    started: None,
                    due: at,
                    held: false,
                    forced: false,
                    asked: None,
                },
            };
            kept.push(worktree);
        }
        self.worktrees = kept;
    }

    /// A worktree's reading is behind where the listing says it stands, or
    /// the window came back to the pane standing on it: due at once.
    pub(super) fn worktree_stale(&mut self, key: &str, at: At) {
        if let Some(worktree) = self.worktrees.iter_mut().find(|c| c.key == key) {
            worktree.due = worktree.due.min(at);
            worktree.asked = Some(at);
        }
    }

    /// What to start at `at`, marked as started: this tree's read when it
    /// is due, and at most one worktree's.
    pub(super) fn starts(&mut self, at: At) -> Vec<Start> {
        let mut starts = Vec::new();
        if !self.active {
            return starts;
        }
        if !self.own.running && self.own.due <= at {
            self.own.running = true;
            self.own.started = Some(at);
            self.own.asked = false;
            starts.push(Start::Own {
                with_stashes: std::mem::take(&mut self.own.stashes),
            });
        }
        if let Some(key) = self.next_worktree(at) {
            self.reading = Some(key.clone());
            if let Some(worktree) = self.worktrees.iter_mut().find(|c| c.key == key) {
                worktree.started = Some(at);
                worktree.held = false;
                worktree.forced = false;
                worktree.asked = None;
            }
            starts.push(Start::Worktree(key));
        }
        starts
    }

    /// When [`Self::starts`] next has something to start, unless an end
    /// comes first; `None` while nothing is waiting on time.
    pub(super) fn next_wake(&self) -> Option<At> {
        if !self.active {
            return None;
        }
        let own = (!self.own.running).then_some(self.own.due);
        let worktree = if self.reads_worktrees() && self.reading.is_none() && !self.passing {
            self.worktrees
                .iter()
                .filter(|c| !c.held)
                .map(|c| c.due)
                .min()
        } else {
            None
        };
        match (own, worktree) {
            (Some(own), Some(worktree)) => Some(own.min(worktree)),
            (one, other) => one.or(other),
        }
    }

    /// This tree's read ended. `weight` is `None` where the read spawned
    /// nothing of its own: another read answered it.
    pub(super) fn own_ended(&mut self, at: At, weight: Option<Duration>) {
        self.own.running = false;
        if let Some(weight) = weight {
            self.own.estimate.observe(weight);
        }
        let started = self.own.started.unwrap_or(at);
        let rest = OWN_REST.min(self.bounds.own_floor);
        self.own.rested = at + rest;
        self.own.asked = std::mem::take(&mut self.own.again);
        self.own.due = if self.own.asked {
            at
        } else {
            (started + self.own_interval()).max(self.own.rested)
        };
        for worktree in &mut self.worktrees {
            if std::mem::take(&mut worktree.held) {
                worktree.forced = true;
            }
        }
    }

    /// This tree's read, started, was turned away before it read anything
    /// (a write of this tree runs, or another poll holds the flight): it
    /// ends as a read that weighed nothing, and stashes it was to list are
    /// left to the next.
    pub(super) fn own_refused(&mut self, at: At, with_stashes: bool) {
        self.own_ended(at, None);
        self.own.stashes |= with_stashes;
    }

    /// A worktree's read, begun at `began`, ended (`weight` as for
    /// [`Self::own_ended`]) — one this pace started, or one a pass made.
    pub(super) fn worktree_ended(&mut self, key: &str, began: At, weight: Option<Duration>) {
        if self.reading.as_deref() == Some(key) {
            self.reading = None;
        }
        let count = self.worktrees.len();
        let (pace, bounds) = (self.pace, self.bounds);
        if let Some(worktree) = self.worktrees.iter_mut().find(|c| c.key == key) {
            if let Some(weight) = weight {
                worktree.estimate.observe(weight);
            }
            worktree.started = Some(began);
            worktree.held = false;
            worktree.forced = false;
            // Asked for after this read began — the listing moved past it
            // meanwhile: this read answered the asks before it, not that one.
            if worktree.asked.is_some_and(|asked| asked > began) {
                worktree.due = began;
            } else {
                worktree.asked = None;
                worktree.due = began
                    + worktree_target(pace, bounds, worktree.estimate, count)
                        .unwrap_or(bounds.worktree_ceiling);
            }
        }
    }

    /// This tree's wait, planned again under the pace as it stands — from
    /// where its last read began, and never before the rest after it. Not
    /// while a read is under way (its end plans the next), before the first
    /// (nothing to count from), or once asked for at once.
    fn plan_own_again(&mut self) {
        if self.own.running || self.own.asked {
            return;
        }
        if let Some(started) = self.own.started {
            self.own.due = (started + self.own_interval()).max(self.own.rested);
        }
    }

    /// The worktrees' waits, planned again the same way: from where each
    /// last read began. A worktree never read is due already, the one being
    /// read plans its next as it ends, and one asked for at once stays asked.
    fn plan_worktrees_again(&mut self) {
        let count = self.worktrees.len();
        let (pace, bounds) = (self.pace, self.bounds);
        for worktree in &mut self.worktrees {
            if worktree.asked.is_some() || self.reading.as_deref() == Some(worktree.key.as_str()) {
                continue;
            }
            if let Some(started) = worktree.started {
                worktree.due = started
                    + worktree_target(pace, bounds, worktree.estimate, count)
                        .unwrap_or(bounds.worktree_ceiling);
            }
        }
    }

    /// A worktree's read never began (a pass took the worktrees meanwhile):
    /// it stays due, and waits for the end of this tree's next read as a
    /// held worktree does — asked again at once, it would be turned away
    /// again for as long as the pass lasts.
    pub(super) fn worktree_skipped(&mut self, key: &str) {
        if self.reading.as_deref() == Some(key) {
            self.reading = None;
        }
        if let Some(worktree) = self.worktrees.iter_mut().find(|c| c.key == key) {
            worktree.held = true;
        }
    }

    /// A pass over every worktree began or ended; its reads are reported one
    /// by one through [`Self::worktree_ended`].
    pub(super) fn set_passing(&mut self, passing: bool) {
        self.passing = passing;
    }

    fn reads_worktrees(&self) -> bool {
        self.pace != WorktreesPace::Off && !self.worktrees.is_empty()
    }

    fn own_interval(&self) -> Duration {
        let PaceBounds {
            own_floor,
            own_ceiling,
            ..
        } = self.bounds;
        match self.own.estimate.weight() {
            None => own_floor,
            Some(weight) => weight.div_f64(OWN_SHARE).clamp(own_floor, own_ceiling),
        }
    }

    /// When this tree's next read can be expected to end: the read under
    /// way's, or the next one's.
    fn own_free_at(&self, at: At) -> At {
        let weight = self.own.estimate.weight().unwrap_or(Duration::ZERO);
        if self.own.running {
            (self.own.started.unwrap_or(at) + weight).max(at)
        } else {
            self.own.due.max(at) + weight
        }
    }

    /// The worktree to read now, if any: the one longest overdue that is
    /// forced, fits before this tree's next read, or would pass its ceiling
    /// waiting for that read to end. A due worktree that does none of these
    /// is held — a worktree never read among them, since nothing says it
    /// would fit.
    fn next_worktree(&mut self, at: At) -> Option<String> {
        if !self.reads_worktrees() || self.reading.is_some() || self.passing {
            return None;
        }
        // A held worktree waits for this tree's read to end, whatever wakes
        // the pace meanwhile; that end forces it.
        let mut due: Vec<usize> = (0..self.worktrees.len())
            .filter(|&i| self.worktrees[i].due <= at && !self.worktrees[i].held)
            .collect();
        due.sort_by_key(|&i| (!self.worktrees[i].forced, self.worktrees[i].due));
        let free_at = self.own_free_at(at);
        let next_own = (!self.own.running).then_some(self.own.due);
        let count = self.worktrees.len();
        for i in due {
            let worktree = &self.worktrees[i];
            if worktree.forced {
                return Some(worktree.key.clone());
            }
            let fits = match (worktree.estimate.weight(), next_own) {
                (Some(weight), Some(next)) => at + weight <= next,
                _ => false,
            };
            let past_its_ceiling = worktree.started.is_some_and(|started| {
                let ceiling = self.bounds.worktree_ceiling;
                let ceiling = worktree_target(self.pace, self.bounds, worktree.estimate, count)
                    .unwrap_or(ceiling)
                    .max(ceiling);
                free_at > started + ceiling
            });
            if fits || past_its_ceiling {
                return Some(worktree.key.clone());
            }
            self.worktrees[i].held = true;
        }
        None
    }
}

/// The interval one worktree is read at; `None` where the worktrees are off.
fn worktree_target(
    pace: WorktreesPace,
    bounds: PaceBounds,
    estimate: Estimate,
    count: usize,
) -> Option<Duration> {
    match pace {
        WorktreesPace::Off => None,
        WorktreesPace::Fixed(every) => Some(every),
        WorktreesPace::Auto => Some(match estimate.weight() {
            None => bounds.worktree_floor,
            Some(weight) => {
                let count = u32::try_from(count.max(1)).unwrap_or(u32::MAX);
                (weight * count)
                    .div_f64(WORKTREE_SHARE)
                    .clamp(bounds.worktree_floor, bounds.worktree_ceiling)
            }
        }),
    }
}
