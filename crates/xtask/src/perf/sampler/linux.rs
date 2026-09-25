//! The Linux sampler: `/proc`, read in-process at the samplers' pace.

use std::time::{Duration, Instant};

use super::Sample;
use crate::perf::SAMPLE_MS;

/// Samples `pid` while the process stands, for at most `window` past
/// `started`. Reaching the window's end is nobody's failure — `measure`
/// ends the run inside it — so this is a stand watched (`wait::stood`).
pub(super) fn linux_sampler(
    pid: u32,
    started: Instant,
    window: Duration,
    record: &mut dyn FnMut(Sample) -> Result<(), String>,
) -> Result<(), String> {
    let status = format!("/proc/{pid}/status");
    let stretch = window.saturating_sub(started.elapsed());
    match crate::wait::stood(stretch, Duration::from_millis(SAMPLE_MS), || {
        let sample = linux_sample_once(pid);
        if sample.working_set == 0 && sample.private == 0 && !std::path::Path::new(&status).exists()
        {
            return Some(Ok(()));
        }
        record(sample).err().map(Err)
    }) {
        Err(ended) => ended,
        Ok(_stood_for) => Ok(()),
    }
}

/// Memory and processor time only: nothing measures a real window on
/// Linux (`perf::guard_the_window`).
fn linux_sample_once(pid: u32) -> Sample {
    let mut sample = Sample::default();
    let Ok(text) = std::fs::read_to_string(format!("/proc/{pid}/status")) else {
        return sample;
    };
    for line in text.lines() {
        let kb = |l: &str| {
            l.split_whitespace()
                .next()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0)
                * 1024
        };
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            sample.working_set = kb(rest);
        } else if let Some(rest) = line.strip_prefix("VmHWM:") {
            sample.os_peak_working_set = Some(kb(rest));
        } else if let Some(rest) = line.strip_prefix("VmData:") {
            sample.private = kb(rest);
        }
    }
    // USER_HZ ticks: the ratios this feeds are unit-free, so only the
    // shape has to match Windows.
    if let Ok(stat) = std::fs::read_to_string("/proc/stat")
        && let Some(cpu) = stat.lines().next().and_then(|l| l.strip_prefix("cpu "))
    {
        let values: Vec<u64> = cpu
            .split_whitespace()
            .filter_map(|v| v.parse().ok())
            .collect();
        sample.user = values.first().copied().unwrap_or(0) + values.get(1).copied().unwrap_or(0);
        sample.idle = values.get(3).copied().unwrap_or(0);
        sample.kernel = values.iter().sum::<u64>() - sample.user;
    }
    // The process's utime + stime. After the last `)` is field 3 onwards —
    // the only safe split past a command name that may hold spaces and
    // parentheses.
    if let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat"))
        && let Some(after) = stat.rfind(')').map(|at| &stat[at + 1..])
    {
        let field = |at: usize| -> u64 {
            after
                .split_whitespace()
                .nth(at)
                .and_then(|v| v.parse().ok())
                .unwrap_or(0)
        };
        sample.app = field(11) + field(12);
        sample.own = sample.app;
    }
    sample
}
