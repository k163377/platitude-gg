//! The font database's population, weighed on its own.
//!
//! The first glyph the UI family lacks makes Qt build a fallback list,
//! and building one populates every family the database knows — on
//! Windows a DirectWrite face over each file, kept for the life of the
//! process. Tens of MB once, whichever glyph asked (an emoji, a
//! simplified-Chinese ideograph), and nothing the product can decline:
//! the fallback-family and emoji-family APIs only order that list, the
//! QML `font` type has no `families`, and the fonts are the machine's.
//! The record reads the budget line net of it
//! (ci/baseline/perf-windows-x64.md §判定), so it is weighed, and
//! weighed where the sampler can see it. The corpus asks during
//! the scroll, where the walk's bytes and the bench's arrive in the
//! same ticks; the calibration run asks before
//! `perf_done` instead, idle either side, and says when ([`mark`] —
//! `WindowPerfDriver` under `PGG_PERF_FONT_WALK=1`). Offscreen there is
//! no walk to weigh: that platform's FreeType database holds no fonts,
//! which is why `verify-ui` cannot see any of this.

use super::Options;

/// What the sampler read at one tick, on the parent's clock.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Tick {
    pub(super) at_us: u64,
    pub(super) working_set: u64,
    pub(super) private: u64,
}

/// The three lines the run says around the walk, on the parent's clock
/// as they arrived, and what the sampler had last seen on either side.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct FontWalk {
    pub(super) begin_us: Option<u64>,
    pub(super) done_us: Option<u64>,
    pub(super) settled_us: Option<u64>,
    /// The last tick before the app said it was about to ask.
    pub(super) before: Option<Tick>,
    /// The last tick before the app said it had settled after asking.
    pub(super) after: Option<Tick>,
}

/// Which line of the three, in the order they come.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Mark {
    Begin,
    Done,
    Settled,
}

/// Which of the three lines `line` is, if any.
pub(super) fn mark(line: &str) -> Option<Mark> {
    if line.contains("perf_font_walk_begin") {
        Some(Mark::Begin)
    } else if line.contains("perf_font_walk_done") {
        Some(Mark::Done)
    } else if line.contains("perf_font_walk_settled") {
        Some(Mark::Settled)
    } else {
        None
    }
}

impl FontWalk {
    /// The first time each line arrived: a run says each once, and a
    /// second saying would be a run that walked twice, which the first
    /// mark is the honest end of.
    pub(super) fn note(&mut self, mark: Mark, at_us: u64) {
        let slot = match mark {
            Mark::Begin => &mut self.begin_us,
            Mark::Done => &mut self.done_us,
            Mark::Settled => &mut self.settled_us,
        };
        slot.get_or_insert(at_us);
    }

    /// Whether all three lines arrived, in their order.
    pub(super) fn said(&self) -> bool {
        matches!(
            (self.begin_us, self.done_us, self.settled_us),
            (Some(begin), Some(done), Some(settled)) if begin <= done && done <= settled
        )
    }

    /// The walk read against the sampler's series: the last tick before
    /// the app said it was about to ask, and the last before it said it
    /// had settled — both taken of a process that had been idle for the
    /// app's own settle, which is what makes their difference the walk.
    pub(super) fn weigh(mut self, history: &[Tick]) -> Self {
        self.before = self.begin_us.and_then(|at| last_before(history, at));
        self.after = self.settled_us.and_then(|at| last_before(history, at));
        self
    }

    /// Whether there is a number here: said in order, and sampled on
    /// both sides.
    pub(super) fn weighed(&self) -> bool {
        self.said() && self.before.is_some() && self.after.is_some()
    }

    /// What the walk added to the working set, signed: a walk that had
    /// already been paid before the app asked reads as noise around zero.
    pub(super) fn working_set(&self) -> Option<i64> {
        Some(delta(self.after?.working_set, self.before?.working_set))
    }

    pub(super) fn private(&self) -> Option<i64> {
        Some(delta(self.after?.private, self.before?.private))
    }

    /// What the budget line is read net of: the working set the walk
    /// added, or nothing where it added nothing — a walk paid before
    /// the app asked is in every reading already and cannot be taken off
    /// what was never measured.
    pub(super) fn charge(&self) -> u64 {
        self.working_set()
            .and_then(|added| u64::try_from(added).ok())
            .unwrap_or(0)
    }

    /// The walk as one run's line says it.
    pub(super) fn line(&self) -> String {
        match (self.working_set(), self.private(), self.before, self.after) {
            (Some(ws), Some(private), Some(before), Some(after)) => format!(
                " font-walk: ws {} private {} (before {:.1} → after {:.1}MB)",
                signed_mb(ws),
                signed_mb(private),
                super::mb(before.working_set),
                super::mb(after.working_set)
            ),
            _ => " font-walk: not weighed".to_string(),
        }
    }
}

fn delta(after: u64, before: u64) -> i64 {
    i64::try_from(after).unwrap_or(i64::MAX) - i64::try_from(before).unwrap_or(i64::MAX)
}

/// `+12.3MB` / `-0.4MB`: a difference of working sets, which unlike a
/// working set has a sign worth printing.
pub(super) fn signed_mb(bytes: i64) -> String {
    format!("{:+.1}MB", bytes as f64 / (1024.0 * 1024.0))
}

fn last_before(history: &[Tick], at_us: u64) -> Option<Tick> {
    history
        .iter()
        .rev()
        .find(|tick| tick.at_us <= at_us)
        .copied()
}

/// The calibration run's options: the same build, window, screen and
/// repository as the runs it is read beside, driven no further than a
/// graph, asked to walk, and asked nothing else — no settle, no
/// attribution, no breakdown, no frame trace — because its one number
/// is the difference between two idle ticks of its own.
pub(super) fn calibration(opts: &Options) -> Options {
    Options {
        cases: Vec::new(),
        cycles: 1,
        completion: "raw".into(),
        diff_scroll: false,
        selection: "none".into(),
        select: false,
        scroll: false,
        oid: String::new(),
        file: String::new(),
        font_walk: true,
        attribute: false,
        settle_ms: 0,
        breakdown: false,
        trace_frames: false,
        ..opts.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::{FontWalk, Mark, Tick, calibration, mark};

    fn tick(at_us: u64, working_set: u64, private: u64) -> Tick {
        Tick {
            at_us,
            working_set,
            private,
        }
    }

    /// Ticks every 100ms; the app says begin at 2.05s and settled at
    /// 6.05s, so the walk is read between the 2.0s tick and the 6.0s one.
    fn series() -> Vec<Tick> {
        (0..=70)
            .map(|n| {
                let at = n * 100_000;
                if at < 2_100_000 {
                    tick(at, 200 << 20, 150 << 20)
                } else {
                    tick(at, 255 << 20, 203 << 20)
                }
            })
            .collect()
    }

    fn said() -> FontWalk {
        let mut walk = FontWalk::default();
        walk.note(Mark::Begin, 2_050_000);
        walk.note(Mark::Done, 2_250_000);
        walk.note(Mark::Settled, 6_050_000);
        walk
    }

    #[test]
    fn the_three_lines_are_told_apart_and_anything_else_is_not_one() {
        assert_eq!(
            mark("INFO bench: perf_font_walk_begin clock_ms=2050.1"),
            Some(Mark::Begin)
        );
        assert_eq!(
            mark("INFO bench: perf_font_walk_done clock_ms=2251.7 width=14"),
            Some(Mark::Done)
        );
        assert_eq!(
            mark("INFO bench: perf_font_walk_settled clock_ms=6050.2"),
            Some(Mark::Settled)
        );
        assert_eq!(mark("INFO bench: perf_done open=true"), None);
    }

    /// The walk is the difference of the last idle tick on either side
    /// of the app's own lines, and its charge is that difference.
    #[test]
    fn the_walk_is_read_between_the_last_ticks_before_its_first_and_last_line() {
        let walk = said().weigh(&series());
        assert!(walk.weighed());
        assert_eq!(walk.before.map(|t| t.at_us), Some(2_000_000));
        assert_eq!(walk.after.map(|t| t.at_us), Some(6_000_000));
        assert_eq!(walk.working_set(), Some(55 << 20));
        assert_eq!(walk.private(), Some(53 << 20));
        assert_eq!(walk.charge(), 55 << 20);
        assert_eq!(
            walk.line(),
            " font-walk: ws +55.0MB private +53.0MB (before 200.0 → after 255.0MB)"
        );
    }

    /// Three lines out of order, or a line missing, is no walk; and a
    /// line said twice keeps its first time — the run walked once.
    #[test]
    fn a_walk_is_said_once_and_in_order() {
        let mut walk = FontWalk::default();
        walk.note(Mark::Settled, 6_000_000);
        walk.note(Mark::Begin, 2_000_000);
        walk.note(Mark::Done, 2_200_000);
        assert!(
            walk.said(),
            "the order they are said in is the order they are read in"
        );
        walk.note(Mark::Begin, 9_000_000);
        assert_eq!(walk.begin_us, Some(2_000_000));
        let mut unfinished = FontWalk::default();
        unfinished.note(Mark::Begin, 2_000_000);
        unfinished.note(Mark::Done, 2_200_000);
        assert!(!unfinished.said());
        assert!(!unfinished.weigh(&series()).weighed());
        let mut backwards = FontWalk::default();
        backwards.note(Mark::Begin, 6_000_000);
        backwards.note(Mark::Done, 6_100_000);
        backwards.note(Mark::Settled, 2_000_000);
        assert!(!backwards.said());
    }

    /// No tick before the first line is no reading — the sampler was
    /// not watching — and a walk that added nothing charges nothing:
    /// it was paid before the app asked, and is in every reading already.
    #[test]
    fn a_walk_nobody_sampled_or_that_added_nothing_charges_nothing() {
        let late: Vec<Tick> = series()
            .into_iter()
            .filter(|t| t.at_us > 2_050_000)
            .collect();
        let walk = said().weigh(&late);
        assert!(!walk.weighed());
        assert_eq!(walk.working_set(), None);
        assert_eq!(walk.charge(), 0);
        assert_eq!(walk.line(), " font-walk: not weighed");
        let flat: Vec<Tick> = (0..=70)
            .map(|n| tick(n * 100_000, 255 << 20, 203 << 20))
            .collect();
        let paid = said().weigh(&flat);
        assert!(paid.weighed());
        assert_eq!(paid.working_set(), Some(0));
        assert_eq!(paid.charge(), 0);
        let noisy: Vec<Tick> = (0..=70)
            .map(|n| tick(n * 100_000, if n < 21 { 256 << 20 } else { 255 << 20 }, 0))
            .collect();
        assert_eq!(said().weigh(&noisy).working_set(), Some(-(1 << 20)));
        assert_eq!(said().weigh(&noisy).charge(), 0);
    }

    /// The calibration run keeps everything that places the window and
    /// names the repository, and drops everything that would make it a
    /// measurement of anything but the walk.
    #[test]
    fn the_calibration_run_drives_no_further_than_a_graph() {
        let asked = crate::perf::options::parse(
            &[
                "--repo",
                "C:/r",
                "--settle-ms",
                "8000",
                "--select-oid",
                "0123456789012345678901234567890123456789",
                "--screen",
                "\\\\.\\DISPLAY2",
                "--runs",
                "5",
            ]
            .iter()
            .map(|w| (*w).to_string())
            .collect::<Vec<_>>(),
        )
        .expect("a judgment run's options");
        let run = calibration(&asked);
        assert!(run.font_walk && !run.select && !run.scroll);
        assert_eq!(run.selection, "none");
        assert!(run.oid.is_empty() && run.file.is_empty());
        assert_eq!(run.settle_ms, 0);
        assert!(!run.attribute && !run.breakdown && !run.trace_frames);
        assert_eq!(run.screen, asked.screen);
        assert_eq!(run.repo, asked.repo);
        assert_eq!(run.runs, asked.runs);
        assert!(run.harness && run.open);
    }
}
