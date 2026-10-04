//! The processes a reading asks after, as one listing of the system's:
//! each owner by its image name and its start, and the children of an
//! owner by theirs.
//!
//! One listing answers every claim. A query per owner — WMI's
//! `Win32_Process`, `Get-Process` — walks every process of the machine
//! each time it is asked: asked once a claim, a reading costs as much as
//! the claims standing, idle ones included.
//!
//! A listing is as old as the moment it is made, and a child that started
//! after it is not in it — the holder says which listing answers what
//! (`holder::SCRIPT`). A start is the kernel's creation time, in the
//! milliseconds the hooks stamp a call's span in. One that cannot be read
//! — the process went between the listing and the question — is 0.

#[cfg(windows)]
pub(super) const SCRIPT: &str = r#"
Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
public sealed class PggProcesses {
  [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
  struct Entry {
    public uint Size, Usage, Id; public IntPtr Heap; public uint Module, Threads, Parent; public int Priority; public uint Flags;
    [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 260)] public string Image;
  }
  [DllImport("kernel32.dll", SetLastError = true)] static extern IntPtr CreateToolhelp32Snapshot(uint flags, uint id);
  [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)] static extern bool Process32FirstW(IntPtr snapshot, ref Entry entry);
  [DllImport("kernel32.dll", CharSet = CharSet.Unicode)] static extern bool Process32NextW(IntPtr snapshot, ref Entry entry);
  [DllImport("kernel32.dll")] static extern IntPtr OpenProcess(uint access, bool inherit, uint id);
  [DllImport("kernel32.dll")] static extern bool GetProcessTimes(IntPtr process, out long created, out long exited, out long kernel, out long user);
  [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr handle);
  readonly List<Entry> all = new List<Entry>();
  public PggProcesses() {
    IntPtr snapshot = CreateToolhelp32Snapshot(2, 0);
    if (snapshot == new IntPtr(-1)) throw new IOException("Cannot list the processes", Marshal.GetLastWin32Error());
    try {
      Entry entry = new Entry();
      entry.Size = (uint)Marshal.SizeOf(typeof(Entry));
      if (!Process32FirstW(snapshot, ref entry)) throw new IOException("Cannot read the process listing", Marshal.GetLastWin32Error());
      do { all.Add(entry); } while (Process32NextW(snapshot, ref entry));
    } finally { CloseHandle(snapshot); }
  }
  public string Image(uint id) {
    foreach (Entry entry in all) if (entry.Id == id) return entry.Image;
    return null;
  }
  public static long Born(uint id) {
    IntPtr process = OpenProcess(0x1000, false, id);
    if (process == IntPtr.Zero) return 0;
    try {
      long created, exited, kernel, user;
      return GetProcessTimes(process, out created, out exited, out kernel, out user) ? (created - 116444736000000000L) / 10000 : 0;
    } finally { CloseHandle(process); }
  }
  public long[] Children(uint parent) {
    List<long> born = new List<long>();
    foreach (Entry entry in all) {
      if (entry.Parent != parent || string.Equals(entry.Image, "conhost.exe", StringComparison.OrdinalIgnoreCase)) continue;
      long at = Born(entry.Id);
      if (at != 0) born.Add(at);
    }
    return born.ToArray();
  }
}
'@
function Get-Processes { return [PggProcesses]::new() }
"#;
