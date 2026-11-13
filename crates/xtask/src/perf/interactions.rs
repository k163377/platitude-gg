//! Per-operation evidence; an aggregate cannot prove every requested case ran.
use super::reading::token;
use super::{Options, Reading};

#[derive(Clone, Debug)]
pub(super) struct Event {
    pub(super) kind: String,
    pub(super) case: String,
    pub(super) operation: usize,
    pub(super) oid: String,
    pub(super) fingerprint: String,
    pub(super) ms: f64,
}

/// The two frames timed for an input the build says it shows at once
/// (`Reading::frames_synced`), ahead of the frames with the answers in
/// them. No name here is part of another, so a line is one kind.
const FEEDBACK: [&str; 2] = ["perf_details_feedback", "perf_diff_feedback"];
const KINDS: [&str; 6] = [
    FEEDBACK[0],
    "perf_details_frame",
    FEEDBACK[1],
    "perf_diff_frame",
    "perf_colour_frame",
    "perf_diff_scroll_frame",
];

pub(super) fn read(line: &str) -> Option<Event> {
    let kind = KINDS.into_iter().find(|k| line.contains(k))?;
    Some(Event {
        kind: kind.into(),
        case: token(line, "case=")?.into(),
        operation: token(line, "operation=")?.parse().ok()?,
        oid: token(line, "oid=").unwrap_or("").into(),
        fingerprint: token(line, "fingerprint=").unwrap_or("").into(),
        ms: if kind == "perf_diff_scroll_frame" {
            0.0
        } else {
            token(line, "elapsed_ms=")?.parse().ok()?
        },
    })
}

pub(super) fn validate(reading: &Reading, opts: &Options) -> Result<(), String> {
    // A run of one operation says no case and numbers nothing, so there is
    // no per-operation reading to hold to: what it owes is read off the
    // run as a whole (`reading::method_gaps`), this off the runs that
    // count their operations.
    if opts.cases.is_empty() && !opts.diff_scroll && opts.completion == "raw" {
        return Ok(());
    }
    let count = opts.cases.len().max(1) * opts.cycles as usize;
    for operation in 0..count {
        let case = if opts.cases.is_empty() {
            None
        } else {
            opts.cases.get(operation % opts.cases.len())
        };
        let name = case.map_or("default", |c| c.name.as_str());
        let oid = case.map_or(opts.oid.as_str(), |c| c.oid.as_str());
        let completion = case.map_or(opts.completion.as_str(), |c| c.completion.as_str());
        for kind in KINDS {
            if (kind == "perf_colour_frame" && completion != "coloured")
                || (kind == "perf_diff_scroll_frame" && !opts.diff_scroll)
                || (FEEDBACK.contains(&kind) && !reading.frames_synced())
            {
                continue;
            }
            let matching: Vec<_> = reading
                .events
                .iter()
                .filter(|e| e.operation == operation && e.kind == kind)
                .collect();
            if matching.len() != 1
                || matching[0].case != name
                || !matching[0].ms.is_finite()
                || matching[0].ms < 0.0
                || (kind != "perf_diff_scroll_frame" && !oid.is_empty() && oid != matching[0].oid)
            {
                return Err(format!(
                    "operation {operation} case {name}: missing, duplicate or mismatched {kind}"
                ));
            }
        }
        // The input is shown before it is answered: a feedback frame
        // later than its completion is not the frame it claims to be.
        for (feedback, completed) in [
            (FEEDBACK[0], "perf_details_frame"),
            (FEEDBACK[1], "perf_diff_frame"),
        ] {
            let of = |kind: &str| {
                reading
                    .events
                    .iter()
                    .find(|e| e.operation == operation && e.kind == kind)
                    .map(|e| e.ms)
            };
            if let (Some(shown), Some(answered)) = (of(feedback), of(completed))
                && shown > answered
            {
                return Err(format!(
                    "operation {operation} case {name}: {feedback} came after {completed}"
                ));
            }
        }
        if completion == "coloured" {
            let raw = reading
                .events
                .iter()
                .find(|e| e.operation == operation && e.kind == "perf_diff_frame");
            let colour = reading
                .events
                .iter()
                .find(|e| e.operation == operation && e.kind == "perf_colour_frame");
            if let (Some(raw), Some(colour)) = (raw, colour)
                && (raw.fingerprint.is_empty()
                    || raw.fingerprint != colour.fingerprint
                    || colour.ms < raw.ms)
            {
                return Err(format!(
                    "operation {operation}: colour does not follow the same diff"
                ));
            }
        }
    }
    if reading.events.iter().any(|e| e.operation >= count) {
        return Err("the run reported unrequested operations".into());
    }
    if opts.diff_scroll && reading.diff_scrolls.len() != count {
        return Err("missing diff scroll measurements".into());
    }
    validate_scrolls(reading, opts)
}

fn validate_scrolls(reading: &Reading, opts: &Options) -> Result<(), String> {
    let mut seen = std::collections::BTreeSet::new();
    for line in &reading.diff_scrolls {
        let operation = token(line, "operation=").and_then(|s| s.parse::<usize>().ok());
        let event = reading
            .events
            .iter()
            .find(|e| Some(e.operation) == operation && e.kind == "perf_diff_scroll_frame");
        if !opts.diff_scroll
            || event.is_none_or(|e| {
                token(line, "case=").map(|s| s.trim_matches('"')) != Some(e.case.as_str())
            })
            || !seen.insert(operation)
        {
            return Err("diff scroll measurements have duplicate or mismatched operations".into());
        }
        let positive = |key| {
            token(line, key)
                .and_then(|s| s.parse::<f64>().ok())
                .is_some_and(|v| v.is_finite() && v > 0.0)
        };
        if token(line, "visible=") != Some("true")
            || !positive("moved=")
            || !positive("fps=")
            || !positive("frame_count=")
        {
            return Err("diff scroll did not measure visible movement and frames".into());
        }
    }
    Ok(())
}

pub(super) fn report(kept: &[Reading]) {
    // Each case and kind, with its times run by run.
    let mut groups = std::collections::BTreeMap::<(&str, &str), Vec<Vec<f64>>>::new();
    for (run, reading) in kept.iter().enumerate() {
        for e in reading
            .events
            .iter()
            .filter(|e| e.kind != "perf_diff_scroll_frame")
        {
            let runs = groups.entry((&e.case, &e.kind)).or_default();
            runs.resize(kept.len(), Vec::new());
            runs[run].push(e.ms);
        }
    }
    for ((case, kind), runs) in groups {
        println!("  case {case} {kind}: {}", distribution(&runs));
    }
    for line in kept.iter().flat_map(|r| &r.diff_scrolls) {
        println!("  diff scroll: {line}");
    }
}

/// The least a sample has to hold for a percentile to be a time one of
/// its operations took and not its maximum under another name.
const P95_FROM: usize = 20;
const P99_FROM: usize = 100;

/// One case's times over the kept runs, pooled, and the middle of each
/// run beside them: a whole run sits several milliseconds off another of
/// the same build (ci/baseline/perf-windows-x64.md §この記録の読み方), which
/// the pooled percentiles alone would show as spread within a run.
fn distribution(runs: &[Vec<f64>]) -> String {
    let mut times: Vec<f64> = runs.iter().flatten().copied().collect();
    times.sort_by(f64::total_cmp);
    let n = times.len();
    if n == 0 {
        return "n=0".into();
    }
    let at =
        |sorted: &[f64], pct: usize| sorted[(sorted.len() * pct).div_ceil(100).saturating_sub(1)];
    let mut said = format!("n={n} p50={:.1}", at(&times, 50));
    if n >= P95_FROM {
        said.push_str(&format!(" p95={:.1}", at(&times, 95)));
    }
    if n >= P99_FROM {
        said.push_str(&format!(" p99={:.1}", at(&times, 99)));
    }
    let over = times.iter().filter(|t| **t > 100.0).count();
    said.push_str(&format!(
        " max={:.1}ms over100={over} ({:.1}%)",
        times[n - 1],
        over as f64 * 100.0 / n as f64
    ));
    let middles: Vec<String> = runs
        .iter()
        .filter(|run| !run.is_empty())
        .map(|run| {
            let mut sorted = run.clone();
            sorted.sort_by(f64::total_cmp);
            format!("{:.1}", at(&sorted, 50))
        })
        .collect();
    if middles.len() > 1 && n > middles.len() {
        said.push_str(&format!(" | p50 by run {}", middles.join(" ")));
    }
    said
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_raw_frame_does_not_satisfy_the_colour_contract() {
        let opts = super::super::options::parse(
            &["--repo", "x", "--completion", "coloured"].map(str::to_owned),
        )
        .unwrap();
        let mut reading = Reading::default();
        for kind in ["perf_details_frame", "perf_diff_frame"] {
            reading.events.push(
                read(&format!(
                    "{kind} case=default operation=0 oid=abc fingerprint=f elapsed_ms=10"
                ))
                .unwrap(),
            );
        }
        assert!(
            validate(&reading, &opts)
                .unwrap_err()
                .contains("perf_colour_frame")
        );
        reading.events.push(read("perf_colour_frame case=default operation=0 oid=abc fingerprint=wrong elapsed_ms=20").unwrap());
        assert!(validate(&reading, &opts).unwrap_err().contains("same diff"));
        reading.events[2].fingerprint = "f".into();
        assert!(validate(&reading, &opts).is_ok());
        reading.events.push(reading.events[2].clone());
        assert!(validate(&reading, &opts).unwrap_err().contains("duplicate"));
    }

    #[test]
    fn missing_time_and_foreign_scrolls_are_not_measurements() {
        assert!(read("perf_colour_frame case=default operation=0 oid=abc fingerprint=f").is_none());
        let opts =
            super::super::options::parse(&["--repo", "x", "--diff-scroll"].map(str::to_owned))
                .unwrap();
        let mut reading = Reading::default();
        for kind in [
            "perf_details_frame",
            "perf_diff_frame",
            "perf_diff_scroll_frame",
        ] {
            reading.events.push(
                read(&format!(
                    "{kind} case=default operation=0 oid=abc fingerprint=f elapsed_ms=10"
                ))
                .unwrap(),
            );
        }
        reading.diff_scrolls.push("scroll_bench surface=\"diff\" case=\"default\" operation=1 visible=true moved=100 fps=60 frame_count=120".into());
        assert!(
            validate(&reading, &opts)
                .unwrap_err()
                .contains("mismatched operations")
        );
        reading.diff_scrolls[0] = reading.diff_scrolls[0].replace("operation=1", "operation=0");
        assert!(validate(&reading, &opts).is_ok());
        let mut explicit = opts;
        explicit.oid = "another".into();
        assert!(
            validate(&reading, &explicit)
                .unwrap_err()
                .contains("mismatched perf_details")
        );
    }

    /// Asked of a build that says it times them, of every operation, and
    /// held to the order an input is shown and answered in.
    #[test]
    fn feedback_is_owed_by_the_build_that_times_it_and_comes_before_the_answer() {
        let opts = super::super::options::parse(
            &["--repo", "x", "--completion", "coloured"].map(str::to_owned),
        )
        .unwrap();
        let said = |kind: &str, ms: u32| {
            read(&format!(
                "{kind} elapsed_ms={ms} asked_ms=0 passed_over=1 case=default operation=0 oid=abc \
                 rows_gen=1 fingerprint=f"
            ))
            .unwrap()
        };
        let mut reading = Reading::default();
        for (kind, ms) in [
            ("perf_details_frame", 80),
            ("perf_diff_frame", 90),
            ("perf_colour_frame", 120),
        ] {
            reading.events.push(said(kind, ms));
        }
        assert!(validate(&reading, &opts).is_ok(), "a build that says none");
        reading.method = Some("frames=synced start=idle".into());
        assert!(
            validate(&reading, &opts)
                .unwrap_err()
                .contains("perf_details_feedback")
        );
        reading.events.push(said("perf_details_feedback", 9));
        reading.events.push(said("perf_diff_feedback", 95));
        assert!(
            validate(&reading, &opts)
                .unwrap_err()
                .contains("perf_diff_feedback came after perf_diff_frame")
        );
        reading.events[4].ms = 8.0;
        assert!(validate(&reading, &opts).is_ok());
    }

    #[test]
    fn a_percentile_is_printed_only_where_the_sample_holds_one() {
        assert_eq!(
            distribution(&[vec![90.0], vec![128.0], vec![93.0]]),
            "n=3 p50=93.0 max=128.0ms over100=1 (33.3%)"
        );
        let twenty: Vec<f64> = (1..=20).map(f64::from).collect();
        let said = distribution(&[twenty]);
        assert!(
            said.contains("p95=19.0") && !said.contains("p99="),
            "{said}"
        );
        let hundred: Vec<f64> = (1..=100).map(f64::from).collect();
        assert!(distribution(&[hundred]).contains("p99=99.0"));
        assert_eq!(distribution(&[Vec::new()]), "n=0");
    }

    /// The pooled middle of two runs that sit apart is a time neither
    /// run's operations took.
    #[test]
    fn each_runs_middle_stands_beside_the_pooled_one() {
        let said = distribution(&[vec![70.0; 10], vec![90.0; 10], Vec::new()]);
        assert!(said.starts_with("n=20 p50=70.0 p95=90.0"), "{said}");
        assert!(said.ends_with("| p50 by run 70.0 90.0"), "{said}");
    }
}
