//! Waiting, in this runner: the budget every wait is held to, the pace a
//! look is taken at, and what a wait that ran out says.
//!
//! Nothing here establishes correctness by elapsed
//! time (.claude/rules/core.md §非同期・並行テスト): a wait
//! ends on what it was waiting for — a lock granted, a
//! process gone, a line said — and the budget is the
//! diagnostic under it, so that a thing that stopped is
//! named. A budget that runs out is a failure, worded
//! with what was waited for, at which stage, and what
//! was last seen ([`Expired`]). The one wait whose end
//! *is* the answer is spelled out as such ([`stood`]).
//!
//! **The suite's budget is one budget** ([`Budget::SUITE`]), as it is in
//! the core crate's `support::wait`: a silence budget every sign of
//! progress renews, and an overall cap only a livelock reaches. A test
//! given a budget of its own is handed a head start on noticing a hang —
//! or, the other way, goes red for the load on the machine — so tests
//! take this one. The runner's verbs bring ceilings of their own, since
//! what they wait out is other work on the machine (a measurement's hold,
//! a gate's lanes, a step's cargo) and the reason for each number stands
//! beside it where it is declared; what they take from here is the
//! mechanism — the clock, the pace, and the words.
//!
//! **The looks stay where they are.** A lock another process holds and a
//! process that has not exited are announced by nothing this runner can
//! block on, so a wait on them is a loop of looks; what this makes of the
//! loop is that its deadline and its pace are not spelled out at the seat.
//!
//! This file is where the clock is read on purpose, and `cargo xtask
//! waits` reads neither it nor its twin in the core crate. It names
//! nothing else in the runner: the gate's own tests read it in by path
//! (`tests/gate`), and `crate::` is another crate there.

use std::fmt;
use std::time::{Duration, Instant};

/// How long a test's wait puts up with nothing changing. Every sign of
/// progress renews it, so what spends it is silence — not the wait
/// taking a while. The core suite's number, for the core suite's reason:
/// under `cargo test --workspace` one round trip inflates many times over
/// its solo time, and a shorter budget goes red for the load.
#[cfg(test)]
const QUIET_BUDGET: Duration = Duration::from_secs(120);

/// The whole of a test's wait, as a backstop under [`QUIET_BUDGET`]:
/// something that keeps changing without ever getting to the answer
/// renews the silence budget forever, and only a livelock reaches this
/// one.
#[cfg(test)]
const OVERALL_BUDGET: Duration = Duration::from_secs(900);

/// How long between looks at something the operating system announces
/// nothing about — a lock another process holds, a process that has not
/// exited, a file that will not rename yet. A look is cheap (a
/// `try_lock`, a `try_wait`, a `stat`), so the pace is set by how soon a
/// wait should notice and not by what a look costs; a look that costs
/// more sets its own pace where it is taken.
pub(crate) const LOOK_AGAIN: Duration = Duration::from_millis(100);

/// How long between tries at a name a reader holds for the microseconds
/// of a sweep — a lock file being taken down under its own lock. The
/// other side is a call or two away, so the tries come close together.
pub(crate) const TRY_AGAIN: Duration = Duration::from_millis(5);

/// What a wait is held to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Budget {
    /// How long nothing new may be seen for ([`Wait::saw`]); None where
    /// the wait has nothing to watch change, and the whole is the budget.
    quiet: Option<Duration>,
    /// The whole of the wait.
    whole: Duration,
}

impl Budget {
    /// The suite's: what every test waits with, and only a test — the
    /// runner's verbs bring ceilings of their own.
    #[cfg(test)]
    pub(crate) const SUITE: Budget = Budget {
        quiet: Some(QUIET_BUDGET),
        whole: OVERALL_BUDGET,
    };

    /// A ceiling on the whole of a wait that sees nothing change on the
    /// way — a lock that is held until it is not.
    pub(crate) const fn whole(whole: Duration) -> Budget {
        Budget { quiet: None, whole }
    }

    /// A silence budget under a ceiling, for a wait that sees progress: a
    /// step's log growing, a set of builds shrinking.
    pub(crate) const fn of(quiet: Duration, whole: Duration) -> Budget {
        Budget {
            quiet: Some(quiet),
            whole,
        }
    }
}

/// A wait under way: who is waiting, what it is held to, and what it has
/// seen. Made where the waiting starts; every look the loop takes ends in
/// [`Wait::look_again`], which is where the budget is checked and the
/// pace is paid.
#[derive(Debug)]
pub(crate) struct Wait {
    what: String,
    budget: Budget,
    pace: Duration,
    started: Instant,
    /// When something new was last seen; the start until then.
    renewed: Instant,
    looks: u32,
    seen: Option<String>,
}

impl Wait {
    /// A wait by `what` — the party that waits, or the thing waited on,
    /// as the failure should open — under `budget`, `pace` between looks.
    pub(crate) fn new(what: impl Into<String>, budget: Budget, pace: Duration) -> Self {
        Self::since(Instant::now(), what, budget, pace)
    }

    /// The same, with its clock started at `started`: the instant another
    /// party says the thing began, such as a sampler resuming the process
    /// it times.
    pub(crate) fn since(
        started: Instant,
        what: impl Into<String>,
        budget: Budget,
        pace: Duration,
    ) -> Self {
        Self {
            what: what.into(),
            budget,
            pace,
            started,
            renewed: started,
            looks: 0,
            seen: None,
        }
    }

    /// Notes what a look saw. What is new renews the silence budget; what
    /// is not stays as the last thing seen, for the failure to name.
    pub(crate) fn saw(&mut self, seen: impl fmt::Display) {
        let seen = seen.to_string();
        if self.seen.as_deref() != Some(seen.as_str()) {
            self.renewed = Instant::now();
            self.seen = Some(seen);
        }
    }

    pub(crate) fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    /// How many looks have ended in [`Wait::look_again`]: zero for a wait
    /// answered at the first look, which is no wait at all.
    pub(crate) fn looks(&self) -> u32 {
        self.looks
    }

    /// Time until the budget runs out — what a blocking receive is given,
    /// so that its timeout fires when the budget does.
    fn remaining(&self) -> Duration {
        let whole = self.budget.whole.saturating_sub(self.started.elapsed());
        match self.budget.quiet {
            Some(quiet) => quiet.saturating_sub(self.renewed.elapsed()).min(whole),
            None => whole,
        }
    }

    /// Whether the budget has run out, said as the failure it is. For a
    /// wait that pays no pace of its own (a second deadline read inside
    /// another wait's loop); a loop's own looks end in [`Wait::look_again`].
    pub(crate) fn check(&self, stage: &str) -> Result<(), Expired> {
        let (quiet, whole) = (self.renewed.elapsed(), self.started.elapsed());
        let limit = if whole >= self.budget.whole {
            Limit::Whole(self.budget.whole)
        } else if let Some(budget) = self.budget.quiet.filter(|budget| quiet >= *budget) {
            Limit::Quiet(budget)
        } else {
            return Ok(());
        };
        Err(self.expired(stage, limit))
    }

    /// Ends a look that found nothing: the budget is checked, and the
    /// pace is paid before the next — no longer than the budget has left,
    /// so a wait is named the moment it runs out.
    pub(crate) fn look_again(&mut self, stage: &str) -> Result<(), Expired> {
        self.looks += 1;
        self.check(stage)?;
        std::thread::sleep(self.pace.min(self.remaining()));
        Ok(())
    }

    /// The failure, worded now: who waited for what, which limit was met,
    /// how long it took and how many looks, and what was last seen.
    fn expired(&self, stage: &str, limit: Limit) -> Expired {
        let what = &self.what;
        let head = match limit {
            Limit::Quiet(budget) => format!(
                "{what}: still waiting for {stage} after {} of nothing new",
                clock(budget)
            ),
            Limit::Whole(ceiling) => format!(
                "{what}: still waiting for {stage} at the {} ceiling",
                clock(ceiling)
            ),
        };
        Expired {
            said: head + &self.trailer(),
        }
    }

    /// The failure of a channel whose other end went away: the answer
    /// never came, and no limit had to be met for that to be known.
    #[cfg(any(windows, test))]
    fn gone(&self, stage: &str) -> Expired {
        Expired {
            said: format!(
                "{}: {stage} never came — the other end went away{}",
                self.what,
                self.trailer()
            ),
        }
    }

    /// What every failure ends with: how long it took, how many looks,
    /// how long nothing new had been seen for, and what was last seen.
    fn trailer(&self) -> String {
        let mut said = format!(" — after {}", clock(self.started.elapsed()));
        if self.looks > 0 {
            said.push_str(&format!(", {} look(s)", self.looks));
        }
        if self.budget.quiet.is_some() {
            said.push_str(&format!(
                ", the last {} with nothing new",
                clock(self.renewed.elapsed())
            ));
        }
        if let Some(seen) = &self.seen {
            said.push_str(&format!("; last seen {seen}"));
        }
        said
    }
}

/// A wait that ran out: who was waiting for what, which limit it met,
/// and what it last saw — worded as it happened ([`Wait::expired`]), for
/// a caller to put its own advice after.
#[derive(Debug)]
pub(crate) struct Expired {
    said: String,
}

/// Which limit a wait met.
#[derive(Debug, Clone, Copy)]
enum Limit {
    /// Nothing new for the whole of the silence budget.
    Quiet(Duration),
    /// The ceiling on the whole.
    Whole(Duration),
}

impl fmt::Display for Expired {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.said)
    }
}

/// A duration as a person reads one in a failure: tenths under a minute,
/// minutes and seconds past it.
fn clock(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{:.1}s", duration.as_secs_f64())
    } else {
        format!("{}m{:02}s", secs / 60, secs % 60)
    }
}

/// Watches something asked to stand — a window, a process — for `stretch`
/// of the clock, `pace` between looks, and answers how long it stood, or
/// what a look said when it ended before then.
///
/// **The one wait here whose end is the answer**, and the shape
/// says so: what is asked is "did it stay", and a thing still
/// there at the end of the stretch has stood for at least that
/// long — a lower bound on the product's own time, which no load
/// can break (§非同期: 実時間で見るのは製品の clock の下限 1 本だけ).
/// This shape is for staying alone; a stretch that passes says
/// nothing about a completion.
pub(crate) fn stood<T>(
    stretch: Duration,
    pace: Duration,
    mut look: impl FnMut() -> Option<T>,
) -> Result<Duration, T> {
    let started = Instant::now();
    loop {
        if let Some(ended) = look() {
            return Err(ended);
        }
        let elapsed = started.elapsed();
        if elapsed >= stretch {
            return Ok(elapsed);
        }
        std::thread::sleep(pace.min(stretch - elapsed));
    }
}

/// Receives on `rx` under `budget`: the answer, or the failure that it
/// never came — the budget run out, or the other end gone. The channel
/// waits in this runner's verbs are the Windows samplers' (`perf::sampler`
/// reads its scripts on a thread of their own); elsewhere the tests'. The
/// cfg follows the callers.
#[cfg(any(windows, test))]
pub(crate) fn receive<T>(
    what: &str,
    stage: &str,
    rx: &std::sync::mpsc::Receiver<T>,
    budget: Budget,
) -> Result<T, Expired> {
    use std::sync::mpsc::RecvTimeoutError::{Disconnected, Timeout};

    let wait = Wait::new(what, budget, Duration::ZERO);
    loop {
        match rx.recv_timeout(wait.remaining()) {
            Ok(answer) => return Ok(answer),
            Err(Timeout) => wait.check(stage)?,
            Err(Disconnected) => return Err(wait.gone(stage)),
        }
    }
}

/// What a test heard from `what` on `rx`, under the suite's budget; a
/// channel that says nothing fails the test by name.
#[cfg(test)]
pub(crate) fn heard<T>(what: &str, stage: &str, rx: &std::sync::mpsc::Receiver<T>) -> T {
    receive(what, stage, rx, Budget::SUITE).unwrap_or_else(|expired| panic!("{expired}"))
}

/// Looks until `answered` is happy with what `look` sees, under the
/// suite's budget, and hands that look back; one that never is fails the
/// test with the last look in the message.
#[cfg(test)]
pub(crate) fn until<T: fmt::Debug>(
    stage: &str,
    mut look: impl FnMut() -> T,
    answered: impl Fn(&T) -> bool,
) -> T {
    let mut wait = Wait::new("the test", Budget::SUITE, LOOK_AGAIN);
    loop {
        let seen = look();
        if answered(&seen) {
            return seen;
        }
        wait.saw(format!("{seen:?}"));
        if let Err(expired) = wait.look_again(stage) {
            panic!("{expired}");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{Budget, Wait, clock, receive, stood};

    /// A wait whose clock started `ago` before now.
    fn aged(ago: Duration, budget: Budget) -> Wait {
        Wait::since(Instant::now() - ago, "the test", budget, Duration::ZERO)
    }

    #[test]
    fn a_ceiling_is_met_by_the_whole_and_a_silence_budget_by_nothing_new() {
        let fresh = aged(
            Duration::ZERO,
            Budget::of(Duration::from_secs(5), Duration::from_secs(60)),
        );
        assert!(fresh.check("an answer").is_ok());
        let quiet = aged(
            Duration::from_secs(10),
            Budget::of(Duration::from_secs(5), Duration::from_secs(60)),
        );
        let expired = quiet
            .check("an answer")
            .expect_err("ten seconds of nothing new");
        assert!(
            expired.to_string().contains("after 5.0s of nothing new"),
            "{expired}"
        );
        let whole = aged(
            Duration::from_secs(70),
            Budget::whole(Duration::from_secs(60)),
        );
        let expired = whole.check("an answer").expect_err("past the ceiling");
        assert!(
            expired.to_string().contains("at the 1m00s ceiling"),
            "{expired}"
        );
    }

    #[test]
    fn something_new_renews_the_silence_budget_and_the_same_thing_again_does_not() {
        let mut wait = aged(
            Duration::from_secs(10),
            Budget::of(Duration::from_secs(5), Duration::from_secs(60)),
        );
        wait.saw("two builds");
        assert!(wait.check("the builds to end").is_ok(), "new news renews");
        wait.renewed = Instant::now() - Duration::from_secs(10);
        wait.saw("two builds");
        assert!(
            wait.check("the builds to end").is_err(),
            "the same news is silence"
        );
        wait.saw("one build");
        assert!(wait.check("the builds to end").is_ok());
    }

    #[test]
    fn a_failure_names_who_waited_for_what_and_what_was_last_seen() {
        let mut wait = aged(
            Duration::from_secs(70),
            Budget::whole(Duration::from_secs(60)),
        );
        wait.saw("host-0 held, host-1 held");
        let said = wait
            .look_again("a free lane")
            .expect_err("past the ceiling")
            .to_string();
        assert!(
            said.starts_with("the test: still waiting for a free lane at the 1m00s ceiling"),
            "{said}"
        );
        assert!(said.contains("1 look(s)"), "{said}");
        assert!(
            said.ends_with("; last seen host-0 held, host-1 held"),
            "{said}"
        );
        assert!(
            !said.contains("nothing new"),
            "a whole-only budget watched nothing: {said}"
        );
        let quiet = aged(Duration::from_secs(130), Budget::SUITE);
        let expired = quiet.check("a look").expect_err("no news for the budget");
        assert!(
            expired.to_string().contains("after 2m00s of nothing new"),
            "{expired}"
        );
        assert!(
            expired
                .to_string()
                .contains(", the last 2m10s with nothing new"),
            "{expired}"
        );
    }

    #[test]
    fn a_stretch_stood_is_answered_by_how_long_and_one_cut_short_by_what_cut_it() {
        assert_eq!(
            stood(Duration::from_secs(1), Duration::ZERO, || Some(7)),
            Err(7)
        );
        let stood_for =
            stood(Duration::ZERO, Duration::ZERO, || None::<()>).expect("nothing ended it");
        assert!(stood_for >= Duration::ZERO);
    }

    #[test]
    fn a_receive_is_answered_by_the_channel_or_names_a_budget_or_a_sender_gone() {
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(3).expect("a receiver");
        assert_eq!(
            receive("the test", "a number", &rx, Budget::SUITE).expect("sent already"),
            3
        );
        let spent = receive("the test", "a number", &rx, Budget::whole(Duration::ZERO))
            .expect_err("nothing sent, no budget");
        assert!(
            spent
                .to_string()
                .starts_with("the test: still waiting for a number at the 0.0s ceiling"),
            "{spent}"
        );
        drop(tx);
        let gone =
            receive("the test", "a number", &rx, Budget::SUITE).expect_err("the sender is gone");
        assert!(
            gone.to_string()
                .starts_with("the test: a number never came — the other end went away"),
            "{gone}"
        );
    }

    #[test]
    fn a_duration_reads_in_tenths_under_a_minute_and_in_minutes_past_it() {
        assert_eq!(clock(Duration::from_millis(500)), "0.5s");
        assert_eq!(clock(Duration::from_secs(8)), "8.0s");
        assert_eq!(clock(Duration::from_secs(120)), "2m00s");
        assert_eq!(clock(Duration::from_secs(30 * 60 + 5)), "30m05s");
    }
}
