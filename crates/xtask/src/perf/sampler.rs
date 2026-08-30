//! Peak process-memory sampling for the performance harness.

use std::io::Write;
#[cfg(windows)]
use std::process::Command;
use std::time::Instant;

#[cfg(target_os = "linux")]
use std::time::Duration;

use super::SAMPLE_MS;

/// Samples until the measured process is reaped or the outer watchdog ends.
///
/// Windows keeps one PowerShell process for the whole run (spawning one per
/// 100ms sample would perturb the measurement); Linux reads `/proc` directly.
pub(super) fn sample_memory(
    pid: u32,
    deadline: Instant,
    started: Instant,
    mut csv: std::fs::File,
) -> std::thread::JoinHandle<Result<(u64, u64), String>> {
    std::thread::spawn(move || {
        writeln!(csv, "parent_elapsed_us,working_set_bytes,private_bytes")
            .map_err(|e| e.to_string())?;
        #[cfg(windows)]
        {
            windows_sampler(pid, deadline, started, &mut csv)
        }
        #[cfg(target_os = "linux")]
        {
            linux_sampler(pid, deadline, started, &mut csv)
        }
        #[cfg(not(any(windows, target_os = "linux")))]
        {
            let _ = (pid, deadline, started);
            Err("memory sampling is not implemented for this OS".into())
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
fn windows_sampler(
    pid: u32,
    deadline: Instant,
    started: Instant,
    csv: &mut std::fs::File,
) -> Result<(u64, u64), String> {
    use std::io::{BufRead, BufReader};
    let seconds = deadline.saturating_duration_since(Instant::now()).as_secs() + 5;
    // `Refresh()` makes the held Process object re-read its counters;
    // without it every iteration would return the first sample.
    let script = format!(
        "$ErrorActionPreference='SilentlyContinue';\
         $p=Get-Process -Id {pid};\
         $end=(Get-Date).AddSeconds({seconds});\
         while($p -ne $null -and -not $p.HasExited -and (Get-Date) -lt $end){{\
           $p.Refresh();\
           Write-Output \"$($p.WorkingSet64) $($p.PrivateMemorySize64)\";\
           Start-Sleep -Milliseconds {SAMPLE_MS};\
         }}"
    );
    let mut child = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let stdout = child.stdout.take().ok_or("memory sampler stdout missing")?;
    let mut peak = (0, 0);
    let mut error = None;
    for line in BufReader::new(stdout).lines() {
        let pair = match line {
            Ok(line) => parse_pair(&line),
            Err(e) => {
                error = Some(e.to_string());
                break;
            }
        };
        peak.0 = peak.0.max(pair.0);
        peak.1 = peak.1.max(pair.1);
        if let Err(e) = writeln!(
            csv,
            "{},{},{}",
            started.elapsed().as_micros(),
            pair.0,
            pair.1
        ) {
            error = Some(e.to_string());
            break;
        }
    }
    if error.is_some() {
        child.kill().map_err(|e| e.to_string())?;
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if let Some(error) = error {
        return Err(error);
    }
    if !status.success() {
        return Err("memory sampler failed".into());
    }
    Ok(peak)
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

/// Holds for `ms` and answers the *last* working-set sample — the settled
/// value, where [`sample_memory`] answers the peak. One resident process
/// on Windows for the same reason as there: a spawn per 100ms sample
/// perturbs the very settling this measures (and cannot keep the period).
pub(super) fn sample_last(pid: u32, ms: u64) -> u64 {
    #[cfg(windows)]
    {
        let script = format!(
            "$ErrorActionPreference='SilentlyContinue';\
             $p=Get-Process -Id {pid};\
             $ws=0;$end=(Get-Date).AddMilliseconds({ms});\
             while($p -ne $null -and -not $p.HasExited -and (Get-Date) -lt $end){{\
               $p.Refresh();\
               $ws=$p.WorkingSet64;\
               Start-Sleep -Milliseconds {SAMPLE_MS};\
             }};\
             Write-Output \"$ws\""
        );
        let out = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .output();
        let Ok(out) = out else { return 0 };
        parse_pair(&String::from_utf8_lossy(&out.stdout)).0
    }
    #[cfg(target_os = "linux")]
    {
        let until = Instant::now() + Duration::from_millis(ms);
        let mut last = linux_sample_once(pid).0;
        while Instant::now() < until {
            std::thread::sleep(Duration::from_millis(SAMPLE_MS));
            last = linux_sample_once(pid).0;
        }
        last
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = (pid, ms);
        0
    }
}

#[cfg(target_os = "linux")]
fn linux_sampler(
    pid: u32,
    deadline: Instant,
    started: Instant,
    csv: &mut std::fs::File,
) -> Result<(u64, u64), String> {
    let status = format!("/proc/{pid}/status");
    let (mut ws, mut pv) = (0u64, 0u64);
    while Instant::now() < deadline {
        let (next_ws, next_pv) = linux_sample_once(pid);
        if next_ws == 0 && next_pv == 0 && !std::path::Path::new(&status).exists() {
            break;
        }
        ws = ws.max(next_ws);
        pv = pv.max(next_pv);
        writeln!(csv, "{},{next_ws},{next_pv}", started.elapsed().as_micros())
            .map_err(|e| e.to_string())?;
        std::thread::sleep(Duration::from_millis(SAMPLE_MS));
    }
    Ok((ws, pv))
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
                .next()
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
#[cfg(any(windows, test))]
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
