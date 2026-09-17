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
use super::fonts::{FontWalk, signed_mb};
use super::{Options, Reading};

pub(super) fn mb(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

pub(super) struct Context<'a> {
    pub(super) screen: Option<&'a Screen>,
    pub(super) corpus: Option<&'a Corpus>,
    /// What was measured: the commit, and the tree it was built in.
    pub(super) built: &'a super::rig::Built,
    pub(super) retries: u32,
    /// What the calibration run weighed the font walk at, read beside
    /// the kept runs (`perf::fonts`); `None` under `--no-font-walk`
    /// and for the shipped build.
    pub(super) font_walk: Option<&'a FontWalk>,
}

pub(super) fn report(opts: &Options, kept: &[Reading], context: &Context<'_>) {
    if kept.is_empty() {
        return;
    }
    memory(opts, kept, context);
    timings(kept, opts.software);
    scroll(kept, context, opts.software);
    interaction(kept, opts.software);
    super::interactions::report(kept);
    host(kept);
    tail(kept);
    attribution(kept);
}

/// What every frame number is read as under `--software`: the software
/// scene graph's own rate, which reached no screen (`measure::command`).
fn not_the_display(software: bool) -> &'static str {
    if software {
        " — the software scene graph, not the display path"
    } else {
        ""
    }
}

/// What the settled process was holding, by kind, read from outside it
/// (`perf::attribution`). Last, because it is the reading of the
/// reading: which side of the process the working set above is on.
fn attribution(kept: &[Reading]) {
    for line in attribution_lines(kept) {
        println!("{line}");
    }
}

/// The lines of that block, over the kept runs that have an attribution;
/// none where none has. Counts are spread as counts, bytes as MiB.
fn attribution_lines(kept: &[Reading]) -> Vec<String> {
    let taken: Vec<&super::attribution::Attribution> =
        kept.iter().filter_map(|r| r.attribution.as_ref()).collect();
    if taken.is_empty() {
        return Vec::new();
    }
    let of = |pick: fn(&super::attribution::Attribution) -> u64| -> Vec<f64> {
        taken.iter().map(|a| mb(pick(a))).collect()
    };
    let mut lines = vec![
        format!(
            "\n  resident    : private {} | mapped {} | image {} MiB (settled, read from outside \
             the process)",
            spread(&of(|a| a.resident_private)),
            spread(&of(|a| a.resident_mapped)),
            spread(&of(|a| a.resident_image))
        ),
        format!(
            "  of which fonts: {} MiB in {} files (.ttf/.ttc/.otf, StaticCache.dat, FontCache)",
            spread(&of(|a| a.fonts_resident)),
            count_spread(&taken.iter().map(|a| a.fonts_files).collect::<Vec<_>>())
        ),
    ];
    let heaps: Vec<&super::attribution::Heap> =
        taken.iter().filter_map(|a| a.heap.as_ref()).collect();
    if heaps.is_empty() {
        lines.push(format!(
            "  process heap: not walked — {}",
            taken
                .iter()
                .find_map(|a| a.heap_error.as_deref())
                .unwrap_or("the script gave no heap line")
        ));
    } else {
        let heap = |pick: fn(&super::attribution::Heap) -> u64| -> Vec<u64> {
            heaps.iter().map(|h| pick(h)).collect()
        };
        lines.push(format!(
            "  process heap: busy {} MiB in {} blocks, free {} MiB in {} blocks{}",
            spread(&heap(|h| h.busy).iter().map(|v| mb(*v)).collect::<Vec<_>>()),
            count_spread(&heap(|h| h.busy_blocks)),
            spread(&heap(|h| h.free).iter().map(|v| mb(*v)).collect::<Vec<_>>()),
            count_spread(&heap(|h| h.free_blocks)),
            if heaps.len() < taken.len() {
                " (not every kept run's walk answered)"
            } else {
                ""
            }
        ));
    }
    lines.push(
        "  attribution : attribution.txt in each run — private by allocation size class, resident \
         by file, heap blocks by size class"
            .to_string(),
    );
    lines
}

/// `min–max` over counts, or the one count when they agree: a block
/// count has no tenths, and no middle worth a third number.
fn count_spread(values: &[u64]) -> String {
    match (values.iter().min(), values.iter().max()) {
        (Some(lo), Some(hi)) if lo == hi => lo.to_string(),
        (Some(lo), Some(hi)) => format!("{lo}–{hi}"),
        _ => "-".into(),
    }
}

/// What the run weighed, and the conditions the weight is only readable
/// under: which build, which repository, which screen.
fn memory(opts: &Options, kept: &[Reading], context: &Context<'_>) {
    let ws: Vec<f64> = kept.iter().map(|r| mb(r.peak_working_set)).collect();
    let private: Vec<f64> = kept.iter().map(|r| mb(r.peak_private)).collect();
    println!("\n== {} ==", opts.label);
    println!("  build       : {}", opts.features());
    println!(
        "  cache       : {} | platform: {}",
        opts.cache,
        super::experiment::platform()
    );
    println!(
        "  commit      : {} built in {}",
        context.built.short(),
        context.built.tree.display()
    );
    if let Some(corpus) = context.corpus {
        println!("  corpus      : {corpus}");
    }
    if let Some(screen) = context.screen {
        println!(
            "  screen      : {} at {}Hz, {}x{} from ({}, {})",
            screen.name, screen.hz, screen.width, screen.height, screen.x, screen.y
        );
    }
    if opts.software {
        println!(
            "  renderer    : the software scene graph (--software), whose frames need no display; \
             read the working set beside a D3D reading of the same commit, not against the \
             budget line"
        );
    }
    println!("  working set : {}", spread(&ws));
    let os_peaks: Vec<_> = kept
        .iter()
        .filter_map(|r| r.os_peak_working_set)
        .map(mb)
        .collect();
    if !os_peaks.is_empty() {
        println!(
            "  OS resident peak: {} MiB (process lifetime high-water mark; diagnostic, no font subtraction)",
            spread(&os_peaks)
        );
        let missed: Vec<_> = kept
            .iter()
            .filter_map(|r| {
                r.os_peak_working_set
                    .map(|p| mb(p.saturating_sub(r.peak_working_set)))
            })
            .collect();
        println!(
            "  above sampled peak: {} MiB (the high-water mark does not locate the allocation)",
            spread(&missed)
        );
    }
    println!("  private     : {}", spread(&private));
    if cfg!(target_os = "linux") {
        println!("  Linux private column is VmData; Windows is committed Private Bytes.");
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
    if let Some(walk) = context.font_walk {
        for line in font_walk_lines(kept, walk, opts) {
            println!("{line}");
        }
    }
}

/// Whether the runs being reported walked the font database themselves.
///
/// **The corpus asks during the scroll** ([`super::fonts`]): one subject
/// in a thousand opens with an emoji, so the first screen carries none
/// and the pass over the window's rows carries two. A run that never
/// scrolled — the staged table's `--no-open`, `--no-select --no-scroll`,
/// `--no-scroll` — never paid the charge the calibration weighed, and
/// subtracting it there would take tens of MB off a number nobody spent.
///
/// Conservative on purpose, and in the direction that cannot mislead: a
/// run named onto an emoji row with `--select-oid` does pay without
/// scrolling, and is reported gross. Reading a gross number as gross is
/// the reader's own arithmetic; reading a number as net when nothing was
/// taken off is the report lying about the budget line.
fn paid_the_walk(opts: &Options) -> bool {
    opts.open && opts.scroll
}

/// The flag that named this run's shape, for the line that says why
/// there is no net — the reader asked for the shape and gets it named
/// back.
fn shape_of(opts: &Options) -> &'static str {
    if !opts.open {
        "--no-open"
    } else {
        "--no-scroll"
    }
}

/// The walk's weight, and the working set net of it — the line the
/// budget is read against (ci/baseline/perf-windows-x64.md §判定). The
/// working set above stays as sampled, walk included, so the two can be
/// read against each other.
fn font_walk_lines(kept: &[Reading], walk: &FontWalk, opts: &Options) -> Vec<String> {
    let settle_ms = opts.settle_ms;
    let (Some(working_set), Some(private)) = (walk.working_set(), walk.private()) else {
        return vec![
            "  font walk   : not weighed — the calibration run said its three lines, but the \
             sampler had no tick on one side of them"
                .to_string(),
        ];
    };
    let charge = walk.charge();
    let unpaid = !paid_the_walk(opts);
    let mut lines = vec![format!(
        "  font walk   : {} working set, {} private — Qt populating its font database for the \
         first glyph the UI family lacks, weighed by the calibration run (run-font-walk){}",
        signed_mb(working_set),
        signed_mb(private),
        if charge == 0 {
            "; nothing to take off — the walk had already been paid before that run asked"
        } else if unpaid {
            "; not taken off below — these runs never reached the rows that ask"
        } else {
            ", and taken off the working set below"
        }
    )];
    if unpaid {
        lines.push(format!(
            "  net         : - (no net line: {} never scrolls the window, so nothing here paid \
             the walk — read the working set above as it stands)",
            shape_of(opts)
        ));
        return lines;
    }
    let net: Vec<f64> = kept
        .iter()
        .map(|r| mb(r.peak_working_set.saturating_sub(charge)))
        .collect();
    lines.push(format!(
        "  net         : {} (working set less the font walk — the line the budget is read against)",
        spread(&net)
    ));
    let settled: Vec<f64> = kept
        .iter()
        .filter(|r| r.settled_working_set > 0)
        .map(|r| mb(r.settled_working_set.saturating_sub(charge)))
        .collect();
    if !settled.is_empty() {
        lines.push(format!(
            "  settled net : {} (settled working set less the font walk, after {settle_ms}ms idle)",
            spread(&settled)
        ));
    }
    lines
}

/// How long a person waited: to a frame with the graph in it, and to the
/// graph data being whole, which is the number both builds answer.
fn timings(kept: &[Reading], software: bool) {
    let startups: Vec<f64> = kept.iter().filter_map(|r| r.startup_ms).map(f).collect();
    if !startups.is_empty() {
        println!(
            "  startup     : {} ms (to a visible graph frame{})",
            spread(&startups),
            not_the_display(software)
        );
    }
    let graphs: Vec<f64> = kept.iter().filter_map(|r| r.graph_ms).map(f).collect();
    if !graphs.is_empty() {
        println!(
            "  to a graph  : {} ms (stream finished; the number both builds answer)",
            spread(&graphs)
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
fn scroll(kept: &[Reading], context: &Context<'_>, software: bool) {
    let fps: Vec<f64> = kept.iter().filter_map(|r| r.fps).collect();
    if !fps.is_empty() {
        println!(
            "  scroll      : {} fps (GUI-delivered frameSwapped{})",
            spread(&fps),
            not_the_display(software)
        );
        // What the screen could have delivered is the only thing that
        // makes two screens comparable: an application that keeps up with
        // its screen reads as a different fps on each one. The software
        // scene graph delivered nothing to the screen, so it has no
        // share of one.
        if let Some(hz) = context
            .screen
            .map(|s| s.hz)
            .filter(|hz| *hz > 0 && !software)
        {
            let share: Vec<f64> = fps.iter().map(|v| v * 100.0 / f64::from(hz)).collect();
            println!("  of its screen: {}% of {hz}Hz", spread(&share));
        }
    }
    let stutters: Vec<f64> = kept.iter().filter_map(|r| r.over_16_ms).map(g).collect();
    if !stutters.is_empty() {
        println!(
            "  over 16.7ms : {} frames (a stutter at any refresh rate{})",
            spread(&stutters),
            not_the_display(software)
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
fn interaction(kept: &[Reading], software: bool) {
    let details: Vec<f64> = kept
        .iter()
        .flat_map(|r| r.details_ms.clone())
        .map(f)
        .collect();
    if !details.is_empty() {
        println!("  details data: {} ms (request to drain)", spread(&details));
    }
    let applied: Vec<f64> = kept
        .iter()
        .flat_map(|r| r.details_applied_ms.clone())
        .map(f)
        .collect();
    if !applied.is_empty() {
        println!(
            "  details rows: {} ms (request to the rows being in the model)",
            spread(&applied)
        );
    }
    let rendered: Vec<f64> = kept
        .iter()
        .flat_map(|r| r.details_frame_ms.iter().copied())
        .collect();
    if !rendered.is_empty() {
        println!(
            "  details frame: {} ms (handler to frame; excludes OS input delivery{})",
            spread(&rendered),
            not_the_display(software)
        );
    }
    let read: Vec<f64> = kept.iter().flat_map(|r| r.diff_ms.clone()).map(f).collect();
    if !read.is_empty() {
        println!("  diff data   : {} ms (request to drain)", spread(&read));
    }
    let laid: Vec<f64> = kept
        .iter()
        .flat_map(|r| r.diff_applied_ms.clone())
        .map(f)
        .collect();
    if !laid.is_empty() {
        println!(
            "  diff rows   : {} ms (request to the rows being in the model)",
            spread(&laid)
        );
    }
    let diff: Vec<f64> = kept
        .iter()
        .flat_map(|r| r.diff_frame_ms.iter().copied())
        .collect();
    if !diff.is_empty() {
        println!(
            "  diff frame  : {} ms (raw diff; highlighting may follow{})",
            spread(&diff),
            not_the_display(software)
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
    // What "not this process" excludes: the git the app runs is its own
    // where the job object counted it, and somebody else's where it
    // could not (`sampler::Sample::job`).
    let children = if kept.iter().all(|r| r.conditions.children_counted) {
        " or its children"
    } else {
        ""
    };
    println!(
        "  machine     : {}% busy, {}% not this process{children} (worst tick {}%)",
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
    // Said only where a tick found the screen off: under a D3D run that
    // is a refusal already, and under --software it is the one thing the
    // record wants to know about the screen, which the reading did not
    // depend on.
    let dark: Vec<f64> = kept
        .iter()
        .filter(|r| r.conditions.dark > 0 && r.conditions.samples > 0)
        .map(|r| r.conditions.dark as f64 * 100.0 / r.conditions.samples as f64)
        .collect();
    if !dark.is_empty() {
        println!(
            "  display     : off for {}% of the sampled ticks",
            spread(&dark)
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
    use super::{
        Options, Reading, attribution_lines, count_spread, font_walk_lines, median, spread,
    };
    use crate::perf::attribution::{Attribution, Heap};
    use crate::perf::fonts::{FontWalk, Tick};

    /// The run as its flags shaped it, settled the way an invocation's
    /// are — so a test names the measurement mode the reader names
    /// (`--no-scroll`).
    fn shaped(words: &[&str]) -> Options {
        let mut args = vec!["--repo".to_string(), "x".to_string()];
        args.extend(words.iter().map(|w| (*w).to_string()));
        let mut opts = crate::perf::options::parse(&args).expect("a shape the parser takes");
        opts.settle_ms = 8000;
        opts
    }

    /// The walk's weight is said beside the working set, and the net
    /// line is every kept run's peak less that weight — settled too,
    /// where the runs settled. A walk that added nothing takes nothing
    /// off, and says why.
    #[test]
    fn the_budget_line_is_read_net_of_the_font_walk() {
        let kept = [
            Reading {
                peak_working_set: 300 << 20,
                settled_working_set: 296 << 20,
                ..Reading::default()
            },
            Reading {
                peak_working_set: 304 << 20,
                settled_working_set: 298 << 20,
                ..Reading::default()
            },
        ];
        let tick = |at_us, working_set: u64, private: u64| Tick {
            at_us,
            working_set: working_set << 20,
            private: private << 20,
        };
        let walk = FontWalk {
            begin_us: Some(1),
            done_us: Some(2),
            settled_us: Some(3),
            before: Some(tick(0, 200, 150)),
            after: Some(tick(3, 255, 203)),
        };
        let full = shaped(&[]);
        let text = font_walk_lines(&kept, &walk, &full).join("\n");
        assert!(
            text.contains("font walk   : +55.0MB working set, +53.0MB private"),
            "{text}"
        );
        assert!(text.contains("taken off the working set below"), "{text}");
        assert!(text.contains("net         : 245.0–249.0"), "{text}");
        assert!(text.contains("settled net : 241.0–243.0"), "{text}");
        assert!(text.contains("after 8000ms idle"), "{text}");
        let paid = FontWalk {
            after: walk.before,
            ..walk.clone()
        };
        let text = font_walk_lines(&kept, &paid, &full).join("\n");
        assert!(text.contains("+0.0MB working set"), "{text}");
        assert!(text.contains("already been paid"), "{text}");
        assert!(text.contains("net         : 300.0–304.0"), "{text}");
        assert!(text.contains("settled net : 296.0–298.0"), "{text}");
        let unweighed = FontWalk {
            after: None,
            ..walk
        };
        let text = font_walk_lines(&kept, &unweighed, &full).join("\n");
        assert!(text.contains("not weighed"), "{text}");
        assert!(!text.contains("net         :"), "{text}");
    }

    /// **The runs that stage the table never scroll, so nothing in them
    /// paid the walk** — and a net line under one would take tens of MB
    /// off a number nobody spent (P3-確認事項 §性能). The weight is still
    /// printed, because the calibration run did weigh it; what is
    /// withheld is the subtraction and the word `net` in front of it.
    #[test]
    fn a_run_that_never_scrolls_is_not_reported_net_of_a_walk_it_did_not_pay() {
        let kept = [Reading {
            peak_working_set: 300 << 20,
            settled_working_set: 296 << 20,
            ..Reading::default()
        }];
        let tick = |at_us, working_set: u64, private: u64| Tick {
            at_us,
            working_set: working_set << 20,
            private: private << 20,
        };
        let walk = FontWalk {
            begin_us: Some(1),
            done_us: Some(2),
            settled_us: Some(3),
            before: Some(tick(0, 200, 150)),
            after: Some(tick(3, 255, 203)),
        };
        // Every shape the staged table is taken in, and the one it is
        // not: the three on the left hold the window still, and only the
        // full run walks the rows the emoji are in.
        for words in [
            vec!["--no-open"],
            vec!["--no-select", "--no-scroll"],
            vec!["--no-scroll"],
        ] {
            let text = font_walk_lines(&kept, &walk, &shaped(&words)).join("\n");
            assert!(
                text.contains("+55.0MB working set"),
                "the weight is still said for {words:?}: {text}"
            );
            assert!(
                text.contains("never scrolls the window"),
                "and why there is no net, for {words:?}: {text}"
            );
            assert!(
                !text.contains("245.0") && !text.contains("241.0"),
                "nothing is taken off for {words:?}: {text}"
            );
            assert!(
                !text.contains("settled net"),
                "and the settled line goes with it for {words:?}: {text}"
            );
        }
        // `--no-select` alone still scrolls, so it still pays.
        let text = font_walk_lines(&kept, &walk, &shaped(&["--no-select"])).join("\n");
        assert!(text.contains("net         : 245.0"), "{text}");
    }

    #[test]
    fn a_spread_of_one_value_is_printed_once() {
        assert_eq!(spread(&[3.0, 3.0]), "3.0");
        assert_eq!(spread(&[]), "-");
        assert_eq!(count_spread(&[7, 7]), "7");
        assert_eq!(count_spread(&[9, 7, 8]), "7–9");
        assert_eq!(count_spread(&[]), "-");
    }

    /// The block at the end is a spread over the kept runs that had an
    /// attribution, counts printed as counts, and says so when one
    /// run's heap walk was refused.
    #[test]
    fn the_attribution_is_summarised_over_the_runs_that_had_one() {
        let walked = Attribution {
            resident_private: 200 << 20,
            resident_mapped: 30 << 20,
            resident_image: 70 << 20,
            fonts_resident: 31 << 20,
            fonts_files: 12,
            heap: Some(Heap {
                heaps: 12,
                busy: 150 << 20,
                busy_blocks: 700_000,
                free: 12 << 20,
                free_blocks: 4_000,
                committed: 170 << 20,
            }),
            ..Attribution::default()
        };
        let refused = Attribution {
            heap: None,
            heap_error: Some("RtlQueryProcessDebugInformation returned 0xC0000017".into()),
            ..walked.clone()
        };
        let with = |attribution: Attribution| Reading {
            attribution: Some(attribution),
            ..Reading::default()
        };
        assert!(attribution_lines(&[Reading::default()]).is_empty());
        let text = attribution_lines(&[with(walked.clone()), with(walked)]).join("\n");
        assert!(
            text.contains("private 200.0 | mapped 30.0 | image 70.0 MiB"),
            "{text}"
        );
        assert!(text.contains("31.0 MiB in 12 files"), "{text}");
        assert!(
            text.contains("busy 150.0 MiB in 700000 blocks, free 12.0 MiB in 4000 blocks\n"),
            "{text}"
        );
        let text = attribution_lines(&[with(refused.clone()), Reading::default()]).join("\n");
        assert!(text.contains("not walked — RtlQuery"), "{text}");
        let text = attribution_lines(&[
            with(Attribution {
                fonts_files: 11,
                ..refused.clone()
            }),
            with(refused),
        ])
        .join("\n");
        assert!(text.contains("in 11–12 files"), "{text}");
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
