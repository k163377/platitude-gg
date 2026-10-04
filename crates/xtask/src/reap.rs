//! Ending a step and everything it started.
//!
//! [`Child::kill`] reaches the spawned process and nothing below it: a
//! cargo ended at `check`'s ceiling leaves its rustc holding this tree's
//! build lock, and the next gate waits on it with nothing to say why.
//!
//! * **unix** — a step that leads a group of its own ([`own_group`]) is
//!   ended by that group: a kernel object, so nothing under the step
//!   escapes it and SIGKILL cannot be refused. A step left in this
//!   runner's group is reached by walking parents, which needs no start
//!   time: the kernel re-points an orphan at init when its parent goes, so
//!   no living process names a number handed out again.
//! * **windows** — no group, and no job object: that takes Win32, this
//!   crate is std alone (CLAUDE.md 技術スタック), and the PowerShell-made
//!   one (`perf::sampler`) would cost a process per step. So the tree is
//!   walked, and a dead parent's number stays on its children and is
//!   handed out again — a walk by number alone (`taskkill /T`) reaps
//!   strangers. A process counts as a descendant only if it started no
//!   earlier than the parent it names.
//!
//! The root goes first, so what is walked can start nothing more. On
//! Windows the walk is taken again after it: a child started in between
//! still names the root's number, which stays this runner's until it
//! waits on the handle. On unix the kill re-points such a child at init,
//! and it is left to its own end.

use std::process::{Child, Command};

/// What ending a step's tree came to.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Reaped {
    /// The processes that were under the step, its own not counted — None
    /// where that could not be found out, which leaves its children running.
    pub(crate) under: Option<Vec<u32>>,
    /// The ones still there afterwards — always empty on unix, where a
    /// signal that has been sent cannot be refused.
    pub(crate) left: Vec<u32>,
}

impl Reaped {
    /// How this reads in the failure that ended the step — said even when
    /// it is nothing, so the next stall can be told from this run's fault.
    pub(crate) fn line(&self) -> String {
        let Some(under) = self.under.as_ref() else {
            return "what it started could not be looked up and may hold this side's build lock"
                .to_string();
        };
        let went = match under.len() {
            0 => "nothing was running under it".to_string(),
            1 => "the one process under it went with it".to_string(),
            n => format!("the {n} processes under it went with it"),
        };
        if self.left.is_empty() {
            return went;
        }
        let left: Vec<String> = self.left.iter().map(u32::to_string).collect();
        format!(
            "{went}, except {} that would not go and may hold this side's build lock: {}",
            self.left.len(),
            left.join(", ")
        )
    }
}

/// Starts what `command` runs in a group of its own, which [`reap`] ends
/// whole. No signal aimed at this runner reaches the group — the
/// terminal's Ctrl-C, a ceiling above this one — so it is left to the
/// ceiling that calls [`reap`]. A no-op where the tree is walked.
#[cfg(unix)]
pub(crate) fn own_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    // 0 asks for a group of the child's own, named by its own pid.
    command.process_group(0);
}

#[cfg(not(unix))]
pub(crate) fn own_group(_command: &mut Command) {}

/// Who, out of what this runner started, is still running — for a
/// caller that must not take a directory away from a process still
/// writing into it. Only unix has to ask: there a directory with a file
/// open in it goes all the same. `None` is "could not be found out", not
/// "nobody".
///
/// Read from `/proc`, not asked of `ps` ([`processes`]): that `ps` would
/// be in this group and its own listing. By group, not by parent: a
/// verified run's app and all it spawned stay in this runner's group
/// (`verify::child`) through the re-parenting after the app's exit, which
/// a parent walk cannot see past. It errs towards somebody — from a shell
/// the group holds the rest of the pipeline too — which costs only a
/// sweep.
#[cfg(target_os = "linux")]
pub(crate) fn others_in_this_group() -> Option<Vec<u32>> {
    others_in_group_under(std::path::Path::new("/proc"))
}

/// [`others_in_this_group`] against a given `/proc`, so a test can arrange
/// an unreadable listing or `stat`. A process gone between the listing and
/// the read is not running, whether it went before the open or after it;
/// any other failure to read makes the whole answer `None`.
#[cfg(target_os = "linux")]
pub(crate) fn others_in_group_under(proc: &std::path::Path) -> Option<Vec<u32>> {
    let me = std::process::id();
    let Standing::In(mine) = standing_of(proc, me) else {
        return None;
    };
    let mut others = Vec::new();
    for entry in std::fs::read_dir(proc).ok()? {
        // An error part way through is no answer, not a shorter listing.
        let entry = entry.ok()?;
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        if pid == me {
            continue;
        }
        match standing_of(proc, pid) {
            Standing::In(group) if group == mine => others.push(pid),
            Standing::In(_) | Standing::Over => {}
            Standing::Unreadable => return None,
        }
    }
    Some(others)
}

/// What `/proc/<pid>/stat` said about a process.
#[cfg(target_os = "linux")]
enum Standing {
    /// Gone since the listing, or a zombie: it runs no code and holds
    /// nothing open.
    Over,
    /// Running, in the group given.
    In(u32),
    /// There, and not to be made sense of.
    Unreadable,
}

#[cfg(target_os = "linux")]
fn standing_of(proc: &std::path::Path, pid: u32) -> Standing {
    standing_read(std::fs::read_to_string(
        proc.join(pid.to_string()).join("stat"),
    ))
}

/// `ESRCH`, the same number on every Linux architecture.
#[cfg(target_os = "linux")]
const NO_SUCH_PROCESS: i32 = 3;

/// [`standing_of`] from what the read of a `stat` came to.
#[cfg(target_os = "linux")]
fn standing_read(read: std::io::Result<String>) -> Standing {
    let stat = match read {
        Ok(stat) => stat,
        // The two failures that are an answer: gone before the open
        // (`ENOENT`), and gone between the open and the read — the kernel
        // answers a `stat` whose task is no more with `ESRCH`, which the
        // standard library has no kind for.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Standing::Over,
        Err(error) if error.raw_os_error() == Some(NO_SUCH_PROCESS) => return Standing::Over,
        Err(_) => return Standing::Unreadable,
    };
    // Counted from the last `)`: the second field is the command, in
    // brackets, and a command may hold spaces and brackets of its own.
    let Some(after) = stat.rfind(')').map(|at| &stat[at + 1..]) else {
        return Standing::Unreadable;
    };
    let mut fields = after.split_whitespace();
    match (fields.next(), fields.next(), fields.next()) {
        (Some("Z"), _, _) => Standing::Over,
        (Some(_), Some(_), Some(group)) => match group.parse() {
            Ok(group) => Standing::In(group),
            Err(_) => Standing::Unreadable,
        },
        _ => Standing::Unreadable,
    }
}

/// Nobody, because Windows asks the question by removing: a directory
/// with a handle open in it refuses the removal.
#[cfg(windows)]
pub(crate) fn others_in_this_group() -> Option<Vec<u32>> {
    Some(Vec::new())
}

/// Not implemented: a removal on macOS does not refuse and there is no
/// `/proc`, so this answers "could not be found out" and the caller keeps
/// what it would have removed. The only Mac that verifies is CI's runner,
/// thrown away with what it kept; a desk that is one fills this in off
/// `sysctl`'s `KERN_PROC`.
#[cfg(not(any(target_os = "linux", windows)))]
pub(crate) fn others_in_this_group() -> Option<Vec<u32>> {
    None
}

/// Ends the step itself, which [`reap`] waits on afterwards either way.
fn end_step(child: &mut Child) {
    if let Err(error) = child.kill() {
        println!("  note: could not end the step ({error})");
    }
}

/// Ends `child` and everything under it, and reaps the child: what went
/// with it, and the child's own exit.
pub(crate) fn reap(child: &mut Child) -> (Reaped, Option<std::process::ExitStatus>) {
    let reaped = end_tree(child);
    // Last, and after the walk on Windows: waiting is what hands the
    // child's number back to the machine.
    let ended = match child.wait() {
        Ok(status) => Some(status),
        Err(error) => {
            println!("  note: could not reap the step's own process ({error})");
            None
        }
    };
    (reaped, ended)
}

#[cfg(unix)]
fn end_tree(child: &mut Child) -> Reaped {
    let root = child.id();
    let listing = processes();
    // A root the listing lacks (gone, or no listing): what was under it is
    // not this runner's to read.
    let Some(leads) = listing
        .iter()
        .find(|process| process.pid == root)
        .map(|process| process.group == root)
    else {
        end_step(child);
        return Reaped::default();
    };
    // Only a step that leads a group of its own may be ended by group:
    // `kill -<n>` with any other number names a group this runner is in,
    // and would end the run doing the killing.
    let under = if leads {
        let under: Vec<u32> = listing
            .iter()
            .filter(|process| process.group == root && process.pid != root)
            .map(|process| process.pid)
            .collect();
        end_group(root);
        // Always: a group kill that did not land would leave the step
        // running, and the wait after this would never come back.
        end_step(child);
        under
    } else {
        let under = descendants(&listing, root);
        end_step(child);
        end(&under);
        under
    };
    // Nothing is asked afterwards: a SIGKILL cannot be refused, and an
    // ended child not yet reaped still answers a liveness probe.
    Reaped {
        under: Some(under),
        left: Vec::new(),
    }
}

#[cfg(unix)]
fn end_group(root: u32) {
    let mut command = Command::new("kill");
    // `--` before the group: the `kill` on PATH is not the shell's, and
    // reads a leading `-1234` as another signal.
    command.args(["-9", "--", &format!("-{root}")]);
    match crate::subprocess::run_captured(&mut command) {
        Ok(out) if out.status.success() => {}
        Ok(out) => println!(
            "  note: the step's process group would not end ({})",
            String::from_utf8_lossy(&out.stderr).trim()
        ),
        Err(error) => println!("  note: could not end the step's process group ({error})"),
    }
}

/// Ends the walked processes, each by its own number: a `-<pid>` here
/// would name a process group this runner is in.
#[cfg(unix)]
fn end(pids: &[u32]) {
    if pids.is_empty() {
        return;
    }
    let mut command = Command::new("kill");
    command.arg("-9");
    command.args(pids.iter().map(u32::to_string));
    // The status is not read: a walked process that ended on its own
    // meanwhile makes `kill` exit non-zero.
    if let Err(error) = crate::subprocess::run_captured(&mut command) {
        println!("  note: could not end what was under the step ({error})");
    }
}

/// One process as the listing gives it.
#[cfg(unix)]
struct Process {
    pid: u32,
    group: u32,
    parent: u32,
}

/// Every process still running, with its group and the parent it names.
/// Zombies are left out: under a container's pid 1 one can stay listed
/// for the whole run, but it runs no code and holds no lock.
#[cfg(unix)]
fn processes() -> Vec<Process> {
    let mut command = Command::new("ps");
    // One `-o` per field: the two `ps` this runs under read a comma-joined
    // list with empty headings differently.
    command.args([
        "-A", "-o", "pid=", "-o", "pgid=", "-o", "ppid=", "-o", "stat=",
    ]);
    let Ok(out) = crate::subprocess::run_captured(&mut command) else {
        return Vec::new();
    };
    listed(&String::from_utf8_lossy(&out.stdout))
}

#[cfg(unix)]
fn listed(listing: &str) -> Vec<Process> {
    listing
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let process = Process {
                pid: fields.next()?.parse().ok()?,
                group: fields.next()?.parse().ok()?,
                parent: fields.next()?.parse().ok()?,
            };
            let reaped = fields.next()?.starts_with('Z');
            (!reaped).then_some(process)
        })
        .collect()
}

/// What is under `root`, by the parent each process names. Sharing
/// `root`'s group is not descent: this runner's own group is full of
/// processes it did not start.
#[cfg(unix)]
fn descendants(processes: &[Process], root: u32) -> Vec<u32> {
    let mut under: Vec<u32> = Vec::new();
    let mut growing = true;
    while growing {
        growing = false;
        for process in processes {
            if process.pid == root || under.contains(&process.pid) {
                continue;
            }
            if process.parent == root || under.contains(&process.parent) {
                under.push(process.pid);
                growing = true;
            }
        }
    }
    under
}

#[cfg(windows)]
fn end_tree(child: &mut Child) -> Reaped {
    let root = child.id();
    let before = snapshot();
    // A root the listing lacks (gone, or no listing): the numbers under it
    // are not this runner's to read.
    let Some(born) = before
        .iter()
        .find(|process| process.pid == root)
        .map(|process| process.born)
    else {
        end_step(child);
        return Reaped::default();
    };
    let mut under = descendants(&before, root, born);
    end_step(child);
    for pid in descendants(&snapshot(), root, born) {
        if !under.contains(&pid) {
            under.push(pid);
        }
    }
    end(&under);
    let left = under
        .iter()
        .copied()
        .filter(|pid| crate::subprocess::process_exists(*pid))
        .collect();
    Reaped {
        under: Some(under),
        left,
    }
}

/// One process as the listing gives it.
#[cfg(windows)]
struct Process {
    pid: u32,
    parent: u32,
    born: u64,
}

/// Every process, the parent it names, and when it started — from CIM:
/// `tasklist` carries no parent, and `wmic` is on its way off the system.
#[cfg(windows)]
fn snapshot() -> Vec<Process> {
    const LISTING: &str = "Get-CimInstance Win32_Process | ForEach-Object { \
                           '{0} {1} {2}' -f $_.ProcessId, $_.ParentProcessId, $_.CreationDate.Ticks }";
    let mut command = Command::new("powershell");
    command.args(["-NoProfile", "-NonInteractive", "-Command", LISTING]);
    let Ok(out) = crate::subprocess::run_captured(&mut command) else {
        return Vec::new();
    };
    listed(&String::from_utf8_lossy(&out.stdout))
}

#[cfg(windows)]
fn listed(listing: &str) -> Vec<Process> {
    listing
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            // An undated process is in no tree: without a start time, the
            // parent that made it cannot be told from a later holder of
            // the number.
            Some(Process {
                pid: fields.next()?.parse().ok()?,
                parent: fields.next()?.parse().ok()?,
                born: fields.next()?.parse().ok()?,
            })
        })
        .collect()
}

/// What is under `root`, which started at `born`, by parent — each
/// started no earlier than the parent it names.
#[cfg(windows)]
fn descendants(processes: &[Process], root: u32, born: u64) -> Vec<u32> {
    let mut tree: Vec<(u32, u64)> = vec![(root, born)];
    let mut under: Vec<u32> = Vec::new();
    let mut growing = true;
    while growing {
        growing = false;
        for process in processes {
            if process.pid == root || under.contains(&process.pid) {
                continue;
            }
            if tree
                .iter()
                .any(|(pid, born)| *pid == process.parent && process.born >= *born)
            {
                tree.push((process.pid, process.born));
                under.push(process.pid);
                growing = true;
            }
        }
    }
    under
}

/// Ends the walked processes, by number and never by name: a name reaches
/// every seat's runs and the user's own window (`gui`).
#[cfg(windows)]
fn end(pids: &[u32]) {
    if pids.is_empty() {
        return;
    }
    let mut command = Command::new("taskkill");
    command.arg("/F");
    for pid in pids {
        command.args(["/PID", &pid.to_string()]);
    }
    if let Err(error) = crate::subprocess::run_captured(&mut command) {
        println!("  note: could not end what was under the step ({error})");
    }
}

#[cfg(test)]
mod tests {
    use std::process::{Command, Stdio};

    use super::{Reaped, own_group, reap};
    use crate::wait::until;

    #[test]
    fn what_was_reaped_is_said_whether_or_not_anything_was() {
        // A look that could not be taken must not read as a step that
        // started nothing: its survivors are what the next run waits on.
        let unlooked = Reaped::default().line();
        assert!(unlooked.contains("could not be looked up"), "{unlooked}");
        assert!(unlooked.ends_with("build lock"), "{unlooked}");
        let nothing = Reaped {
            under: Some(Vec::new()),
            left: Vec::new(),
        };
        assert_eq!(nothing.line(), "nothing was running under it");
        let one = Reaped {
            under: Some(vec![7]),
            left: Vec::new(),
        };
        assert_eq!(one.line(), "the one process under it went with it");
        let stubborn = Reaped {
            under: Some(vec![7, 8, 9]),
            left: vec![8, 9],
        }
        .line();
        assert!(
            stubborn.starts_with("the 3 processes under it"),
            "{stubborn}"
        );
        assert!(stubborn.ends_with("build lock: 8, 9"), "{stubborn}");
    }

    /// The window a busy machine reads [`super::others_in_group_under`]
    /// through: a `stat` opened while its process ran and read after it was
    /// reaped. The kernel's own answer, not an arranged one.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_stat_whose_process_went_after_the_open_is_over() {
        use std::io::Read;

        let mut child = Command::new("sleep")
            .arg("30")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("a child to open the stat of");
        let mut stat = std::fs::File::open(format!("/proc/{}/stat", child.id()))
            .expect("the stat of a running child");
        let _ = child.kill();
        child.wait().expect("the child reaped");
        let mut text = String::new();
        let read = stat.read_to_string(&mut text).map(|_| text);

        let errno = read.as_ref().err().and_then(std::io::Error::raw_os_error);
        assert_eq!(
            errno,
            Some(super::NO_SUCH_PROCESS),
            "the read of a reaped process's stat came to {read:?}"
        );
        assert!(
            matches!(super::standing_read(read), super::Standing::Over),
            "a process that went after the open was taken for an unreadable listing"
        );
    }

    /// The trap the start time is there for: a process naming the step's
    /// number as its parent but started before the step is somebody else's.
    #[cfg(windows)]
    #[test]
    fn the_walk_leaves_what_named_the_number_before_the_step_held_it() {
        let listed = super::listed(
            "100 4 1000\n\
             200 100 1500\n\
             300 200 1600\n\
             400 100 900\n\
             500 4 1200\n",
        );
        let mut under = super::descendants(&listed, 100, 1000);
        under.sort_unstable();
        assert_eq!(under, vec![200, 300], "the stale parent's child was reaped");
    }

    #[cfg(windows)]
    #[test]
    fn a_process_the_listing_could_not_date_is_left_out() {
        assert!(super::listed("100 4\n200 100 \n300 100 x\n\n").is_empty());
    }

    /// In this runner's group: the step's tree however deep, not the
    /// processes merely sharing the group, and not the zombie under it.
    #[cfg(unix)]
    #[test]
    fn the_walk_takes_the_steps_tree_and_not_the_group_it_shares() {
        let listed = super::listed(
            "100 900 4 Ss\n\
             200 900 100 S\n\
             300 900 200 S\n\
             400 900 100 Z\n\
             500 900 4 S\n",
        );
        let mut under = super::descendants(&listed, 100);
        under.sort_unstable();
        assert_eq!(under, vec![200, 300], "not the tree the walk names");
    }

    /// The grandchild is the one `Child::kill` cannot reach, and the shape
    /// that holds a build lock.
    #[test]
    fn a_grandchild_of_the_step_goes_with_the_step() {
        let mut command = tree_three_deep();
        own_group(&mut command);
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("a throwaway process tree");
        let root = child.id();
        // Waited for: the tree grows in the shells' own time.
        let under = until(
            "two processes under the step",
            || under_the_step(root),
            |under| under.len() >= 2,
        );
        let (reaped, ended) = reap(&mut child);
        assert!(
            reaped.under.as_ref().is_some_and(|under| under.len() >= 2),
            "the kill walked less of the tree than the look did: {reaped:?}"
        );
        assert!(ended.is_some(), "the step itself was not reaped");
        // Asked until gone: a process ended this instant can still be in
        // the next listing.
        until(
            "nothing left under the step",
            || still_running(&under),
            Vec::is_empty,
        );
    }

    /// The same tree left in this runner's group: the walk reaches it, as a
    /// kill by group would end the test itself.
    #[cfg(unix)]
    #[test]
    fn a_grandchild_of_a_step_in_this_runners_group_goes_with_the_step() {
        let mut command = tree_three_deep();
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("a throwaway process tree");
        let root = child.id();
        let under = until(
            "two processes under the step",
            || under_by_parent(root),
            |under| under.len() >= 2,
        );
        let (reaped, ended) = reap(&mut child);
        let walked = reaped.under.clone().unwrap_or_default();
        assert!(
            under.iter().all(|pid| walked.contains(pid)),
            "the kill walked less of the tree than the look did: {reaped:?}"
        );
        assert!(
            !walked.contains(&std::process::id()),
            "the walk followed the group as far as this runner: {reaped:?}"
        );
        assert!(ended.is_some(), "the step itself was not reaped");
        until(
            "nothing left under the step",
            || still_running(&under),
            Vec::is_empty,
        );
    }

    /// Which of `pids` are still running — zombies not counted on unix
    /// (`processes`).
    #[cfg(windows)]
    fn still_running(pids: &[u32]) -> Vec<u32> {
        pids.iter()
            .copied()
            .filter(|pid| crate::subprocess::process_exists(*pid))
            .collect()
    }

    #[cfg(unix)]
    fn still_running(pids: &[u32]) -> Vec<u32> {
        let listing = super::processes();
        pids.iter()
            .copied()
            .filter(|pid| listing.iter().any(|process| process.pid == *pid))
            .collect()
    }

    #[cfg(windows)]
    fn tree_three_deep() -> Command {
        let mut command = Command::new("cmd");
        command.args(["/c", "cmd", "/c", "ping", "-n", "300", "127.0.0.1"]);
        command
    }

    /// Each level backgrounds the next and waits on it: a shell whose
    /// script is one plain command replaces itself with it.
    #[cfg(unix)]
    fn tree_three_deep() -> Command {
        let mut command = Command::new("sh");
        command.args(["-c", "sh -c \"sleep 300 & wait\" & wait"]);
        command
    }

    /// What is under `root` right now, asked the way the kill asks.
    #[cfg(windows)]
    fn under_the_step(root: u32) -> Vec<u32> {
        let listed = super::snapshot();
        let born = listed
            .iter()
            .find(|process| process.pid == root)
            .map(|process| process.born);
        born.map_or_else(Vec::new, |born| super::descendants(&listed, root, born))
    }

    #[cfg(unix)]
    fn under_the_step(root: u32) -> Vec<u32> {
        super::processes()
            .iter()
            .filter(|process| process.group == root && process.pid != root)
            .map(|process| process.pid)
            .collect()
    }

    /// What is under `root` by the parent each process names, asked the
    /// way the walk asks.
    #[cfg(unix)]
    fn under_by_parent(root: u32) -> Vec<u32> {
        super::descendants(&super::processes(), root)
    }
}
