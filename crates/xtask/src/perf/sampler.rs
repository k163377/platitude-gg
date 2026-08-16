//! Peak process-memory sampling for the performance harness.

use std::process::Command;
use std::time::Instant;

#[cfg(target_os = "linux")]
use std::time::Duration;

use super::SAMPLE_MS;

/// Samples until the measured process is reaped or the outer watchdog ends.
///
/// Windows keeps one PowerShell process for the whole run (spawning one per
/// 100ms sample would perturb the measurement); Linux reads `/proc` directly.
pub(super) fn sample_memory(pid: u32, deadline: Instant) -> std::thread::JoinHandle<(u64, u64)> {
    std::thread::spawn(move || {
        #[cfg(windows)]
        {
            windows_sampler(pid, deadline)
        }
        #[cfg(target_os = "linux")]
        {
            linux_sampler(pid, deadline)
        }
        #[cfg(not(any(windows, target_os = "linux")))]
        {
            let _ = (pid, deadline);
            (0, 0)
        }
    })
}

/// Takes the final sample while the child is deliberately held after
/// `perf_done`; this closes the startup race for very short bare-window runs.
pub(super) fn sample_once(pid: u32) -> (u64, u64) {
    #[cfg(windows)]
    let sample = windows_sample_once(pid);
    #[cfg(target_os = "linux")]
    let sample = linux_sample_once(pid);
    #[cfg(not(any(windows, target_os = "linux")))]
    let sample = {
        let _ = pid;
        (0, 0)
    };
    sample
}

#[cfg(windows)]
fn windows_sampler(pid: u32, deadline: Instant) -> (u64, u64) {
    let seconds = deadline.saturating_duration_since(Instant::now()).as_secs() + 5;
    // `Refresh()` makes the held Process object re-read its counters;
    // without it every iteration would return the first sample.
    let script = format!(
        "$ErrorActionPreference='SilentlyContinue';\
         $p=Get-Process -Id {pid};\
         $ws=0;$pv=0;$end=(Get-Date).AddSeconds({seconds});\
         while($p -ne $null -and -not $p.HasExited -and (Get-Date) -lt $end){{\
           $p.Refresh();\
           if($p.WorkingSet64 -gt $ws){{$ws=$p.WorkingSet64}};\
           if($p.PrivateMemorySize64 -gt $pv){{$pv=$p.PrivateMemorySize64}};\
           Start-Sleep -Milliseconds {SAMPLE_MS};\
         }};\
         Write-Output \"$ws $pv\""
    );
    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output();
    let Ok(out) = out else { return (0, 0) };
    parse_pair(&String::from_utf8_lossy(&out.stdout))
}

#[cfg(windows)]
fn windows_sample_once(pid: u32) -> (u64, u64) {
    let script = format!(
        "$p=Get-Process -Id {pid} -ErrorAction SilentlyContinue;\
         if($p -ne $null){{\
           Write-Output \"$($p.WorkingSet64) $($p.PrivateMemorySize64)\"\
         }}"
    );
    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output();
    out.ok()
        .map(|out| parse_pair(&String::from_utf8_lossy(&out.stdout)))
        .unwrap_or_default()
}

#[cfg(target_os = "linux")]
fn linux_sampler(pid: u32, deadline: Instant) -> (u64, u64) {
    let status = format!("/proc/{pid}/status");
    let (mut ws, mut pv) = (0u64, 0u64);
    while Instant::now() < deadline {
        let (next_ws, next_pv) = linux_sample_once(pid);
        if next_ws == 0 && next_pv == 0 && !std::path::Path::new(&status).exists() {
            break;
        }
        ws = ws.max(next_ws);
        pv = pv.max(next_pv);
        std::thread::sleep(Duration::from_millis(SAMPLE_MS));
    }
    (ws, pv)
}

#[cfg(target_os = "linux")]
fn linux_sample_once(pid: u32) -> (u64, u64) {
    let path = format!("/proc/{pid}/status");
    let Ok(text) = std::fs::read_to_string(path) else {
        return (0, 0);
    };
    let mut pair = (0, 0);
    for line in text.lines() {
        let kb = |l: &str| {
            l.split_whitespace()
                .nth(1)
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0)
                * 1024
        };
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            pair.0 = kb(rest);
        } else if let Some(rest) = line.strip_prefix("VmData:") {
            pair.1 = kb(rest);
        }
    }
    pair
}

/// `"123 456"` -> `(123, 456)`; anything else becomes zero.
fn parse_pair(text: &str) -> (u64, u64) {
    let mut numbers = text.split_whitespace().filter_map(|v| v.parse().ok());
    (numbers.next().unwrap_or(0), numbers.next().unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::parse_pair;

    #[test]
    fn the_sampler_pair_survives_junk() {
        assert_eq!(parse_pair("123 456\r\n"), (123, 456));
        assert_eq!(parse_pair(""), (0, 0));
    }
}
