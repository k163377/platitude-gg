//! `cargo xtask kill` and `cargo xtask launch` — the app's process
//! lifecycle, scoped to the tree this task runner was built in.
//!
//! Kills are the dangerous half: an image-name kill (taskkill /IM,
//! Stop-Process -Name) reaps every seat's runs and the user's own window
//! in one line, and that is exactly what a session reaches for when a
//! stale run holds this tree's exe against the next link. `kill` reaps
//! only processes whose executable lives under this tree, and the
//! pre-shell hook points broad kills here.
//!
//! `launch` is the real-window start (the verify-ui skill's fast path):
//! reap this tree's stale runs, build, start detached, and say whether
//! it lived past the first second. The app separates its settings store
//! by build tree on its own (a seat's build locks `dev-<seat>`, never
//! another seat's), so no store juggling happens here.

use std::path::Path;
use std::process::{Command, Stdio};

/// `cargo xtask kill`: reap this tree's app processes. Quiet success when
/// there is nothing to reap.
pub fn kill(args: &[String]) -> Result<(), String> {
    if !args.is_empty() {
        return Err(format!("kill takes no arguments (got {args:?})"));
    }
    let root = crate::tree::workspace_root();
    let reaped = reap_under(&root)?;
    if reaped.is_empty() {
        println!("no {APP_NAME} process under {} to reap.", root.display());
    } else {
        for (pid, path) in &reaped {
            println!("reaped {pid} ({path})");
        }
    }
    Ok(())
}

/// `cargo xtask launch [--no-build]`: start a real window from this tree.
/// The pre-shell hook requires PG_ALLOW_GUI=1 in front from a worktree —
/// a real window is the user's ask, never routine verification.
pub fn launch(args: &[String]) -> Result<(), String> {
    let mut build = true;
    for arg in args {
        match arg.as_str() {
            "--no-build" => build = false,
            other => return Err(format!("launch does not take {other:?}")),
        }
    }
    let root = crate::tree::workspace_root();
    let path = crate::qt::path_with_qt()?;
    // A stale run of this tree holds both the exe (against the link) and
    // this tree's own store lock (the app would open on the refusal gate).
    for (pid, exe) in reap_under(&root)? {
        println!("reaped this tree's stale run first: {pid} ({exe})");
    }
    let exe = crate::tree::app_exe(&root, &path, build, &[])?;
    let mut command = Command::new(&exe);
    command
        .current_dir(&root)
        .env("PATH", &path)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // The window is the user's to drive, so the git it runs is the user's
    // too: it answers to the gate on main no more than the git in their
    // IDE does (gate::hooks). Which holds only of a window nothing is
    // driving — so the automation knobs go first, as they do for every
    // other app child started here, and the harness stays inert.
    crate::app_env::clear_automation(&mut command);
    command.env_remove(crate::gate::SESSION);
    let child = command
        .spawn()
        .map_err(|e| format!("failed to start {}: {e}", exe.display()))?;
    // Long enough for a Qt platform plugin failure to have ended the
    // process (that death takes ~10ms; the margin is for a cold start).
    std::thread::sleep(std::time::Duration::from_millis(900));
    let mut child = child;
    match child.try_wait() {
        Ok(None) => {
            println!("launched pid={} ({})", child.id(), exe.display());
            Ok(())
        }
        Ok(Some(status)) => Err(format!(
            "the window exited at once ({status}) — is Qt's bin on PATH?"
        )),
        Err(error) => Err(format!("could not read the run's state: {error}")),
    }
}

const APP_NAME: &str = "platitude-gg";

/// Kills every app process whose executable sits under `root`, and
/// answers who they were. Enumeration is per-OS; the path judgement is
/// one place, here.
fn reap_under(root: &Path) -> Result<Vec<(u32, String)>, String> {
    let mine: Vec<(u32, String)> = app_processes()?
        .into_iter()
        .filter(|(_, exe)| is_under(exe, root))
        .collect();
    for (pid, _) in &mine {
        kill_pid(*pid)?;
    }
    Ok(mine)
}

/// Whether `exe` (any slash direction) lives under `root`.
fn is_under(exe: &str, root: &Path) -> bool {
    let exe = exe.replace('\\', "/").to_lowercase();
    let mut root = root.to_string_lossy().replace('\\', "/").to_lowercase();
    if !root.ends_with('/') {
        root.push('/');
    }
    exe.starts_with(&root)
}

/// Every running app process as (pid, executable path).
#[cfg(windows)]
fn app_processes() -> Result<Vec<(u32, String)>, String> {
    // PowerShell is the one stock tool that answers with the image path;
    // the filtering stays in Rust where quoting cannot bend it.
    let output = crate::subprocess::run_captured(Command::new("powershell").args([
        "-NoProfile",
        "-Command",
        "Get-Process platitude-gg -ErrorAction SilentlyContinue | ForEach-Object { \"$($_.Id)\t$($_.Path)\" }",
    ]))?;
    Ok(parse_pid_paths(&String::from_utf8_lossy(&output.stdout)))
}

#[cfg(not(windows))]
fn app_processes() -> Result<Vec<(u32, String)>, String> {
    if Path::new("/proc").is_dir() {
        return Ok(proc_scan());
    }
    // macOS: comm is the full executable path for processes started by
    // path, which every launch of a built binary is.
    let output = crate::subprocess::run_captured(Command::new("ps").args(["-axo", "pid=,comm="]))?;
    let listing = String::from_utf8_lossy(&output.stdout);
    Ok(listing
        .lines()
        .filter_map(|line| {
            let (pid, comm) = line.trim().split_once(' ')?;
            let comm = comm.trim();
            (Path::new(comm).file_name()? == APP_NAME)
                .then(|| Some((pid.parse().ok()?, comm.to_string())))?
        })
        .collect())
}

#[cfg(not(windows))]
fn proc_scan() -> Vec<(u32, String)> {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let pid: u32 = entry.file_name().to_str()?.parse().ok()?;
            let exe = std::fs::read_link(entry.path().join("exe")).ok()?;
            (exe.file_name()? == APP_NAME).then(|| (pid, exe.to_string_lossy().into_owned()))
        })
        .collect()
}

#[cfg(any(windows, test))]
fn parse_pid_paths(listing: &str) -> Vec<(u32, String)> {
    listing
        .lines()
        .filter_map(|line| {
            let (pid, path) = line.trim().split_once('\t')?;
            let path = path.trim();
            (!path.is_empty()).then(|| Some((pid.parse().ok()?, path.to_string())))?
        })
        .collect()
}

fn kill_pid(pid: u32) -> Result<(), String> {
    let mut command = if cfg!(windows) {
        let mut c = Command::new("taskkill");
        c.args(["/F", "/PID", &pid.to_string()]);
        c
    } else {
        let mut c = Command::new("kill");
        c.args(["-9", &pid.to_string()]);
        c
    };
    let output = crate::subprocess::run_captured(&mut command)?;
    if output.status.success() {
        return Ok(());
    }
    // A process that ended between the listing and this line is a
    // success; anything else (access denied, a wedged handle) has to be
    // reported — "reaped" claimed over a survivor sends the launch
    // straight into the still-locked exe.
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if said.contains("not found") || said.contains("No such process") {
        return Ok(());
    }
    Err(format!("could not kill pid {pid}: {}", said.trim()))
}

#[cfg(test)]
mod tests {
    use super::{is_under, parse_pid_paths};
    use std::path::{Path, PathBuf};

    #[test]
    fn judges_the_tree_by_path_prefix_whatever_the_slashes() {
        let seat = PathBuf::from("C:/x/platitude-gg/.claude/worktrees/a");
        assert!(is_under(
            "C:\\x\\platitude-gg\\.claude\\worktrees\\a\\target\\release\\platitude-gg.exe",
            &seat
        ));
        assert!(!is_under(
            "C:/x/platitude-gg/target/release/platitude-gg.exe",
            &seat
        ));
        // A sibling seat whose name extends this one is not this one.
        assert!(!is_under(
            "C:/x/platitude-gg/.claude/worktrees/ab/target/release/platitude-gg.exe",
            &seat
        ));
        assert!(is_under(
            "/home/x/platitude-gg/target/release/platitude-gg",
            Path::new("/home/x/platitude-gg")
        ));
    }

    #[test]
    fn reads_pid_and_path_lines_and_skips_the_pathless() {
        let listing = "1234\tC:\\x\\target\\release\\platitude-gg.exe\r\n99\t\r\nnot a line\n";
        let parsed = parse_pid_paths(listing);
        assert_eq!(
            parsed,
            vec![(1234, "C:\\x\\target\\release\\platitude-gg.exe".to_string())]
        );
    }
}
