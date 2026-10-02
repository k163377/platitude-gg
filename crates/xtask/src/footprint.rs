//! `cargo xtask footprint <command…>` — what the machine is holding
//! while something runs, on one time axis.
//!
//! Both sides are read, because neither answers for the other: the VM
//! process's working set ([`VM_PROCESS`]) is what Windows has lost, the
//! VM's `/proc/meminfo` what Linux thinks the whole VM uses — memory is
//! not namespaced, so a sum over the engine's `stats` is no substitute.
//!
//! Long-lived children sample and are killed when the command ends; a
//! process per tick would cost more than it reads. Their rows are kept
//! under `target/footprint/<run>/` and swept by nothing — they are the
//! evidence the summary is checked against.

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use crate::command::{self, Permission, Where};

pub(crate) static FOOTPRINT: command::Command = command::Command {
    id: "footprint.watch",
    call: "footprint [--settle <seconds>] <command…>",
    purpose: "run a command and record what the host, the VM and the engine hold while it does",
    run_in: Where::Seat,
    needs: &["the container engine (wslc on Windows), for the container side of the reading"],
    permission: Permission::Plain,
};

pub(crate) static COMMANDS: &[&command::Command] = &[&FOOTPRINT];

/// Short against a gate's minutes, long against the cost of a line.
const EVERY: Duration = Duration::from_secs(2);

/// The Windows process the containers' VM is: wslc's session VM, named
/// after the session and its user, hence the wildcard.
const VM_PROCESS: &str = "vmmemwslc*";

/// How long the samplers stay up after the command ends: long enough for
/// the VM to have started handing memory back (wslc tears an idle one down
/// after tens of seconds), short enough that runs back to back still read
/// apart.
const SETTLE: Duration = Duration::from_secs(60);

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

    engine_listing(&kept, "containers-before.txt");
    let mut watching = Watching::start(&kept)?;
    // In the seconds the engine's events carry, so every file can be cut at
    // the command's end.
    let mut timeline = format!("started {}\n", crate::note::now_secs());
    // Through this runner, so the line reads as it would alone.
    let exe = std::env::current_exe().map_err(|e| format!("this runner's own path: {e}"))?;
    let outcome = Command::new(exe).args(args).status();
    timeline.push_str(&format!("ended {}\n", crate::note::now_secs()));
    // The summary's last reading is the end of this window: what is still
    // held after the work ([`SETTLE`]).
    //
    // waits(measured): the length of the window. Nothing is waiting on
    // it — it is part of what is being recorded — and the rows it
    // produces are the evidence for what the run left behind.
    std::thread::sleep(settle);
    watching.stop();
    timeline.push_str(&format!("settled {}\n", crate::note::now_secs()));
    let _ = std::fs::write(kept.join("timeline.txt"), timeline);
    engine_listing(&kept, "containers-after.txt");
    summarise(&kept);

    match outcome {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(format!("the command exited {:?}", status.code())),
        Err(e) => Err(format!("could not run the command: {e}")),
    }
}

/// The samplers, held for as long as the command runs.
struct Watching {
    children: Vec<Child>,
}

/// One sampler, not yet started.
type Starter = Box<dyn FnOnce() -> Result<Child, String>>;

impl Watching {
    fn start(kept: &Path) -> Result<Self, String> {
        let host = kept.join("host.csv");
        let vm = kept.join("vm.txt");
        let events = kept.join("events.txt");
        Self::gather(vec![
            Box::new(move || host_side(&host)),
            Box::new(move || vm_side(&vm)),
            Box::new(move || engine_side(&events)),
        ])
    }

    /// A child joins the watch the moment it exists: the samplers are
    /// endless and only dropping the watch ends them, so a start that
    /// fails half way must fail with the earlier children already inside.
    /// Building the list first leaves them running, owned by nobody.
    fn gather(starts: Vec<Starter>) -> Result<Self, String> {
        let mut watching = Self {
            children: Vec::new(),
        };
        for start in starts {
            watching.children.push(start()?);
        }
        Ok(watching)
    }

    /// Killed, not waited for: the samplers are endless, so this is the
    /// only way they end.
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
/// Not WMI: a `Get-CimInstance Win32_OperatingSystem` loop wedges under a
/// gate's load while its process stays alive. `GlobalMemoryStatusEx`
/// answers the same numbers from the kernel with no service in between.
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
const OURS: &str = "platitude-gg,cargo,rustc,xtask,git,link,wslc,wslcsession,docker";

/// What Windows has lost, one CSV row a tick.
///
/// Nothing in the loop may block: every reading is a kernel call or a
/// refresh of a handle taken once, and a tick that throws is skipped
/// rather than ending the watch.
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
         {{ $vm = Get-Process -Name {VM_PROCESS} -ErrorAction SilentlyContinue \
         | Select-Object -First 1 }}\n\
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

/// What the VM thinks it is using, one block per tick.
///
/// One line, and nothing in it to quote, so the shell picks nothing;
/// [`summarise`] takes the keys out here.
fn vm_side(to: &Path) -> Result<Child, String> {
    let pause = format!("sleep {}", EVERY.as_secs());
    // `memory.peak` catches the high-water mark between ticks.
    let cgroups = "/sys/fs/cgroup/docker/*/memory.current \
                   /sys/fs/cgroup/docker/*/memory.peak \
                   /sys/fs/cgroup/docker/*/pids.current";
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
    crate::linux::engine::in_its_vm("sh", &["-c", &script])
        .stdout(Stdio::from(out))
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("could not start the VM sampler: {e}"))
}

/// What the engine did while this ran, one line an event: time, type,
/// action and the object's id lead every line ([`events`]). Containers
/// made and torn down are counted off this, not the cgroup samples: one
/// that lived less than a tick is in no sample, and `events --since`
/// remembers too little to ask afterwards.
///
/// wslc takes no template and leads its own lines that way; docker is
/// told to.
fn engine_side(to: &Path) -> Result<Child, String> {
    let out = std::fs::File::create(to).map_err(|e| format!("{}: {e}", to.display()))?;
    let mut events = crate::linux::engine::command();
    events.arg("events");
    if !cfg!(windows) {
        events.args(["--format", "{{.Time}} {{.Type}} {{.Action}} {{.Actor.ID}}"]);
    }
    events
        .stdin(Stdio::null())
        .stdout(Stdio::from(out))
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("could not start the engine's events sampler: {e}"))
}

/// Every container this machine has, running or not — the two ends of
/// the question "did the run leave any".
fn engine_listing(kept: &Path, name: &str) {
    let Ok(out) = crate::linux::engine::command()
        .args(["ps", "-a", "--format", "json"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
    else {
        return;
    };
    let _ = std::fs::write(kept.join(name), out.stdout);
}

/// The VM's columns worth reading. Judge by `MemAvailable`; `MemFree`
/// (free pages only, alarming whenever the cache grew) is here only to
/// show it is not; the rest say which kind grew. `Percpu` and `SUnreclaim`
/// move with cgroups made and torn down — a container's own kernel cost.
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

/// First, extremes and last for every column, out of both files.
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
            // kB in the file; MB, the host side's unit, in the reading.
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
    events(&std::fs::read_to_string(kept.join("events.txt")).unwrap_or_default());
    stalls(&vm);
    readers(&vm);
}

/// Counted by id: a container that ends on its own is docker's `die` and
/// wslc's `stop`, and one stopped from outside is both on docker.
fn events(events: &str) {
    let (mut created, mut destroyed, mut most) = (0usize, 0usize, 0usize);
    let mut up = std::collections::BTreeSet::new();
    for line in events.lines() {
        let mut words = line.split_whitespace();
        let (Some(_time), Some(kind), Some(action), Some(id)) =
            (words.next(), words.next(), words.next(), words.next())
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
                up.insert(id);
                most = most.max(up.len());
            }
            "die" | "stop" => {
                up.remove(id);
            }
            _ => {}
        }
    }
    println!(
        "footprint: containers made       {created} created, {destroyed} destroyed, \
         at most {most} up at once"
    );
}

/// Who read the disk while this ran. The reader need not have a
/// container, so neither the VM's growth nor the engine's `stats` names it.
///
/// A pid is not an identity here: one reused between two ticks reads as
/// a single process that grew. The long-lived readers this exists to
/// name do not move, and the raw rows carry `comm` beside every reading.
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

/// What the containers held, out of their own cgroups. Not the VM's total
/// and cannot be made one — the page cache they filled is charged to the
/// VM, not to them.
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
/// when something was killed for it. A cache that grew is neither.
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

/// Where the rows go (`run` builds the same path).
#[cfg(test)]
pub(crate) fn kept_under(root: &Path) -> std::path::PathBuf {
    root.join("target").join("footprint")
}

#[cfg(test)]
mod tests {
    use super::{EVERY, Watching, kept_under};
    use std::path::Path;
    use std::process::{Command, Stdio};

    /// Not in a temp directory a reader cannot find.
    #[test]
    fn the_rows_are_kept_under_the_tree() {
        assert!(kept_under(Path::new("/w")).ends_with("target/footprint"));
    }

    #[test]
    fn the_sampler_reports_often_enough_to_see_a_step() {
        assert!(EVERY.as_secs() >= 1 && EVERY.as_secs() <= 5);
    }

    /// Lives long enough that its own exit cannot pass for the reaping.
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

    /// Read for absence: a reused pid could only make this answer yes.
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
