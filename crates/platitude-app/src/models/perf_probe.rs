//! Automation-only clock and frame observations. No wall-clock subtraction
//! between QML, Rust, and the parent process.

use std::sync::OnceLock;
use std::time::Instant;

use qtbridge::qobject;

use super::qml_register;

static START: OnceLock<Instant> = OnceLock::new();

pub struct PerfProbe {
    selection: String,
    oid: String,
    file_path: String,
    with_diff: bool,
    verifying: bool,
    scroll_start: Option<f64>,
    last_frame: f64,
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
            scroll_start: None,
            last_frame: 0.0,
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
        self.last_frame = now;
        self.frames.clear();
    }

    #[qslot]
    fn frame(&mut self) {
        if self.scroll_start.is_some() {
            let now = self.clock_ms();
            self.frames.push(now - self.last_frame);
            self.last_frame = now;
        }
    }

    #[qslot]
    fn end_scroll(&mut self, rows: i32, moved: f64, visible: bool) {
        let Some(start) = self.scroll_start.take() else {
            return;
        };
        let elapsed = self.clock_ms() - start;
        self.frames.sort_by(f64::total_cmp);
        let n = self.frames.len();
        tracing::info!(
            fps = if elapsed > 0.0 {
                n as f64 * 1000.0 / elapsed
            } else {
                0.0
            },
            rows,
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

fn percentile(sorted: &[f64], percent: usize) -> f64 {
    let rank = (sorted.len() * percent).div_ceil(100);
    sorted.get(rank.saturating_sub(1)).copied().unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::percentile;

    #[test]
    fn frame_tail_is_not_hidden_by_the_average() {
        let mut frames = vec![10.0; 99];
        frames.push(160.0);
        assert_eq!(percentile(&frames, 50), 10.0);
        assert_eq!(percentile(&frames, 100), 160.0);
        assert_eq!(percentile(&[], 99), 0.0);
    }
}
