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

pub(super) fn read(line: &str) -> Option<Event> {
    let kind = [
        "perf_details_frame",
        "perf_diff_frame",
        "perf_colour_frame",
        "perf_diff_scroll_frame",
    ]
    .into_iter()
    .find(|k| line.contains(k))?;
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
        for kind in [
            "perf_details_frame",
            "perf_diff_frame",
            "perf_colour_frame",
            "perf_diff_scroll_frame",
        ] {
            if (kind == "perf_colour_frame" && completion != "coloured")
                || (kind == "perf_diff_scroll_frame" && !opts.diff_scroll)
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
    let mut groups = std::collections::BTreeMap::<(&str, &str), Vec<f64>>::new();
    for e in kept
        .iter()
        .flat_map(|r| &r.events)
        .filter(|e| e.kind != "perf_diff_scroll_frame")
    {
        groups.entry((&e.case, &e.kind)).or_default().push(e.ms);
    }
    for ((case, kind), mut times) in groups {
        times.sort_by(f64::total_cmp);
        let p = |pct: usize| times[(times.len() * pct).div_ceil(100).saturating_sub(1)];
        let over = times.iter().filter(|t| **t > 100.0).count();
        println!(
            "  case {case} {kind}: n={} p50={:.1} p95={:.1} p99={:.1} max={:.1}ms over100={over} ({:.1}%)",
            times.len(),
            p(50),
            p(95),
            p(99),
            p(100),
            over as f64 * 100.0 / times.len() as f64
        );
    }
    for line in kept.iter().flat_map(|r| &r.diff_scrolls) {
        println!("  diff scroll: {line}");
    }
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
}
