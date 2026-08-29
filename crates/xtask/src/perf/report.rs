//! Turning the readings of a run into the lines the record is written
//! from: megabytes, the spread over the kept runs, and the block printed
//! at the end of `perf`.

use super::{Options, Reading};

pub(super) fn mb(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}
pub(super) fn report(opts: &Options, kept: &[Reading]) {
    if kept.is_empty() {
        return;
    }
    let ws: Vec<f64> = kept.iter().map(|r| mb(r.peak_working_set)).collect();
    let private: Vec<f64> = kept.iter().map(|r| mb(r.peak_private)).collect();
    println!("\n== {} ==", opts.label);
    println!("  working set : {}", spread(&ws));
    println!("  private     : {}", spread(&private));
    let startups: Vec<f64> = kept.iter().filter_map(|r| r.startup_ms).map(f).collect();
    if !startups.is_empty() {
        println!(
            "  startup     : {} ms (to the first rows)",
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
    let fps: Vec<f64> = kept.iter().filter_map(|r| r.fps).collect();
    if !fps.is_empty() {
        println!("  scroll      : {} fps", spread(&fps));
    }
    let details: Vec<f64> = kept
        .iter()
        .flat_map(|r| r.details_ms.clone())
        .map(f)
        .collect();
    if !details.is_empty() {
        println!("  details     : {} ms", spread(&details));
    }
    if let Some(line) = kept
        .iter()
        .max_by_key(|r| r.breakdown_live)
        .and_then(|r| r.breakdown.clone())
    {
        println!("\n  breakdown at the largest Rust heap of the kept runs:\n    {line}");
    }
}

fn f(v: u64) -> f64 {
    v as f64
}

/// `min–max` over the readings, or the single value when they agree.
fn spread(values: &[f64]) -> String {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    match (sorted.first(), sorted.last()) {
        (Some(lo), Some(hi)) if (hi - lo).abs() < 0.05 => format!("{lo:.1}"),
        (Some(lo), Some(hi)) => format!("{lo:.1}–{hi:.1}"),
        _ => "-".into(),
    }
}

#[cfg(test)]
mod tests {

    use super::spread;

    #[test]
    fn a_spread_of_one_value_is_printed_once() {
        assert_eq!(spread(&[3.0, 3.0]), "3.0");

        assert_eq!(spread(&[3.0, 5.0]), "3.0–5.0");

        assert_eq!(spread(&[]), "-");
    }
}
