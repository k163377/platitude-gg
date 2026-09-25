//! What the settled process was holding, read from outside it: which kind
//! of memory the bytes are, which files the resident pages belong to, and
//! how the process heaps are cut up.
//!
//! From outside because the Rust counting allocator (`--breakdown`) sees
//! only the Rust heap, and most of the process — the C++ objects QML
//! builds, tree-sitter's trees, the fonts DirectWrite maps — is on the
//! other side. Three Win32 questions answer it without the process's
//! cooperation:
//!
//! * `VirtualQueryEx`: private / mapped / image per region; the private
//!   regions summed per `AllocationBase` by size class, where the heap
//!   segments line up.
//! * `QueryWorkingSetEx`: which pages are resident, so a mapped file is
//!   charged for what it costs; `GetMappedFileNameW` names the file.
//! * `RtlQueryProcessDebugInformation(PDI_HEAPS | PDI_HEAP_BLOCKS)`: the
//!   heaps block by block, busy and free by size class.
//!
//! Taken **after** the settled reading was read off the sampler: the heap
//! walk runs a thread inside the process and touches every heap page, so
//! the reading is of the series before the walk (`measure`);
//! `memory.csv` keeps what the walk itself did.
//!
//! The script is armed before the app starts, like the sampler
//! (`sampler::Armed`): its C# compiles while nothing is timed, and it
//! waits on stdin for the pid.

use std::path::Path;

/// The summary the report prints, parsed off the script's first lines.
/// Bytes throughout.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Attribution {
    pub(super) elapsed_ms: u64,
    pub(super) committed_private: u64,
    pub(super) committed_mapped: u64,
    pub(super) committed_image: u64,
    pub(super) resident_private: u64,
    pub(super) resident_mapped: u64,
    pub(super) resident_image: u64,
    /// The resident share of every `.ttf` / `.ttc` / `.otf`,
    /// `StaticCache.dat` and `FontCache` file, and how many there were.
    pub(super) fonts_resident: u64,
    pub(super) fonts_files: u64,
    /// The heap walk, or why there is none: the debug buffer is the one
    /// part that can refuse (`STATUS_NO_MEMORY`), and the regions are
    /// still a reading without it.
    pub(super) heap: Option<Heap>,
    pub(super) heap_error: Option<String>,
}

/// The process heaps as `RtlQueryProcessDebugInformation` reports them,
/// summed over every heap.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Heap {
    pub(super) heaps: u64,
    pub(super) busy: u64,
    pub(super) busy_blocks: u64,
    pub(super) free: u64,
    pub(super) free_blocks: u64,
    pub(super) committed: u64,
}

/// The `key=value` lines at the top of the script's text, up to the blank
/// line the tables start after. `None` where there is no `resident` line
/// — an `error:` text, or a script that said something else.
pub(super) fn parse(text: &str) -> Option<Attribution> {
    let mut found = Attribution::default();
    let mut resident = false;
    for line in text.lines() {
        let number = |key: &str| -> Option<u64> {
            super::reading::token(line, key).and_then(|v| v.parse().ok())
        };
        if line.is_empty() {
            break;
        }
        if line.starts_with("attribution ") {
            found.elapsed_ms = number("elapsed_ms=").unwrap_or(0);
        } else if line.starts_with("committed ") {
            found.committed_private = number("private=")?;
            found.committed_mapped = number("mapped=")?;
            found.committed_image = number("image=")?;
        } else if line.starts_with("resident ") {
            found.resident_private = number("private=")?;
            found.resident_mapped = number("mapped=")?;
            found.resident_image = number("image=")?;
            resident = true;
        } else if line.starts_with("fonts ") {
            found.fonts_resident = number("resident=")?;
            found.fonts_files = number("files=")?;
        } else if let Some(error) = line.strip_prefix("heap error=") {
            found.heap_error = Some(error.to_string());
        } else if line.starts_with("heap ") {
            found.heap = Some(Heap {
                heaps: number("heaps=")?,
                busy: number("busy=")?,
                busy_blocks: number("busy_blocks=")?,
                free: number("free=")?,
                free_blocks: number("free_blocks=")?,
                committed: number("committed=")?,
            });
        }
    }
    resident.then_some(found)
}

impl Attribution {
    /// The one line a run says about itself as it is taken.
    pub(super) fn said(&self) -> String {
        let mb = super::report::mb;
        let heap = match (&self.heap, &self.heap_error) {
            (Some(heap), _) => format!(
                "heap busy {:.1} MiB in {} blocks",
                mb(heap.busy),
                heap.busy_blocks
            ),
            (None, Some(error)) => format!("heap not walked: {error}"),
            (None, None) => "heap not walked".to_string(),
        };
        format!(
            "resident private {:.1} | mapped {:.1} | image {:.1} MiB; fonts {:.1}; {heap} ({}ms)",
            mb(self.resident_private),
            mb(self.resident_mapped),
            mb(self.resident_image),
            mb(self.fonts_resident),
            self.elapsed_ms
        )
    }
}

/// Writes what the script said — or why it said nothing — to
/// `attribution.txt`, and prints the summary: the run's own line prints
/// before this is taken.
pub(super) fn record(armed: Armed, pid: u32, run_dir: &Path) -> Option<Attribution> {
    let path = run_dir.join("attribution.txt");
    let (text, summary) = match armed.take(pid) {
        Ok(text) => {
            let summary = parse(&text);
            (text, summary)
        }
        Err(said) => (format!("error: {said}\n"), None),
    };
    if let Err(error) = std::fs::write(&path, &text) {
        println!("  note: could not write {}: {error}", path.display());
    }
    match &summary {
        Some(summary) => println!("  attribution: {}", summary.said()),
        None => println!(
            "  attribution: none — {}",
            text.lines().next().unwrap_or("the script said nothing")
        ),
    }
    summary
}

/// How long the script has to answer once it has the pid. The heap walk
/// dominates — seconds for a process with a million blocks; past this it
/// is stuck, not slow.
pub(super) const CEILING: std::time::Duration = std::time::Duration::from_secs(120);

/// The script with its C# compiled, waiting on stdin for a pid.
pub(super) struct Armed {
    #[cfg(windows)]
    child: Option<std::process::Child>,
    #[cfg(windows)]
    lines: Option<super::sampler::Lines>,
}

#[cfg(windows)]
impl Drop for Armed {
    /// A run that never settled never asked; the script is not left
    /// waiting on a pipe nobody will write.
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            super::sampler::end(child, "the attribution script");
            if let Err(error) = child.wait() {
                println!("  note: could not reap the attribution script: {error}");
            }
        }
    }
}

/// Compiles the script and holds it at `ready`.
#[cfg(windows)]
pub(super) fn arm() -> Result<Armed, String> {
    use std::io::BufRead;
    use std::process::{Command, Stdio};
    let mut child = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", SCRIPT])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or("attribution script stdout missing")?;
    let lines = std::io::BufReader::new(stdout).lines();
    match super::sampler::await_line(lines, "ready") {
        Ok(lines) => Ok(Armed {
            child: Some(child),
            lines: Some(lines),
        }),
        Err(said) => {
            super::sampler::end(&mut child, "the attribution script");
            Err(format!("the attribution script did not arm: {said}"))
        }
    }
}

/// Reached by nothing: `options::settle` refuses `--attribute` off
/// Windows. Armed all the same, so `take` is where the one answer lives.
#[cfg(not(windows))]
pub(super) fn arm() -> Result<Armed, String> {
    Ok(Armed {})
}

impl Armed {
    /// Names `pid` to the script and reads everything it says, within
    /// [`CEILING`].
    #[cfg(windows)]
    fn take(mut self, pid: u32) -> Result<String, String> {
        use std::io::Write;
        let mut child = self
            .child
            .take()
            .ok_or("the attribution script was already taken")?;
        let lines = self
            .lines
            .take()
            .ok_or("the attribution script's output was already taken")?;
        {
            let mut stdin = child
                .stdin
                .take()
                .ok_or("attribution script stdin missing")?;
            writeln!(stdin, "{pid}")
                .and_then(|()| stdin.flush())
                .map_err(|e| {
                    format!("could not name the process to the attribution script: {e}")
                })?;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut text = String::new();
            for line in lines {
                match line {
                    Ok(line) => {
                        text.push_str(&line);
                        text.push('\n');
                    }
                    Err(error) => {
                        // A receiver that gave up on the ceiling is gone;
                        // nothing else is left to tell.
                        let _ = tx.send(Err(format!(
                            "the attribution script's output failed: {error}"
                        )));
                        return;
                    }
                }
            }
            let _ = tx.send(Ok(text));
        });
        let answer = match crate::wait::receive(
            "the attribution script",
            "its answer",
            &rx,
            crate::wait::Budget::whole(CEILING),
        ) {
            Ok(answer) => answer,
            Err(expired) => {
                super::sampler::end(&mut child, "the attribution script");
                Err(expired.to_string())
            }
        };
        let status = child.wait().map_err(|e| e.to_string())?;
        let text = answer?;
        if !status.success() && parse(&text).is_none() {
            return Err(format!(
                "the attribution script failed: {}",
                text.lines().next().unwrap_or("").trim()
            ));
        }
        Ok(text)
    }

    #[cfg(not(windows))]
    fn take(self, _pid: u32) -> Result<String, String> {
        Err("memory attribution is implemented for Windows only".into())
    }
}

/// The three questions above as C#, and the PowerShell that compiles it,
/// says `ready`, reads the pid and prints the text.
///
/// Heap entry flags: busy 0x0001 and segment 0x0002 as in the published
/// header; a segment's uncommitted remainder is 0x0100 in the header but
/// 0x1000 on Windows build 26100, so both are read as one. 0x4000 (a
/// block `VirtualAlloc`'d on its own) and 0x8000 (low-fragmentation heap)
/// are busy or free by bit 0.
///
/// The debug buffer is reserved at 512MB: the default overflows with
/// `STATUS_NO_MEMORY` for a process with a million blocks. The offsets —
/// `Heaps` at +112 of the buffer, 96 bytes per heap with
/// `NumberOfEntries` at +36 and `Entries` at +80, 32 bytes per entry with
/// `Size` at +0 and `Flags` at +8 — are the 64-bit layout, the only one
/// this runs on.
///
/// The `key=value` lines are bytes; the tables are MiB.
#[cfg(windows)]
const SCRIPT: &str = r#"$ErrorActionPreference='Stop';
[Console]::OutputEncoding=[Text.Encoding]::UTF8;
Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class PerfAttribution {
  [StructLayout(LayoutKind.Sequential)] public struct MBI { public IntPtr BaseAddress; public IntPtr AllocationBase; public uint AllocationProtect; public uint Partition; public IntPtr RegionSize; public uint State; public uint Protect; public uint Type; public uint Alignment; }
  [StructLayout(LayoutKind.Sequential)] public struct WSEX { public IntPtr VirtualAddress; public IntPtr Attributes; }
  [DllImport("kernel32.dll", SetLastError=true)] public static extern IntPtr OpenProcess(uint access, [MarshalAs(UnmanagedType.Bool)] bool inherit, uint pid);
  [DllImport("kernel32.dll")] [return: MarshalAs(UnmanagedType.Bool)] public static extern bool CloseHandle(IntPtr h);
  [DllImport("kernel32.dll", SetLastError=true)] public static extern IntPtr VirtualQueryEx(IntPtr h, IntPtr addr, out MBI info, IntPtr size);
  [DllImport("psapi.dll", SetLastError=true)] [return: MarshalAs(UnmanagedType.Bool)] public static extern bool QueryWorkingSetEx(IntPtr h, [In, Out] WSEX[] pages, uint size);
  [DllImport("psapi.dll", CharSet=CharSet.Unicode, SetLastError=true)] public static extern uint GetMappedFileNameW(IntPtr h, IntPtr addr, StringBuilder name, uint size);
  [DllImport("ntdll.dll")] public static extern IntPtr RtlCreateQueryDebugBuffer(uint size, [MarshalAs(UnmanagedType.U1)] bool eventPair);
  [DllImport("ntdll.dll")] public static extern int RtlQueryProcessDebugInformation(IntPtr pid, uint flags, IntPtr buffer);
  [DllImport("ntdll.dll")] public static extern int RtlDestroyQueryDebugBuffer(IntPtr buffer);
  const long PAGE = 4096;
  const int BATCH = 4096;
  class Mapped { public string Name; public string Kind; public long Committed; public long Resident; }
  class Heaps { public string Error; public int Count; public long Busy, BusyBlocks, Free, FreeBlocks, Committed, Allocated, Segments, Uncommitted; public long[] BusyCount = new long[64], BusyBytes = new long[64], FreeCount = new long[64], FreeBytes = new long[64]; public List<string> Rows = new List<string>(); }
  static string Mb(long bytes) { return (bytes / 1048576.0).ToString("F1", System.Globalization.CultureInfo.InvariantCulture); }
  static string Unit(long n) { if (n >= (1L << 30)) return (n >> 30) + "G"; if (n >= (1L << 20)) return (n >> 20) + "M"; if (n >= (1L << 10)) return (n >> 10) + "K"; return n.ToString(); }
  static string Cls(int k) { return Unit(1L << k) + "-" + Unit(1L << (k + 1)); }
  static int ClassOf(long size) { int k = 0; while (k < 62 && (size >> (k + 1)) != 0) k++; return k; }
  static bool IsFont(string file) { string f = file.ToLowerInvariant(); return f.EndsWith(".ttf") || f.EndsWith(".ttc") || f.EndsWith(".otf") || f.EndsWith("staticcache.dat") || f.Contains("fontcache"); }
  static long Resident(IntPtr h, long start, long size) {
    long resident = 0; long pages = size / PAGE; long at = start; WSEX[] batch = null; int stride = Marshal.SizeOf(typeof(WSEX));
    while (pages > 0) {
      int n = (int)Math.Min(pages, (long)BATCH);
      if (batch == null || batch.Length != n) batch = new WSEX[n];
      for (int i = 0; i < n; i++) { batch[i].VirtualAddress = new IntPtr(at + (long)i * PAGE); batch[i].Attributes = IntPtr.Zero; }
      if (QueryWorkingSetEx(h, batch, (uint)(n * stride))) { for (int i = 0; i < n; i++) { if (((long)batch[i].Attributes & 1L) != 0) resident += PAGE; } }
      at += (long)n * PAGE; pages -= n;
    }
    return resident;
  }
  static Heaps WalkHeaps(uint pid) {
    var t = new Heaps();
    IntPtr buf = RtlCreateQueryDebugBuffer(0x20000000, false);
    if (buf == IntPtr.Zero) { t.Error = "RtlCreateQueryDebugBuffer returned null"; return t; }
    try {
      int status = RtlQueryProcessDebugInformation(new IntPtr((long)pid), 0x14, buf);
      if (status != 0) { t.Error = "RtlQueryProcessDebugInformation returned 0x" + status.ToString("X8"); return t; }
      IntPtr heaps = Marshal.ReadIntPtr(buf, 112);
      if (heaps == IntPtr.Zero) { t.Error = "the debug buffer holds no heap information"; return t; }
      t.Count = Marshal.ReadInt32(heaps, 0);
      for (int i = 0; i < t.Count; i++) {
        IntPtr info = new IntPtr((long)heaps + 8 + (long)i * 96);
        long baseAddr = Marshal.ReadInt64(info, 0); uint flags = (uint)Marshal.ReadInt32(info, 8);
        long allocated = Marshal.ReadInt64(info, 16); long committed = Marshal.ReadInt64(info, 24);
        int entries = Marshal.ReadInt32(info, 36); IntPtr entry = Marshal.ReadIntPtr(info, 80);
        long busy = 0, busyBlocks = 0, free = 0, freeBlocks = 0;
        for (int j = 0; j < entries && entry != IntPtr.Zero; j++) {
          long size = Marshal.ReadInt64(entry, 0); int eflags = Marshal.ReadInt16(entry, 8) & 0xFFFF;
          if ((eflags & 0x02) != 0) t.Segments++;
          else if ((eflags & 0x1100) != 0) t.Uncommitted++;
          else if ((eflags & 0x01) != 0) { int k = ClassOf(size); t.BusyCount[k]++; t.BusyBytes[k] += size; busy += size; busyBlocks++; }
          else { int k = ClassOf(size); t.FreeCount[k]++; t.FreeBytes[k] += size; free += size; freeBlocks++; }
          entry = new IntPtr((long)entry + 32);
        }
        t.Busy += busy; t.BusyBlocks += busyBlocks; t.Free += free; t.FreeBlocks += freeBlocks; t.Committed += committed; t.Allocated += allocated;
        t.Rows.Add(string.Format("{0,16:x}  {1,8:x}  {2,12}  {3,12}  {4,11}  {5,9}  {6,11}  {7,9}", baseAddr, flags, Mb(allocated), Mb(committed), busyBlocks, Mb(busy), freeBlocks, Mb(free)));
      }
    } finally { RtlDestroyQueryDebugBuffer(buf); }
    return t;
  }
  public static string Report(uint pid) {
    int began = Environment.TickCount;
    IntPtr h = OpenProcess(0x0410, false, pid);
    if (h == IntPtr.Zero) return "error: OpenProcess(" + pid + ") failed with error " + Marshal.GetLastWin32Error() + "\n";
    long cPriv = 0, cMap = 0, cImg = 0, rPriv = 0, rMap = 0, rImg = 0;
    var allocations = new Dictionary<long, long[]>();
    var files = new Dictionary<string, Mapped>();
    var name = new StringBuilder(1024);
    try {
      long addr = 0; IntPtr mbiSize = new IntPtr(Marshal.SizeOf(typeof(MBI)));
      while (addr < 0x7FFFFFFFFFFFL) {
        MBI m;
        if (VirtualQueryEx(h, new IntPtr(addr), out m, mbiSize) == IntPtr.Zero) break;
        long size = (long)m.RegionSize; long baseAddr = (long)m.BaseAddress;
        if (size <= 0) break;
        if (m.State == 0x1000) {
          long resident = Resident(h, baseAddr, size);
          if (m.Type == 0x20000) {
            cPriv += size; rPriv += resident;
            long key = (long)m.AllocationBase; long[] t;
            if (!allocations.TryGetValue(key, out t)) { t = new long[2]; allocations[key] = t; }
            t[0] += size; t[1] += resident;
          } else {
            bool image = m.Type == 0x1000000;
            if (image) { cImg += size; rImg += resident; } else { cMap += size; rMap += resident; }
            name.Length = 0;
            string file = GetMappedFileNameW(h, new IntPtr(baseAddr), name, (uint)name.Capacity) > 0 ? name.ToString() : "(pagefile-backed section)";
            Mapped f;
            if (!files.TryGetValue(file, out f)) { f = new Mapped(); f.Name = file; f.Kind = image ? "image" : "mapped"; files[file] = f; }
            f.Committed += size; f.Resident += resident;
          }
        }
        addr = baseAddr + size;
      }
    } finally { CloseHandle(h); }
    var heaps = WalkHeaps(pid);
    long fontsResident = 0, fontsCommitted = 0; int fontFiles = 0;
    var list = new List<Mapped>(files.Values);
    foreach (var f in list) { if (IsFont(f.Name)) { fontsResident += f.Resident; fontsCommitted += f.Committed; fontFiles++; } }
    list.Sort(delegate(Mapped a, Mapped b) { return b.Resident.CompareTo(a.Resident); });
    long[] allocCount = new long[64], allocCommitted = new long[64], allocResident = new long[64];
    foreach (var kv in allocations) { int k = ClassOf(kv.Value[0]); allocCount[k]++; allocCommitted[k] += kv.Value[0]; allocResident[k] += kv.Value[1]; }
    var sb = new StringBuilder();
    sb.Append("attribution pid=").Append(pid).Append(" elapsed_ms=").Append(unchecked(Environment.TickCount - began)).Append('\n');
    sb.Append("committed private=").Append(cPriv).Append(" mapped=").Append(cMap).Append(" image=").Append(cImg).Append('\n');
    sb.Append("resident private=").Append(rPriv).Append(" mapped=").Append(rMap).Append(" image=").Append(rImg).Append(" total=").Append(rPriv + rMap + rImg).Append('\n');
    sb.Append("fonts resident=").Append(fontsResident).Append(" committed=").Append(fontsCommitted).Append(" files=").Append(fontFiles).Append('\n');
    if (heaps.Error != null) sb.Append("heap error=").Append(heaps.Error).Append('\n');
    else sb.Append("heap heaps=").Append(heaps.Count).Append(" busy=").Append(heaps.Busy).Append(" busy_blocks=").Append(heaps.BusyBlocks).Append(" free=").Append(heaps.Free).Append(" free_blocks=").Append(heaps.FreeBlocks).Append(" committed=").Append(heaps.Committed).Append(" allocated=").Append(heaps.Allocated).Append(" segments=").Append(heaps.Segments).Append(" uncommitted=").Append(heaps.Uncommitted).Append('\n');
    sb.Append("\n== private, by allocation: AllocationBase totals in a size class (committed, MiB) ==\n");
    sb.Append(string.Format("{0,-12} {1,8} {2,13} {3,12}\n", "class", "count", "committed_MB", "resident_MB"));
    for (int k = 0; k < 64; k++) { if (allocCount[k] > 0) sb.Append(string.Format("{0,-12} {1,8} {2,13} {3,12}\n", Cls(k), allocCount[k], Mb(allocCommitted[k]), Mb(allocResident[k]))); }
    sb.Append("\n== mapped and image, resident by file (MiB) ==\n");
    sb.Append(string.Format("{0,11} {1,12}  {2,-6}  {3}\n", "resident_MB", "committed_MB", "kind", "file"));
    foreach (var f in list) { sb.Append(string.Format("{0,11} {1,12}  {2,-6}  {3}\n", Mb(f.Resident), Mb(f.Committed), f.Kind, f.Name)); }
    sb.Append("\n== fonts: .ttf/.ttc/.otf, StaticCache.dat and FontCache (MiB) ==\n");
    foreach (var f in list) { if (IsFont(f.Name)) sb.Append(string.Format("{0,11} {1,12}  {2,-6}  {3}\n", Mb(f.Resident), Mb(f.Committed), f.Kind, f.Name)); }
    sb.Append("\n== process heaps (MiB) ==\n");
    if (heaps.Error != null) sb.Append("not walked: ").Append(heaps.Error).Append('\n');
    else {
      sb.Append(string.Format("{0,16}  {1,8}  {2,12}  {3,12}  {4,11}  {5,9}  {6,11}  {7,9}\n", "base", "flags", "allocated_MB", "committed_MB", "busy_blocks", "busy_MB", "free_blocks", "free_MB"));
      foreach (var row in heaps.Rows) sb.Append(row).Append('\n');
      sb.Append("\n== process heap blocks by size class, all heaps (MiB) ==\n");
      sb.Append(string.Format("{0,-12} {1,11} {2,9} {3,11} {4,9}\n", "class", "busy_blocks", "busy_MB", "free_blocks", "free_MB"));
      for (int k = 0; k < 64; k++) { if (heaps.BusyCount[k] > 0 || heaps.FreeCount[k] > 0) sb.Append(string.Format("{0,-12} {1,11} {2,9} {3,11} {4,9}\n", Cls(k), heaps.BusyCount[k], Mb(heaps.BusyBytes[k]), heaps.FreeCount[k], Mb(heaps.FreeBytes[k]))); }
    }
    return sb.ToString();
  }
}
'@;
[Console]::Out.WriteLine('ready');
[Console]::Out.Flush();
$target=[uint32][Console]::In.ReadLine();
try { $text=[PerfAttribution]::Report($target) } catch { $text='error: ' + $_.Exception.ToString() + "`n" };
[Console]::Out.Write($text);
[Console]::Out.Flush()"#;

#[cfg(test)]
mod tests {
    use super::{Attribution, Heap, parse};

    /// Real script output, cut to the summary lines and the first table.
    const SAID: &str = "attribution pid=35484 elapsed_ms=16\n\
        committed private=36528128 mapped=114262016 image=204566528\n\
        resident private=23572480 mapped=4390912 image=56406016 total=84369408\n\
        fonts resident=1720320 committed=34697216 files=3\n\
        heap heaps=10 busy=4528528 busy_blocks=16097 free=7290688 free_blocks=2285 \
        committed=5206016 allocated=5014480 segments=21 uncommitted=0\n\
        \n\
        == private, by allocation: AllocationBase totals in a size class (committed, MiB) ==\n\
        class           count  committed_MB  resident_MB\n\
        8M-16M              2          24.7         15.0\n";

    #[test]
    fn the_summary_is_read_off_the_first_lines() {
        let found = parse(SAID).expect("a text with a resident line");
        assert_eq!(
            found,
            Attribution {
                elapsed_ms: 16,
                committed_private: 36_528_128,
                committed_mapped: 114_262_016,
                committed_image: 204_566_528,
                resident_private: 23_572_480,
                resident_mapped: 4_390_912,
                resident_image: 56_406_016,
                fonts_resident: 1_720_320,
                fonts_files: 3,
                heap: Some(Heap {
                    heaps: 10,
                    busy: 4_528_528,
                    busy_blocks: 16_097,
                    free: 7_290_688,
                    free_blocks: 2_285,
                    committed: 5_206_016,
                }),
                heap_error: None,
            }
        );
        assert!(found.said().contains("16097 blocks"), "{}", found.said());
    }

    #[test]
    fn a_heap_walk_that_refused_leaves_the_regions_standing() {
        let text = SAID.replace(
            "heap heaps=10",
            "heap error=RtlQueryProcessDebugInformation returned 0xC0000017\nignored heaps=10",
        );
        let found = parse(&text).expect("the regions are still there");
        assert_eq!(found.heap, None);
        assert_eq!(
            found.heap_error.as_deref(),
            Some("RtlQueryProcessDebugInformation returned 0xC0000017")
        );
        assert_eq!(found.resident_private, 23_572_480);
        assert!(found.said().contains("0xC0000017"), "{}", found.said());
    }

    /// No attribution is what `reading::missing` refuses the run on.
    #[test]
    fn a_text_without_a_resident_line_is_no_attribution() {
        assert_eq!(parse("error: OpenProcess(1) failed with error 5\n"), None);
        assert_eq!(parse(""), None);
        assert_eq!(parse("resident private=x mapped=1 image=1 total=2\n"), None);
    }

    /// The script against a real process: the C# compiles under this
    /// PowerShell and the structure offsets hold on this Windows.
    #[cfg(windows)]
    #[test]
    fn a_process_is_attributed_from_outside() {
        let armed = super::arm().expect("the script compiles and says ready");
        let mut child = std::process::Command::new("ping")
            .args(["-n", "30", "127.0.0.1"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("a process to attribute");
        let text = armed.take(child.id()).expect("the script answered");
        child.kill().expect("the ping is ended");
        child.wait().expect("the ping is reaped");
        let found = parse(&text).unwrap_or_else(|| panic!("a resident line in:\n{text}"));
        assert!(found.resident_image > 0, "{text}");
        assert!(found.resident_private > 0, "{text}");
        assert!(found.committed_private >= found.resident_private, "{text}");
        let heap = found
            .heap
            .unwrap_or_else(|| panic!("a heap walk in:\n{text}"));
        assert!(heap.heaps > 0 && heap.busy_blocks > 0, "{text}");
        for table in [
            "== private, by allocation",
            "== mapped and image, resident by file",
            "== process heaps",
            "== process heap blocks by size class",
        ] {
            assert!(text.contains(table), "{table} missing from:\n{text}");
        }
        // Case-blind: the kernel keeps the spelling of whoever opened the
        // file first (`PING.EXE` under a loaded suite).
        assert!(text.to_ascii_lowercase().contains("ping.exe"), "{text}");
    }
}
