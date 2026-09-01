//! Automation-only clock and frame observations. No wall-clock subtraction
//! between QML, Rust, and the parent process.

use std::sync::OnceLock;
use std::time::Instant;

use qtbridge::qobject;

use crate::models::qml_register;

static START: OnceLock<Instant> = OnceLock::new();

pub struct PerfProbe {
    selection: String,
    oid: String,
    file_path: String,
    with_diff: bool,
    verifying: bool,
    trace_frames: bool,
    scroll_start: Option<f64>,
    // App-clock timestamps stay ordered until the sampling window closes.
    frames: Vec<f64>,
}

impl Default for PerfProbe {
    fn default() -> Self {
        Self {
            selection: std::env::var("PG_PERF_SELECTION").unwrap_or_else(|_| {
                if std::env::var("PG_AUTO_SELECT").as_deref() == Ok("1") {
                    "first".into()
                } else {
                    "none".into()
                }
            }),
            oid: std::env::var("PG_PERF_OID").unwrap_or_default(),
            file_path: std::env::var("PG_PERF_FILE").unwrap_or_default(),
            with_diff: std::env::var("PG_PERF_DIFF").as_deref() != Ok("0"),
            verifying: std::env::var("PG_AUTO_ACT").as_deref() == Ok("perf"),
            trace_frames: std::env::var("PG_PERF_TRACE_FRAMES").as_deref() == Ok("1"),
            scroll_start: None,
            frames: Vec::new(),
        }
    }
}

impl PerfProbe {
    pub fn start_clock() {
        START.get_or_init(Instant::now);
    }
}

#[qobject(ConvertToCamelCase, NoQmlElement)]
impl PerfProbe {
    qproperty!("selection", Member = selection, Constant);
    qproperty!("oid", Member = oid, Constant);
    qproperty!("filePath", Member = file_path, Constant);
    qproperty!("withDiff", Member = with_diff, Constant);
    qproperty!("verifying", Member = verifying, Constant);

    #[qslot]
    fn clock_ms(&self) -> f64 {
        START.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0
    }

    #[qslot]
    fn begin_scroll(&mut self) {
        let now = self.clock_ms();
        self.scroll_start = Some(now);
        self.frames.clear();
        tracing::info!(clock_ms = now, "perf_scroll_begin");
    }

    #[qslot]
    fn frame(&mut self) {
        if self.scroll_start.is_some() {
            let now = self.clock_ms();
            self.frames.push(now);
        }
    }

    #[qslot]
    fn end_scroll(&mut self, rows: i32, moved: f64, visible: bool) {
        let Some(start) = self.scroll_start.take() else {
            return;
        };
        let elapsed = self.clock_ms() - start;
        // No log or file IO in the per-frame callback. Detailed traces are
        // diagnostic runs: the flush can affect the final frame and memory.
        if self.trace_frames {
            let mut previous = start;
            for (index, &clock_ms) in self.frames.iter().enumerate() {
                tracing::info!(
                    index,
                    clock_ms,
                    interval_ms = clock_ms - previous,
                    "perf_frame"
                );
                previous = clock_ms;
            }
        }
        intervals_in_place(&mut self.frames, start);
        self.frames.sort_by(f64::total_cmp);
        let n = self.frames.len();
        tracing::info!(
            fps = if elapsed > 0.0 {
                n as f64 * 1000.0 / elapsed
            } else {
                0.0
            },
            rows,
            start_clock_ms = start,
            elapsed_ms = elapsed,
            moved,
            visible,
            frame_count = n,
            frame_p50_ms = percentile(&self.frames, 50),
            frame_p95_ms = percentile(&self.frames, 95),
            frame_p99_ms = percentile(&self.frames, 99),
            frame_max_ms = self.frames.last().copied().unwrap_or(0.0),
            over_16_ms = self.frames.iter().filter(|v| **v > 1000.0 / 60.0).count(),
            over_100_ms = self.frames.iter().filter(|v| **v > 100.0).count(),
            "scroll_bench"
        );
    }
}
qml_register!(PerfProbe, "PerfProbe", singleton = true);

fn intervals_in_place(timestamps: &mut [f64], mut previous: f64) {
    for timestamp in timestamps {
        let now = *timestamp;
        *timestamp = now - previous;
        previous = now;
    }
}

fn percentile(sorted: &[f64], percent: usize) -> f64 {
    let rank = (sorted.len() * percent).div_ceil(100);
    sorted.get(rank.saturating_sub(1)).copied().unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::{intervals_in_place, percentile};

    #[test]
    fn frame_timestamps_locate_a_stall_without_changing_its_interval() {
        let mut timestamps = vec![1010.0, 1020.0, 1180.0, 1190.0];
        intervals_in_place(&mut timestamps, 1000.0);
        assert_eq!(timestamps, vec![10.0, 10.0, 160.0, 10.0]);
    }

    #[test]
    fn frame_tail_is_not_hidden_by_the_average() {
        let mut frames = vec![10.0; 99];
        frames.push(160.0);
        assert_eq!(percentile(&frames, 50), 10.0);
        assert_eq!(percentile(&frames, 100), 160.0);
        assert_eq!(percentile(&[], 99), 0.0);
    }
}
