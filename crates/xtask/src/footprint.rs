//! `cargo xtask footprint <command…>` — what the machine is holding
//! while something runs, on one time axis.
//!
//! **Why a verb and not a script.** The question it exists for is "does
//! repeating a gate make the VM grow", and that question cannot be
//! answered from one side: `vmmemWSL`'s working set is what Windows has
//! lost, `/proc/meminfo` inside the distro is what Linux thinks it is
//! using, and neither is the other. Memory is not namespaced, so the
//! distro's own `/proc/meminfo` is the whole VM, containers included —
//! which is why a sum over `docker stats` is not a substitute.
//!
//! **It owns what it starts.** Two long-lived children do the sampling,
//! one per side, and both are killed when the command ends; a sample
//! taken by starting a process per tick would cost more than it reads.
//! What they write is kept under `target/footprint/<run>/` and swept by
//! nothing — the raw rows are the evidence, and a summary that replaced
//! them could not be checked.
//!
//! **`MemFree` is not the reading to judge by.** It is free pages only;
//! a VM whose cache has grown looks alarming in it and is not. The
//! column that says whether memory is actually gone is `MemAvailable`,
//! and `Cached` / `SReclaimable` / `AnonPages` say which kind grew.

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use crate::command::{self, Permission, Where};

pub(crate) static FOOTPRINT: command::Command = command::Command {
    id: "footprint.watch",
    call: "footprint [--settle <seconds>] <command…>",
    purpose: "run a command and record what the host, the VM and docker hold while it does",
    run_in: Where::Seat,
    needs: &["docker running, for the container side of the reading"],
    permission: Permission::Plain,
};

pub(crate) static COMMANDS: &[&command::Command] = &[&FOOTPRINT];

/// How often each side reports. Two seconds is short against a gate's
/// minutes and long against the cost of a line.
const EVERY: Duration = Duration::from_secs(2);

/// The distro the containers run in. Its `/proc/meminfo` is the VM's.
const DISTRO: &str = "docker-desktop";

/// How long the samplers stay up after the command ends. Long enough
/// for a VM that means to hand memory back to have started
/// (`autoMemoryReclaim` moves in tens of seconds), short enough that
/// three runs back to back still read as three runs.
const SETTLE: Duration = Duration::from_secs(60);

/// How long the samplers stay up after the command, and what is left to
/// run. `--settle <seconds>` buys a longer window when the question is
/// about what happens *after* the work rather than during it.
fn settle_for(args: &[String]) -> Result<(Duration, Vec<String>), String> {
    let Some(at) = args.iter().position(|a| a == "--settle") else {
        return Ok((SETTLE, args.to_vec()));
    };
    let seconds: u64 = args
        .get(at + 1)
        .ok_or("--settle wants a number of seconds")?
        .parse()
        .map_err(|_| "--settle wants a number of seconds".to_string())?;
    let mut rest = args.to_vec();
    rest.drain(at..=at + 1);
    Ok((Duration::from_secs(seconds), rest))
}

pub fn run(args: &[String]) -> Result<(), String> {
    let (settle, args) = settle_for(args)?;
    let args = &args[..];
    if args.is_empty() {
        return Err(format!("{} — e.g. `footprint gate --all`", FOOTPRINT.call));
    }
    let root = crate::tree::workspace_root();
    let at = crate::note::now_secs();
    let kept = root
        .join("target")
        .join("footprint")
        .join(format!("{at}-{}", std::process::id()));
    std::fs::create_dir_all(&kept).map_err(|e| format!("{}: {e}", kept.display()))?;
    std::fs::write(kept.join("command.txt"), args.join(" "))
        .map_err(|e| format!("{}: {e}", kept.display()))?;
    println!("footprint: {}", kept.display());

    docker_listing(&kept, "docker-before.txt");
    let mut watching = Watching::start(&kept)?;
    // When the command ran and when the window closed, in the seconds
    // the docker events carry, so a reader can cut every file at the
    // command's end and read the run apart from the window after it.
    let mut timeline = format!("started {}\n", crate::note::now_secs());
    // The command as the person typed it, through this runner so the
    // line reads the same as it would alone.
    let exe = std::env::current_exe().map_err(|e| format!("this runner's own path: {e}"))?;
    let outcome = Command::new(exe).args(args).status();
    timeline.push_str(&format!("ended {}\n", crate::note::now_secs()));
    // The reading the question needs is what is *still* held once the
    // work is over, and that is not the last tick of the run: a VM
    // hands pages back to Windows on its own schedule. So the samplers
    // stay up for a window after the command, and `after` in the
    // summary is the end of that window.
    //
    // waits(measured): the length of the window. Nothing is waiting on
    // it — it is part of what is being recorded — and the rows it
    // produces are the evidence for what the run left behind.
    std::thread::sleep(settle);
    watching.stop();
    timeline.push_str(&format!("settled {}\n", crate::note::now_secs()));
    let _ = std::fs::write(kept.join("timeline.txt"), timeline);
    docker_listing(&kept, "docker-after.txt");
    summarise(&kept);

    match outcome {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(format!("the command exited {:?}", status.code())),
        Err(e) => Err(format!("could not run the command: {e}")),
    }
}

/// The two samplers, held for as long as the command runs.
struct Watching {
    children: Vec<Child>,
}

/// One sampler, not yet started. What [`Watching::gather`] is handed.
type Starter = Box<dyn FnOnce() -> Result<Child, String>>;

impl Watching {
    fn start(kept: &Path) -> Result<Self, String> {
        let host = kept.join("host.csv");
        let vm = kept.join("vm.txt");
        let events = kept.join("docker-events.txt");
        Self::gather(vec![
            Box::new(move || host_side(&host)),
            Box::new(move || vm_side(&vm)),
            Box::new(move || docker_side(&events)),
        ])
    }

    /// **A child joins the watch the moment it exists.** Both loops are
    /// endless, and the only thing that ends them is this watch being
    /// dropped — so a start that fails half way has to fail with the
    /// earlier children already inside it. Building the list first and
    /// the watch afterwards leaves them running with nothing on the
    /// machine knowing whose they are, which is the shape of every
    /// sampler that outlived its measurement.
    fn gather(starts: Vec<Starter>) -> Result<Self, String> {
        let mut watching = Self {
            children: Vec::new(),
        };
        for start in starts {
            // The `?` leaves a function that owns the watch, so what it
            // has collected so far is dropped — and killed — on the way
            // out.
            watching.children.push(start()?);
        }
        Ok(watching)
    }

    /// **Killed, not waited for**: both loops are endless by
    /// construction, so the only way they end is this one.
    fn stop(&mut self) {
        for child in &mut self.children {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for Watching {
    fn drop(&mut self) {
        self.stop();
    }
}

/// The Win32 the host sampler reads its numbers from.
///
/// **WMI is not on this road.** A loop around
/// `Get-CimInstance Win32_OperatingSystem` wedged at its fifth reading
/// of a gate and wrote nothing for the remaining six minutes, while the
/// process itself stayed alive (measured 2026-09-19, seat a) — a
/// sampler that stops sampling under exactly the load it exists to
/// watch. `GlobalMemoryStatusEx` answers the same two numbers from the
/// kernel with no service in between.
const MEM_CLASS: &str = "using System;\n\
     using System.Runtime.InteropServices;\n\
     public static class FootMem {\n\
       [StructLayout(LayoutKind.Sequential)] public struct STATUS {\n\
         public uint Length; public uint Load;\n\
         public ulong TotalPhys; public ulong AvailPhys;\n\
         public ulong TotalPage; public ulong AvailPage;\n\
         public ulong TotalVirtual; public ulong AvailVirtual; public ulong AvailExtended;\n\
       }\n\
       [DllImport(\"kernel32.dll\", SetLastError=true)]\n\
       [return: MarshalAs(UnmanagedType.Bool)]\n\
       static extern bool GlobalMemoryStatusEx(ref STATUS s);\n\
       public static long[] Now() {\n\
         var s = new STATUS(); s.Length = (uint)Marshal.SizeOf(typeof(STATUS));\n\
         if (!GlobalMemoryStatusEx(ref s)) return new long[] { -1, -1 };\n\
         return new long[] { (long)(s.AvailPhys >> 20), (long)((s.TotalPage - s.AvailPage) >> 20) };\n\
       }\n\
     }";

/// The processes a gate puts on the machine, by name. The count and the
/// working set they add up to say whether what the host lost is the
/// run's own processes or the VM behind them.
const OURS: &str = "platitude-gg,cargo,rustc,xtask,git,link,docker";

/// What Windows has lost: the physical memory it can still hand out,
/// what is committed, the working set of the VM itself, and the run's
/// own processes beside them.
///
/// **Nothing in the loop may block.** Every reading is a kernel call or
/// a refresh of a handle taken once, each tick stands in its own
/// `try`, and a tick that throws is skipped rather than ending the
/// watch — the sampler outlives what it is sampling or it is worthless.
fn host_side(to: &Path) -> Result<Child, String> {
    // waits(measured): the gap between two readings. It is the sampling
    // rate, nothing waits on it, and what ends the loop is the command
    // ending (`Watching::stop`).
    let pause = format!("Start-Sleep -Seconds {}", EVERY.as_secs());
    let script = format!(
        "$out = '{}'\n\
         Add-Type -TypeDefinition @'\n\
         {MEM_CLASS}\n\
         '@\n\
         [IO.File]::WriteAllText($out, \
         \"iso,host_free_mb,host_commit_mb,vmmem_ws_mb,ours,ours_ws_mb`r`n\")\n\
         $vm = $null\n\
         while ($true) {{\n\
         try {{\n\
         $m = [FootMem]::Now()\n\
         if ($vm -eq $null -or $vm.HasExited) \
         {{ $vm = Get-Process -Name vmmemWSL -ErrorAction SilentlyContinue }}\n\
         $ws = 0\n\
         if ($vm -ne $null) {{ $vm.Refresh(); $ws = [int]($vm.WorkingSet64/1MB) }}\n\
         $ours = @(Get-Process -Name {OURS} -ErrorAction SilentlyContinue)\n\
         $sum = 0\n\
         foreach ($p in $ours) {{ $sum += $p.WorkingSet64 }}\n\
         [IO.File]::AppendAllText($out, ((Get-Date -Format 'HH:mm:ss') + ',' + $m[0] + ',' + \
         $m[1] + ',' + $ws + ',' + $ours.Count + ',' + [int]($sum/1MB) + \"`r`n\"))\n\
         }} catch {{ $vm = $null }}\n\
         {pause}\n\
         }}",
        to.display(),
    );
    Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("could not start the host sampler: {e}"))
}

/// What the VM thinks it is using, one block per tick: `/proc/meminfo`,
/// what is running, whether anything stalled on memory or was killed
/// for it, and what each container's cgroup holds.
///
/// **One line, and nothing in it to quote.** `wsl.exe` re-splits the
/// command it is handed, so a script with newlines in it arrives cut
/// off at the first one (measured: the sampler wrote nothing at all),
/// and one with nested quotes arrives in pieces. So the shell does no
/// picking: the whole file goes down raw, and [`summarise`] takes the
/// keys out on this side, where the parsing can be read.
fn vm_side(to: &Path) -> Result<Child, String> {
    let pause = format!("sleep {}", EVERY.as_secs());
    // `memory.peak` is each container's own high-water mark, which a
    // sample every two seconds would otherwise miss, and `pids.current`
    // says how many processes stood behind it.
    let cgroups = "/sys/fs/cgroup/docker/*/memory.current \
                   /sys/fs/cgroup/docker/*/memory.peak \
                   /sys/fs/cgroup/docker/*/pids.current";
    // **Who is reading, by name, every tick.** "the VM grew" does not
    // say whose bytes those were, and a sum over `docker stats` cannot
    // answer it either — the reader that mattered turned out to have no
    // container at all. `read_bytes` is per process and monotonic, so
    // two ticks give a rate and the whole file gives a start and an end.
    let readers = "grep -H . /proc/[0-9]*/comm 2>/dev/null; \
                   grep -H read_bytes /proc/[0-9]*/io 2>/dev/null";
    let script = format!(
        "while :; do date +%H:%M:%S; \
         cat /proc/meminfo /proc/loadavg /proc/pressure/memory; \
         grep -e oom_kill -e pgscan_direct /proc/vmstat; \
         grep -H . {cgroups} 2>/dev/null; \
         {readers}; \
         echo ---; {pause}; done"
    );
    let out = std::fs::File::create(to).map_err(|e| format!("{}: {e}", to.display()))?;
    Command::new("wsl")
        .args(["-d", DISTRO, "--", "sh", "-c", &script])
        .stdin(Stdio::null())
        .stdout(Stdio::from(out))
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("could not start the VM sampler: {e}"))
}

/// What docker did while this ran, one line an event: the time in
/// seconds, the kind, the action and the container's name. **The count
/// of containers made and torn down is read off this**, not off the
/// cgroup samples: a container that lived for less than a tick is in
/// here and in no sample, and `docker events --since` cannot be asked
/// afterwards for more than the daemon's short memory of them.
fn docker_side(to: &Path) -> Result<Child, String> {
    let out = std::fs::File::create(to).map_err(|e| format!("{}: {e}", to.display()))?;
    Command::new("docker")
        .args([
            "events",
            "--format",
            "{{.Time}} {{.Type}} {{.Action}} {{.Actor.Attributes.name}}",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::from(out))
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("could not start the docker events sampler: {e}"))
}

/// Every container this machine has, running or not — the two ends of
/// the question "did the run leave any".
fn docker_listing(kept: &Path, name: &str) {
    let Ok(out) = Command::new("docker")
        .args([
            "ps",
            "-a",
            "--format",
            "{{.ID}}\t{{.Names}}\t{{.Image}}\t{{.Status}}\t{{.CreatedAt}}",
        ])
        .stderr(Stdio::null())
        .output()
    else {
        return;
    };
    let _ = std::fs::write(kept.join(name), out.stdout);
}

/// What the VM's columns are worth reading. `MemAvailable` is the one
/// that says whether memory is gone; the others say which kind grew,
/// and `MemFree` is here only so that a reading that looks alarming in
/// it can be seen not to be. `Percpu` and `SUnreclaim` are the two that
/// move with cgroups made and torn down — a container's own cost to the
/// kernel, apart from any cache it filled.
const VM_KEYS: [&str; 10] = [
    "MemAvailable",
    "MemFree",
    "AnonPages",
    "Cached",
    "Buffers",
    "SReclaimable",
    "SUnreclaim",
    "Percpu",
    "Shmem",
    "SwapFree",
];

/// First, extreme, and last for every column, out of both files. The
/// rows stay where they are: this is a reading of them, not a
/// replacement for them.
fn summarise(kept: &Path) {
    let host = std::fs::read_to_string(kept.join("host.csv")).unwrap_or_default();
    let mut columns: Vec<(String, Vec<i64>)> = Vec::new();
    let mut lines = host.lines();
    if let Some(header) = lines.next() {
        for name in header.split(',').skip(1) {
            columns.push((name.trim().to_string(), Vec::new()));
        }
        for row in lines {
            for (at, cell) in row.split(',').skip(1).enumerate() {
                if let (Some(column), Ok(v)) = (columns.get_mut(at), cell.trim().parse::<i64>()) {
                    column.1.push(v);
                }
            }
        }
    }
    // The VM's file is `/proc/meminfo` as it stands, one block a tick:
    // the keys are picked out here rather than in there ([`vm_side`]).
    let vm = std::fs::read_to_string(kept.join("vm.txt")).unwrap_or_default();
    for key in VM_KEYS {
        let taken: Vec<i64> = vm
            .lines()
            .filter_map(|line| {
                let (name, rest) = line.split_once(':')?;
                (name == key)
                    .then(|| rest.split_whitespace().next()?.parse::<i64>().ok())
                    .flatten()
            })
            // kB in the file, MB in the reading, so the two sides are
            // the same unit.
            .map(|kb| kb / 1024)
            .collect();
        columns.push((format!("vm_{key}_mb"), taken));
    }
    for (name, taken) in &columns {
        let Some(first) = taken.first() else {
            println!("footprint: {name} — no rows");
            continue;
        };
        println!(
            "footprint: {name:<22} {first} → {} (min {} max {}) over {} rows",
            taken.last().unwrap_or(first),
            taken.iter().min().unwrap_or(first),
            taken.iter().max().unwrap_or(first),
            taken.len()
        );
    }
    containers(&vm);
    events(&std::fs::read_to_string(kept.join("docker-events.txt")).unwrap_or_default());
    stalls(&vm);
    readers(&vm);
}

/// How many containers docker made and tore down while this ran, and
/// the most that were up at once — off the events, which see every
/// one of them ([`docker_side`]).
fn events(events: &str) {
    let (mut created, mut destroyed, mut up, mut most) = (0usize, 0usize, 0i64, 0i64);
    for line in events.lines() {
        let mut words = line.split_whitespace();
        let (Some(_time), Some(kind), Some(action)) = (words.next(), words.next(), words.next())
        else {
            continue;
        };
        if kind != "container" {
            continue;
        }
        match action {
            "create" => created += 1,
            "destroy" => destroyed += 1,
            "start" => {
                up += 1;
                most = most.max(up);
            }
            "die" => up -= 1,
            _ => {}
        }
    }
    println!(
        "footprint: docker                {created} container(s) created, {destroyed} destroyed, \
         at most {most} up at once"
    );
}

/// Who read the disk while this ran, by name and by how much.
///
/// **The question this answers is whose bytes those were.** A VM that
/// grew says nothing about the reader, the reader need not have a
/// container, and `read_bytes` is the only per-process number that
/// survives the process itself being gone by the time anyone asks.
/// First and last reading of each pid, largest three.
///
/// **A pid is not an identity here.** A run that starts hundreds of
/// containers cycles through pids, and one reused between two ticks
/// reads as a single process that grew. The long-lived readers — the
/// ones this exists to name — do not move, and the raw rows carry the
/// `comm` beside every reading for anything that looks wrong.
fn readers(vm: &str) {
    let mut names: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    let mut read: std::collections::HashMap<&str, (i64, i64)> = std::collections::HashMap::new();
    for line in vm.lines() {
        let Some(rest) = line.strip_prefix("/proc/") else {
            continue;
        };
        let Some((pid, tail)) = rest.split_once('/') else {
            continue;
        };
        if let Some(name) = tail.strip_prefix("comm:") {
            names.insert(pid, name.trim());
        } else if let Some(bytes) = tail.strip_prefix("io:read_bytes:") {
            let Ok(now) = bytes.trim().parse::<i64>() else {
                continue;
            };
            read.entry(pid).or_insert((now, now)).1 = now;
        }
    }
    let mut grew: Vec<(i64, &str, &str)> = read
        .iter()
        .map(|(pid, (first, last))| (last - first, *pid, names.get(pid).copied().unwrap_or("?")))
        .filter(|(by, _, _)| *by > 0)
        .collect();
    grew.sort_unstable();
    grew.reverse();
    if grew.is_empty() {
        println!("footprint: the VM read           nothing, or nobody was sampled");
        return;
    }
    let said: Vec<String> = grew
        .iter()
        .take(3)
        .map(|(by, pid, name)| format!("{name}({pid}) {} MB", by / (1024 * 1024)))
        .collect();
    println!("footprint: the VM read           {}", said.join(", "));
}

/// What the containers held, out of their own cgroups: how many the run
/// put up and the largest high-water mark any one of them reached.
///
/// **This is not the VM's total and cannot be made into one.** Memory is
/// not namespaced; the page cache the containers filled is charged to
/// the VM, not to them, and a sum over these would leave it out.
fn containers(vm: &str) {
    let mut seen = std::collections::BTreeSet::new();
    let mut peak = 0i64;
    for line in vm.lines() {
        let Some((path, value)) = line.rsplit_once(':') else {
            continue;
        };
        let Some((dir, file)) = path.rsplit_once('/') else {
            continue;
        };
        if !dir.starts_with("/sys/fs/cgroup/docker/") {
            continue;
        }
        seen.insert(dir.to_string());
        if file == "memory.peak" {
            peak = peak.max(value.trim().parse::<i64>().unwrap_or(0) / (1024 * 1024));
        }
    }
    println!(
        "footprint: containers            {} seen, largest one held {peak} MB",
        seen.len()
    );
}

/// Whether the VM ever actually ran out: the pressure counter only
/// moves while something waits on memory, and `oom_kill` only moves
/// when something was killed for it. **A cache that grew is neither.**
fn stalls(vm: &str) {
    let some: Vec<&str> = vm
        .lines()
        .filter(|l| l.starts_with("some avg10="))
        .filter_map(|l| l.split("total=").nth(1))
        .collect();
    let kills: Vec<&str> = vm
        .lines()
        .filter_map(|l| l.strip_prefix("oom_kill "))
        .collect();
    println!(
        "footprint: memory stalls         {} → {} µs waited, oom kills {} → {}",
        some.first().unwrap_or(&"-"),
        some.last().unwrap_or(&"-"),
        kills.first().unwrap_or(&"-"),
        kills.last().unwrap_or(&"-"),
    );
}

/// Where a run's rows went, for a caller that means to read them.
#[cfg(test)]
pub(crate) fn kept_under(root: &Path) -> std::path::PathBuf {
    root.join("target").join("footprint")
}

#[cfg(test)]
mod tests {
    use super::{EVERY, Watching, kept_under};
    use std::path::Path;
    use std::process::{Command, Stdio};

    /// The rows are kept under the tree, not in a temp directory a
    /// reader cannot find.
    #[test]
    fn the_rows_are_kept_under_the_tree() {
        assert!(kept_under(Path::new("/w")).ends_with("target/footprint"));
    }

    /// Short against a gate's minutes, long against the cost of a line.
    #[test]
    fn the_sampler_reports_often_enough_to_see_a_step() {
        assert!(EVERY.as_secs() >= 1 && EVERY.as_secs() <= 5);
    }

    /// A child that will outlive the test if nobody kills it. Long
    /// enough that its own exit cannot be mistaken for the reaping this
    /// is looking for.
    fn a_child_that_waits() -> std::process::Child {
        let mut command = if cfg!(windows) {
            let mut c = Command::new("ping");
            c.args(["-n", "120", "127.0.0.1"]);
            c
        } else {
            let mut c = Command::new("sleep");
            c.arg("120");
            c
        };
        command
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("a child to watch")
    }

    /// Whether the machine still has that process. **Absence is what
    /// this is read for** — a pid that has been waited on is gone, and a
    /// reused one would only make this answer yes.
    fn still_running(pid: u32) -> bool {
        if cfg!(windows) {
            let out = Command::new("tasklist")
                .args(["/FI", &format!("PID eq {pid}"), "/NH"])
                .output()
                .expect("tasklist");
            String::from_utf8_lossy(&out.stdout).contains(&pid.to_string())
        } else {
            Path::new(&format!("/proc/{pid}")).exists()
        }
    }

    /// **A half-started watch leaves nothing behind.** A second sampler
    /// failing before the watch exists would leave the first one — an
    /// endless loop — sampling for as long as the machine is up,
    /// belonging to nobody.
    #[test]
    fn a_sampler_that_started_is_reaped_when_the_next_one_fails() {
        let started = a_child_that_waits();
        let pid = started.id();
        let mut handed = Some(started);

        let outcome = Watching::gather(vec![
            Box::new(move || Ok(handed.take().expect("started once"))),
            Box::new(|| Err("the second sampler could not start".to_string())),
        ]);

        assert!(outcome.is_err(), "the failure was swallowed");
        assert!(
            !still_running(pid),
            "the first sampler ({pid}) outlived the watch that never got built"
        );
    }
}
