//! Automation-only clock and frame observations. Every reading is taken
//! against the one app clock.

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
    cases: Vec<Vec<String>>,
    cycles: u32,
    completion: String,
    diff_scroll: bool,
    scroll_surface: String,
    scroll_case: String,
    scroll_operation: i32,
    scroll_start: Option<f64>,
    // App-clock timestamps stay ordered until the sampling window closes.
    frames: Vec<f64>,
}

impl Default for PerfProbe {
    fn default() -> Self {
        let knobs = super::knobs();
        Self {
            selection: if knobs.perf_selection.is_empty() {
                if knobs.select { "first" } else { "none" }.into()
            } else {
                knobs.perf_selection.clone()
            },
            oid: knobs.perf_oid.clone(),
            file_path: knobs.perf_file.clone(),
            with_diff: !knobs.perf_no_diff,
            verifying: knobs.act == "perf",
            trace_frames: knobs.perf_trace_frames,
            cases: knobs
                .perf_cases
                .lines()
                .map(|l| l.split('\t').map(str::to_owned).collect())
                .collect(),
            cycles: knobs.perf_cycles.max(1),
            completion: knobs.perf_completion.clone(),
            diff_scroll: knobs.perf_diff_scroll,
            scroll_surface: "graph".into(),
            scroll_case: "default".into(),
            scroll_operation: 0,
            scroll_start: None,
            frames: Vec::new(),
        }
    }
}

impl PerfProbe {
    pub fn start_clock() {
        // waits(measured): the instant every reading of a run is written against
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
    qproperty!("diffScroll", Member = diff_scroll, Constant);

    #[qslot]
    fn operation_count(&self) -> i32 {
        self.cases
            .len()
            .max(1)
            .saturating_mul(self.cycles as usize)
            .min(i32::MAX as usize) as i32
    }

    #[qslot]
    fn case_field(&self, operation: i32, field: i32) -> String {
        if operation < 0 || field < 0 {
            return String::new();
        }
        if self.cases.is_empty() {
            return match field {
                0 => "default".into(),
                1 => self.oid.clone(),
                2 => self.file_path.clone(),
                3 => self.completion.clone(),
                _ => String::new(),
            };
        }
        self.cases
            .get(operation as usize % self.cases.len())
            .and_then(|c| c.get(field as usize))
            .cloned()
            .unwrap_or_default()
    }

    #[qslot]
    fn scroll_surface(&mut self, surface: String) {
        self.scroll_surface = surface;
    }

    #[qslot]
    fn scroll_context(&mut self, case: String, operation: i32) {
        self.scroll_case = case;
        self.scroll_operation = operation;
    }

    #[qslot]
    fn clock_ms(&self) -> f64 {
        // waits(measured): the clock the drivers date their report lines by; no verdict is taken here
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
        // Past the last frame: IO in the per-frame callback would move the
        // frames being measured.
        if self.trace_frames {
            let mut previous = start;
            for (index, &clock_ms) in self.frames.iter().enumerate() {
                tracing::info!(
                    index,
                    clock_ms,
                    interval_ms = clock_ms - previous,
                    surface = self.scroll_surface,
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
            surface = self.scroll_surface,
            case = self.scroll_case,
            operation = self.scroll_operation,
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
