//! Ending a step and everything it started.
//!
//! [`Child::kill`] reaches the process this runner spawned and nothing
//! below it. A cargo ended at `check`'s ceiling leaves the rustc it was
//! waiting on running, and that rustc goes on holding this tree's build
//! lock: the next gate here waits on the lock with nothing to say why,
//! which reads as another session's cargo rather than as the step this
//! one killed.
//!
//! The two systems are asked differently, because only one of them keeps
//! a tree that can be trusted:
//!
//! * **unix** — the step is started in a process group of its own
//!   ([`own_group`]), and the group is what is ended. A group is a kernel
//!   object rather than a number written on a child, so nothing under the
//!   step escapes it, nothing outside it can be caught by it, and SIGKILL
//!   to the group cannot be refused. What it costs is the terminal's own
//!   signals: a step in a group of its own does not take the Ctrl-C typed
//!   at this runner, and is ended by the ceiling instead. A step that
//!   puts *itself* in a third group is out of reach either way; nothing
//!   run here does.
//! * **windows** — there is no group to take, and no job object to take
//!   either: the calls that make one are Win32, this crate's dependencies
//!   are std alone (CLAUDE.md 技術スタック), and the one job object here
//!   is made inside a PowerShell that compiles C# to reach them
//!   (`perf::sampler`) — which on this path would be a cost per step,
//!   paid for a kill only an emergency reaches. So the tree is walked
//!   instead, and the walk cannot trust what it reads: a dead parent's
//!   number stays written on its children and is handed out again, so
//!   dozens of a desktop's processes name a parent that is gone — an IDE,
//!   a shell, the file manager among them. A walk that followed the
//!   number alone (`taskkill /T`, which follows nothing else) would reap
//!   one of those the first time a step was handed that number, so a
//!   process counts as a descendant only if it also started no earlier
//!   than the parent it names.
//!
//! The root goes first either way, so that what is being walked can start
//! nothing more, and on Windows the walk is taken again after it: a child
//! started between the first look and the kill still names the root's
//! number, which stays this runner's to read until it waits on the handle.

use std::process::{Child, Command};

/// What ending a step's tree came to.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Reaped {
    /// The processes that were running under the step, its own not
    /// counted — None where that could not be found out at all, which is
    /// the one outcome that leaves the step's own children running.
    pub(crate) under: Option<Vec<u32>>,
    /// The ones still there afterwards — always empty where the group
    /// answers for them.
    pub(crate) left: Vec<u32>,
}

impl Reaped {
    /// How this reads in the failure that ended the step. Said even when
    /// it is nothing: a ceiling that names no survivors is the difference
    /// between the next stall being this run's fault and being news — so
    /// a look that could not be taken says that, rather than passing for
    /// a step that started nothing.
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

/// Starts what `command` runs where it can be ended with everything it
/// starts. A no-op where the tree is walked instead of grouped.
#[cfg(unix)]
pub(crate) fn own_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    // 0 asks for a group of the child's own, named by its own pid.
    command.process_group(0);
}

#[cfg(not(unix))]
pub(crate) fn own_group(_command: &mut Command) {}

/// Ends `child` and everything under it, and reaps the child itself.
pub(crate) fn kill_tree(child: &mut Child) -> Reaped {
    let reaped = end_tree(child);
    // Last, and after the walk on Windows: waiting is what hands the
    // child's number back to the machine.
    if let Err(error) = child.wait() {
        println!("  note: could not reap the step's own process ({error})");
    }
    reaped
}

#[cfg(unix)]
fn end_tree(child: &mut Child) -> Reaped {
    let root = child.id();
    let groups = groups();
    // Only a step that leads a group of its own may be ended by group:
    // `kill -<n>` with any other number names a group this runner is in,
    // and would end the run doing the killing.
    let leads = groups
        .iter()
        .any(|(pid, group)| (*pid, *group) == (root, root));
    let under: Vec<u32> = groups
        .iter()
        .filter(|(pid, group)| *group == root && *pid != root)
        .map(|(pid, _)| *pid)
        .collect();
    if leads {
        end_group(root);
    }
    // Always, and not only where there was no group to end: a group kill
    // that did not land would otherwise leave the step running, and the
    // wait after this would never come back.
    if let Err(error) = child.kill() {
        println!("  note: could not end the step ({error})");
    }
    // Nothing is asked about afterwards: a SIGKILL to the group cannot be
    // refused, and a child of the step that is ended but not yet reaped
    // by the machine still answers a liveness probe.
    Reaped {
        under: leads.then_some(under),
        left: Vec::new(),
    }
}

/// Ends the process group `root` leads.
#[cfg(unix)]
fn end_group(root: u32) {
    let mut command = Command::new("kill");
    // `--` before the group: the `kill` on PATH is not the shell's, and
    // it reads a leading `-1234` as another signal to send rather than as
    // the group to send it to.
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

/// Every process still running and the group it is in. A process the
/// machine has yet to reap is left out: it is in every listing until
/// whoever inherited it collects it, and under a container's pid 1 that
/// can be for as long as the run lasts — but it runs no code and holds no
/// lock, which is all anything here asks.
#[cfg(unix)]
fn groups() -> Vec<(u32, u32)> {
    let mut command = Command::new("ps");
    // One `-o` per field: a comma-joined list with empty headings is read
    // differently by the two `ps` this has to run under.
    command.args(["-A", "-o", "pid=", "-o", "pgid=", "-o", "stat="]);
    let Ok(out) = crate::subprocess::run_captured(&mut command) else {
        return Vec::new();
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let process = (fields.next()?.parse().ok()?, fields.next()?.parse().ok()?);
            let reaped = fields.next()?.starts_with('Z');
            (!reaped).then_some(process)
        })
        .collect()
}

#[cfg(windows)]
fn end_tree(child: &mut Child) -> Reaped {
    let root = child.id();
    let before = snapshot();
    // A root the listing does not have is a step already gone, or a
    // listing that could not be taken. Either way the numbers under it
    // are not this runner's to read.
    let Some(born) = before
        .iter()
        .find(|process| process.pid == root)
        .map(|process| process.born)
    else {
        if let Err(error) = child.kill() {
            println!("  note: could not end the step ({error})");
        }
        return Reaped::default();
    };
    let mut under = descendants(&before, root, born);
    if let Err(error) = child.kill() {
        println!("  note: could not end the step ({error})");
    }
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

/// Every process, the number of the parent it names, and when it started.
/// The listing is Windows' only answer to what is under a process:
/// `tasklist` does not carry a parent, and `wmic` is on its way off the
/// system.
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
            // A process the listing could not date is in no tree at all:
            // without a start time, nothing tells the parent that made it
            // from the parent whose number it was handed afterwards.
            Some(Process {
                pid: fields.next()?.parse().ok()?,
                parent: fields.next()?.parse().ok()?,
                born: fields.next()?.parse().ok()?,
            })
        })
        .collect()
}

/// What is under `root`, which started at `born`: the processes reachable
/// from it by parent, each of which started no earlier than the parent it
/// names.
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
    use std::time::{Duration, Instant};

    use super::{Reaped, kill_tree, own_group};

    /// How long the ended processes are given to be gone. Generous: on
    /// Windows the walk itself is a listing per look, and on unix the
    /// machine reaps the step's orphans in its own time.
    const GONE_WITHIN: Duration = Duration::from_secs(30);

    #[test]
    fn what_was_reaped_is_said_whether_or_not_anything_was() {
        // A look that could not be taken is the one outcome that must
        // not read as a step that started nothing: the survivors of it
        // are what the next run waits on.
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

    /// The trap the start time is there for: a process that names the
    /// step's number as its parent but was running before the step had
    /// that number is somebody else's, and is not in the tree.
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

    /// A line the listing could not date names no tree.
    #[cfg(windows)]
    #[test]
    fn a_process_the_listing_could_not_date_is_left_out() {
        assert!(super::listed("100 4\n200 100 \n300 100 x\n\n").is_empty());
    }

    /// A throwaway tree three deep: the step's own child and the child's
    /// child both go with it. The grandchild is the one `Child::kill`
    /// cannot reach, and it is the shape that holds a build lock.
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
        let under = settled(child.id());
        assert!(
            under.len() >= 2,
            "the tree never got two deep under the step: {under:?}"
        );
        let reaped = kill_tree(&mut child);
        assert!(
            reaped.under.as_ref().is_some_and(|under| under.len() >= 2),
            "the kill walked less of the tree than the look did: {reaped:?}"
        );
        // Asked until they are gone rather than at once: a process ended
        // this instant can still be in a listing taken the next.
        let deadline = Instant::now() + GONE_WITHIN;
        let mut standing = still_running(&under);
        while !standing.is_empty() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(200));
            standing = still_running(&under);
        }
        assert!(
            standing.is_empty(),
            "still running under the step: {standing:?}"
        );
    }

    /// Which of `pids` are still running — the machine's leftovers left
    /// out on unix, where the step's orphans stay in the listing until
    /// whoever inherited them collects them.
    #[cfg(windows)]
    fn still_running(pids: &[u32]) -> Vec<u32> {
        pids.iter()
            .copied()
            .filter(|pid| crate::subprocess::process_exists(*pid))
            .collect()
    }

    #[cfg(unix)]
    fn still_running(pids: &[u32]) -> Vec<u32> {
        let listing = super::groups();
        pids.iter()
            .copied()
            .filter(|pid| listing.iter().any(|(other, _)| other == pid))
            .collect()
    }

    /// Three processes, one under the other, the last of them long enough
    /// running to be caught in the middle of it.
    #[cfg(windows)]
    fn tree_three_deep() -> Command {
        let mut command = Command::new("cmd");
        command.args(["/c", "cmd", "/c", "ping", "-n", "300", "127.0.0.1"]);
        command
    }

    /// Each level backgrounds the next and waits on it: a shell whose
    /// script is one plain command replaces itself with it, and the tree
    /// would be one process deep instead of three.
    #[cfg(unix)]
    fn tree_three_deep() -> Command {
        let mut command = Command::new("sh");
        command.args(["-c", "sh -c \"sleep 300 & wait\" & wait"]);
        command
    }

    /// What is under `root` once the tree has had time to grow, asked the
    /// way the kill asks.
    #[cfg(windows)]
    fn settled(root: u32) -> Vec<u32> {
        let deadline = Instant::now() + GONE_WITHIN;
        loop {
            let listed = super::snapshot();
            let born = listed
                .iter()
                .find(|process| process.pid == root)
                .map(|process| process.born);
            let under = born.map_or_else(Vec::new, |born| super::descendants(&listed, root, born));
            if under.len() >= 2 || Instant::now() >= deadline {
                return under;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    }

    #[cfg(unix)]
    fn settled(root: u32) -> Vec<u32> {
        let deadline = Instant::now() + GONE_WITHIN;
        loop {
            let under: Vec<u32> = super::groups()
                .iter()
                .filter(|(pid, group)| *group == root && *pid != root)
                .map(|(pid, _)| *pid)
                .collect();
            if under.len() >= 2 || Instant::now() >= deadline {
                return under;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    }
}
