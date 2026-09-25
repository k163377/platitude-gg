//! `cargo xtask kill` and `cargo xtask launch` — the app's process
//! lifecycle, scoped to the tree this task runner was built in.
//!
//! An image-name kill (taskkill /IM, Stop-Process -Name) reaps every
//! seat's runs and the user's own window; `kill` reaps only processes
//! whose executable lives under this tree, and the pre-shell hook points
//! broad kills here.
//!
//! `launch` reaps this tree's stale runs, builds, starts detached from a
//! copy of the build ([`standing_copy`]) and says whether it lived past
//! the first second. The app separates its settings store by build tree
//! on its own (a seat's build locks its own `dev-<seat>`), so no store
//! juggling happens here.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::command::{self, Permission, Where};
use crate::wait::{LOOK_AGAIN, stood};

pub(crate) static KILL: command::Command = command::Command {
    id: "app.kill",
    call: "kill",
    purpose: "reap this tree's app processes, and nobody else's",
    run_in: Where::Either,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static LAUNCH: command::Command = command::Command {
    id: "app.launch",
    call: "launch",
    purpose: "start the app in a real window and confirm it outlived its first second",
    run_in: Where::Seat,
    needs: &[],
    permission: Permission::Escape(crate::hook::GUI_APPROVAL_FLAG),
};

pub(crate) static COMMANDS: &[&command::Command] = &[&KILL, &LAUNCH];

/// How long a window just started is watched before it is called
/// launched: a Qt platform plugin failure ends the process at once, and
/// the rest is margin for a cold start
/// (ci/baseline/code-costs-windows-x64.md §テストとハーネス). A fixed
/// stretch (`wait::stood`), because a window nothing drives says nothing
/// this could wait for.
const FIRST_MOMENT: Duration = Duration::from_millis(900);

/// `cargo xtask kill`. Nothing to reap is a success.
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

/// `cargo xtask launch [--no-build]`. The pre-shell hook requires
/// PGG_ALLOW_GUI=1 in front from a worktree.
pub fn launch(args: &[String]) -> Result<(), String> {
    let mut build = true;
    for arg in args {
        match arg.as_str() {
            "--no-build" => build = false,
            other => return Err(format!("launch does not take {other:?}")),
        }
    }
    let root = crate::tree::workspace_root();
    // Somebody is waiting at the screen: ahead of every test, behind a
    // landing. The ticket covers the build and the start only — the window
    // holds none of the machine.
    let _room = crate::budget::standalone(
        &root,
        if build {
            crate::budget::COMPILE
        } else {
            crate::budget::LIGHT
        },
        crate::budget::Rank::Launch,
        "launch",
    )?;
    let path = crate::qt::path_with_qt()?;
    // A stale run of this tree holds both the exe (against the link) and
    // this tree's own store lock (the app would open on the refusal gate).
    for (pid, exe) in reap_under(&root)? {
        println!("reaped this tree's stale run first: {pid} ({exe})");
    }
    let exe = stand_from_a_copy(&root, &crate::app_build::app_exe(&root, &path, build, &[])?)?;
    let mut command = Command::new(&exe);
    command
        .current_dir(&root)
        .env("PATH", &path)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // The window's git is the user's and does not answer to the gate on
    // main (gate::hooks) — true only of a window nothing drives, so the
    // automation knobs go first.
    crate::app_env::clear_automation(&mut command);
    command.env_remove(crate::gate::SESSION);
    let mut child = command
        .spawn()
        .map_err(|e| format!("failed to start {}: {e}", exe.display()))?;
    match stood(FIRST_MOMENT, LOOK_AGAIN, || child.try_wait().transpose()) {
        Ok(_stood_for) => {
            println!("launched pid={} ({})", child.id(), exe.display());
            Ok(())
        }
        Err(Ok(status)) => Err(format!(
            "the window exited at once ({status}) — is Qt's bin on PATH?"
        )),
        Err(Err(error)) => Err(format!("could not read the run's state: {error}")),
    }
}

const APP_NAME: &str = "platitude-gg";

/// Where a launched window runs from: a copy of the build, in a directory
/// cargo does not link into.
///
/// On Windows a running exe holds its file, and the release binary is a
/// hard link to the one in `deps/`, so a window standing on the build
/// fails the next link — and the gate's first verb builds
/// ([`crate::tree`]) while the user's window, the one run a gate leaves
/// standing, is still up.
///
/// The file name stays the app's ([`app_processes`] enumerates by it) and
/// the directory stays under this tree ([`is_under`]), so `kill` still
/// reaps it. One copy per tree is enough: `launch` reaps this tree's runs
/// before it writes the copy.
fn standing_copy(root: &Path) -> PathBuf {
    root.join("target")
        .join("window")
        .join(crate::app_build::exe_name())
}

/// [`standing_copy`], refreshed from `built` every launch.
fn stand_from_a_copy(root: &Path, built: &Path) -> Result<PathBuf, String> {
    let copy = standing_copy(root);
    if let Some(dir) = copy.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::copy(built, &copy).map_err(|e| {
        format!(
            "could not copy {} to {}: {e} — something is still standing on it \
             (`cargo xtask kill` reaps this tree's runs)",
            built.display(),
            copy.display()
        )
    })?;
    Ok(copy)
}

/// Every app process whose executable sits under `root`: what
/// [`reap_under`] kills, and what a red release build names as possibly
/// holding the file it was linking (`crate::app_build::app_exe`).
pub(crate) fn standing_under(root: &Path) -> Result<Vec<(u32, String)>, String> {
    Ok(app_processes()?
        .into_iter()
        .filter(|(_, exe)| is_under(exe, root))
        .collect())
}

pub(crate) fn reap_under(root: &Path) -> Result<Vec<(u32, String)>, String> {
    let mine = standing_under(root)?;
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
    // A process that ended since the listing is a success; anything else
    // is reported — a survivor claimed as reaped sends the launch into the
    // still-locked exe.
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
    use super::{is_under, parse_pid_paths, standing_copy};
    use std::path::{Path, PathBuf};

    /// A name the process listing does not know is a window `kill` walks
    /// past.
    #[test]
    fn the_window_stands_beside_the_slot_and_stays_reapable() {
        let seat = PathBuf::from("C:/x/platitude-gg/.claude/worktrees/a");
        let copy = standing_copy(&seat);
        assert_ne!(
            copy,
            seat.join("target")
                .join("release")
                .join(crate::app_build::exe_name())
        );
        assert_eq!(
            copy.file_name(),
            Some(std::ffi::OsStr::new(crate::app_build::exe_name()))
        );
        assert!(is_under(&copy.to_string_lossy(), &seat));
    }

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
