//! Turning the readings of a run into the lines the record is written
//! from: megabytes, the middle and the spread over the kept runs, and the
//! block printed at the end of `perf`.
//!
//! **The middle beside the spread.** A range over N runs can only grow
//! with N — one unlucky run widens it forever — so a range on its own
//! cannot say whether a change moved anything. The median is what holds
//! still while the machine misbehaves, and both are printed because the
//! budget is read against the worst run and a comparison is read against
//! the middle one.

use super::corpus::Corpus;
use super::display::Screen;
use super::{Options, Reading};

pub(super) fn mb(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

pub(super) struct Context<'a> {
    pub(super) screen: Option<&'a Screen>,
    pub(super) corpus: Option<&'a Corpus>,
    pub(super) retries: u32,
}

pub(super) fn report(opts: &Options, kept: &[Reading], context: &Context<'_>) {
    if kept.is_empty() {
        return;
    }
    memory(opts, kept, context);
    timings(kept);
    scroll(kept, context);
    interaction(kept);
    host(kept);
    tail(kept);
}

/// What the run weighed, and the conditions the weight is only readable
/// under: which build, which repository, which screen.
fn memory(opts: &Options, kept: &[Reading], context: &Context<'_>) {
    let ws: Vec<f64> = kept.iter().map(|r| mb(r.peak_working_set)).collect();
    let private: Vec<f64> = kept.iter().map(|r| mb(r.peak_private)).collect();
    println!("\n== {} ==", opts.label);
    println!("  build       : {}", opts.features());
    if let Some(corpus) = context.corpus {
        println!("  corpus      : {corpus}");
    }
    if let Some(screen) = context.screen {
        println!(
            "  screen      : {} at {}Hz, {}x{} from ({}, {})",
            screen.name, screen.hz, screen.width, screen.height, screen.x, screen.y
        );
    }
    println!("  working set : {}", spread(&ws));
    println!("  private     : {}", spread(&private));
    if cfg!(target_os = "linux") {
        println!("  Linux private column is VmData, not Windows committed Private Bytes.");
    }
    println!("  memory units: MiB (1024 * 1024 bytes)");
    diagnostic_notes(opts, context);
    let settled: Vec<f64> = kept
        .iter()
        .filter(|r| r.settled_working_set > 0)
        .map(|r| mb(r.settled_working_set))
        .collect();
    if !settled.is_empty() {
        println!(
            "  settled     : {} (working set, after {}ms idle)",
            spread(&settled),
            opts.settle_ms
        );
    }
}

/// How long a person waited for a frame with the graph in it, and how
/// much of that was the walk.
fn timings(kept: &[Reading]) {
    let startups: Vec<f64> = kept.iter().filter_map(|r| r.startup_ms).map(f).collect();
    if !startups.is_empty() {
        println!(
            "  startup     : {} ms (to a visible graph frame)",
            spread(&startups)
        );
    }
    let firsts: Vec<f64> = kept
        .iter()
        .filter_map(|r| r.first_chunk_ms)
        .map(f)
        .collect();
    if !firsts.is_empty() {
        println!("  of which walk: {} ms", spread(&firsts));
    }
}

/// The scroll bench, read three ways: the raw rate, that rate against
/// what the screen could have delivered, and the frames a person would
/// have seen as a stutter whatever the screen was.
fn scroll(kept: &[Reading], context: &Context<'_>) {
    let fps: Vec<f64> = kept.iter().filter_map(|r| r.fps).collect();
    if !fps.is_empty() {
        println!(
            "  scroll      : {} fps (GUI-delivered frameSwapped)",
            spread(&fps)
        );
        // What the screen could have delivered is the only thing that
        // makes two screens comparable: 176fps on a 180Hz monitor and
        // 98fps on a 100Hz one are the same application.
        if let Some(hz) = context.screen.map(|s| s.hz).filter(|hz| *hz > 0) {
            let share: Vec<f64> = fps.iter().map(|v| v * 100.0 / f64::from(hz)).collect();
            println!("  of its screen: {}% of {hz}Hz", spread(&share));
        }
    }
    let stutters: Vec<f64> = kept.iter().filter_map(|r| r.over_16_ms).map(g).collect();
    if !stutters.is_empty() {
        println!(
            "  over 16.7ms : {} frames (a stutter at any refresh rate)",
            spread(&stutters)
        );
    }
    for (name, values) in [
        (
            "p95",
            kept.iter()
                .filter_map(|r| r.frame_p95_ms)
                .collect::<Vec<_>>(),
        ),
        ("p99", kept.iter().filter_map(|r| r.frame_p99_ms).collect()),
        ("max", kept.iter().filter_map(|r| r.frame_max_ms).collect()),
    ] {
        if !values.is_empty() {
            println!("  frame {name:>3}   : {} ms", spread(&values));
        }
    }
}

/// One row selection and one diff, timed from the click to the frame
/// that answered it.
fn interaction(kept: &[Reading]) {
    let details: Vec<f64> = kept
        .iter()
        .flat_map(|r| r.details_ms.clone())
        .map(f)
        .collect();
    if !details.is_empty() {
        println!("  details data: {} ms (request to drain)", spread(&details));
    }
    let rendered: Vec<f64> = kept
        .iter()
        .flat_map(|r| r.details_frame_ms.iter().copied())
        .collect();
    if !rendered.is_empty() {
        println!(
            "  details frame: {} ms (handler to frame; excludes OS input delivery)",
            spread(&rendered)
        );
    }
    let diff: Vec<f64> = kept
        .iter()
        .flat_map(|r| r.diff_frame_ms.iter().copied())
        .collect();
    if !diff.is_empty() {
        println!(
            "  diff frame  : {} ms (raw diff; highlighting may follow)",
            spread(&diff)
        );
    }
}

/// What the run was of, in the three ways a later reader will need to
/// know it was the same run: the commit, the graphics device, the heap.
fn tail(kept: &[Reading]) {
    if let Some(oid) = kept.iter().find_map(|r| r.selected_oid.clone()) {
        println!("  selected    : {oid}");
    }
    if let Some(lines) = kept.iter().map(|r| &r.graphics).find(|g| !g.is_empty()) {
        println!("  graphics    : {}", lines.join(" | "));
    }
    if let Some(line) = kept
        .iter()
        .max_by_key(|r| r.breakdown_live)
        .and_then(|r| r.breakdown.clone())
    {
        println!("\n  breakdown at the largest Rust heap of the kept runs:\n    {line}");
    }
}

/// What the machine was doing under the kept runs. Printed even when
/// every run passed, because "the machine was quiet" is a condition of
/// the numbers above and belongs in the record beside them.
fn host(kept: &[Reading]) {
    let busy: Vec<f64> = kept.iter().map(|r| r.conditions.busy_percent).collect();
    let foreign: Vec<f64> = kept.iter().map(|r| r.conditions.foreign_percent).collect();
    let peak: Vec<f64> = kept
        .iter()
        .map(|r| r.conditions.peak_foreign_percent)
        .collect();
    if busy.iter().all(|v| *v == 0.0) && foreign.iter().all(|v| *v == 0.0) {
        return;
    }
    println!(
        "  machine     : {}% busy, {}% not this process (worst tick {}%)",
        spread(&busy),
        spread(&foreign),
        spread(&peak)
    );
    let front: Vec<f64> = kept
        .iter()
        .filter_map(|r| r.conditions.foreground_share())
        .map(|share| share * 100.0)
        .collect();
    if !front.is_empty() {
        println!(
            "  window      : in front for {}% of the sampled ticks",
            spread(&front)
        );
    }
}

fn diagnostic_notes(opts: &Options, context: &Context<'_>) {
    if opts.trace_frames {
        println!("  diagnostic frame trace enabled; do not use this run for budget acceptance.");
    }
    if context.retries > 0 {
        println!(
            "  {} run(s) were taken again: the machine was not quiet enough the first time.",
            context.retries
        );
    }
    if context.screen.is_none() && opts.open && opts.scroll {
        println!(
            "  the window was placed by the platform, so its screen and refresh are not pinned."
        );
    }
    if opts.open && opts.scroll {
        println!(
            "  display snapshots do not detect a mode change that was reverted during the run."
        );
    }
}

fn f(v: u64) -> f64 {
    v as f64
}

fn g(v: usize) -> f64 {
    v as f64
}

/// `min–median–max` over the readings, or the single value when they
/// agree. The middle one is printed because a range only ever widens.
fn spread(values: &[f64]) -> String {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let (Some(lo), Some(hi)) = (sorted.first(), sorted.last()) else {
        return "-".into();
    };
    if (hi - lo).abs() < 0.05 {
        return format!("{lo:.1}");
    }
    if sorted.len() < 3 {
        return format!("{lo:.1}–{hi:.1}");
    }
    format!("{lo:.1}–{:.1}–{hi:.1}", median(&sorted))
}

/// The middle of an already sorted list, averaging the two middles of an
/// even one.
fn median(sorted: &[f64]) -> f64 {
    let n = sorted.len();
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::{median, spread};

    #[test]
    fn a_spread_of_one_value_is_printed_once() {
        assert_eq!(spread(&[3.0, 3.0]), "3.0");
        assert_eq!(spread(&[]), "-");
    }

    /// Two runs have no middle worth printing; three do, and it is the
    /// one number a fourth unlucky run cannot move far.
    #[test]
    fn the_middle_appears_once_there_are_three_runs() {
        assert_eq!(spread(&[3.0, 5.0]), "3.0–5.0");
        assert_eq!(spread(&[5.0, 3.0, 4.0]), "3.0–4.0–5.0");
        assert_eq!(spread(&[3.0, 4.0, 5.0, 40.0]), "3.0–4.5–40.0");
    }

    #[test]
    fn the_middle_of_an_even_list_is_between_its_two_middles() {
        assert!((median(&[1.0, 2.0, 3.0, 4.0]) - 2.5).abs() < f64::EPSILON);
        assert!((median(&[1.0, 2.0, 3.0]) - 2.0).abs() < f64::EPSILON);
    }
}
