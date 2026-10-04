//! The holder: the one process that asks the system to stay up, and the
//! way a hook starts it.

use std::path::Path;

/// How long the holder goes without reading the claims when none changes:
/// how late it sees a process start or end, or a transcript grow.
#[cfg(windows)]
pub(super) const TICK: std::time::Duration = std::time::Duration::from_secs(15);

/// While the machine is held, how far apart the readings an event asks for
/// — a claim rewritten, the desktop app's log written — begin, once the
/// allowance of [`BURST`] is spent. A reading then can only let go, and
/// letting go can wait; every hook of every session at work rewrites a
/// claim, and a reading for each is the holder reading without a pause for
/// as long as any session works. While the machine is not held, no reading
/// waits: a hold cannot. The tick's readings are not paced, so the holder
/// beats at least every [`TICK`] and [`PACE`] together — inside
/// [`HOLDER_STALE`](super::HOLDER_STALE). Kept on a clock that only runs
/// forward: the system's is set, and set back.
#[cfg(windows)]
pub(super) const PACE: std::time::Duration = std::time::Duration::from_secs(5);
/// The readings a quiet holder answers at once before it paces them, so a
/// lone turn's stop lets go as it is written; one is earned back each
/// [`PACE`]. Counted in readings, not in hooks: a hook writes its claim
/// more than once, and the watcher tells a write's time and its size
/// apart.
#[cfg(windows)]
pub(super) const BURST: u32 = 60;

/// The size the history (`history`) is cut back from, to its newer half:
/// room for weeks of turns, so a look back over several days finds them
/// all.
#[cfg(windows)]
pub(super) const HISTORY_BYTES: u32 = 4 * 1024 * 1024;

/// `SetThreadExecutionState` flags, written in decimal: PowerShell reads
/// a hex literal with the top bit set as a negative `Int32`.
#[cfg(windows)]
const HOLD: u32 = 0x8000_0001;
#[cfg(windows)]
const LET_GO: u32 = 0x8000_0000;

/// How far a process's start may sit outside its tool call's span and
/// still be the call's. A hook stamps the span from the system's precise
/// clock, and a process started after the reading is stamped no earlier
/// (measured on Windows 11), so this is margin for both sides' rounding to
/// the millisecond, small enough to leave out the hook's own process —
/// started before the hook reads the clock.
#[cfg(windows)]
const SPAN_SLACK_MS: u64 = 10;

/// The tools whose processes are the Claude process's children and may
/// outlive their call: a shell, and a monitor on one. A child started
/// during any other call — a stdio MCP server started again, a hook — is
/// none of that call's work; one the Claude process starts for itself
/// within a shell call's span counts as the call's, since nothing tells
/// them apart.
#[cfg(windows)]
const STARTS_PROCESSES: &str = "'Bash', 'PowerShell', 'Monitor'";

/// The calls a subagent runs as (`Agent`, `Task` in older builds): the
/// subagent's own claim shows whether it works, so its caller's open call
/// is no work of its own — once the session has a subagent's claim to show
/// it; before the subagent's first hook, the call is its start.
#[cfg(windows)]
const DELEGATED: &str = "'Agent', 'Task'";

/// A reading takes the directory's lock twice, and briefly: once to copy
/// the holder's name and the shared claims as they stand, once to
/// write back what it made of them — a claim a hook changed in between is
/// left for the reading that change sets off. What takes time — asking
/// after processes, reading transcripts — happens between, so no seat's
/// hooks and no other holder wait on it. It reads whenever a claim changes
/// (the watcher's events queue while a reading runs, so none is missed)
/// and at least every tick, for what no file says — a process starting or
/// ending, a transcript growing. While it holds the machine, the readings
/// an event asks for are paced ([`PACE`], past a [`BURST`]): the one thing
/// such a reading can change is to let go. A transcript's tail is read
/// again only once the file has grown or been rewritten, so the claim of
/// a session left open and idle costs a reading no transcript. The
/// processes are listed (`processes`) twice a reading at most: as it
/// begins, for the claims' owners, and — once every transcript is read —
/// for the children of the owners with a shell call. A call's process is
/// older than the call's result, so the listing its children are read
/// from must be the younger of the two: one made before the transcripts
/// would show a call ended with no process, and forget it while its
/// process runs. It drops the claims whose Claude process
/// is gone and the tool calls that ended with no process left, names
/// itself at every reading with its verdict (`holding` / `released`) and
/// the time, in milliseconds on the system's precise clock, it copied the
/// claims that verdict comes from — a reading that could not copy them
/// keeps the verdict and the time it had — and
/// quits — letting go in `finally` — once no compatible claim is left
/// and its name is gone, once it steps down for another build's holder
/// (`hooks-r<n>`, [`RETIRE_AFTER`](super::RETIRE_AFTER)) and its name is
/// gone, or when another holder took its name; until its first
/// reading its name may still be the reservation of the hook that started
/// it. Owners are matched by image name and by the start time first seen
/// for that claim and pid, since a pid is reused — by another session's
/// Claude process too. Compatible holder revisions share activity records.
/// An unknown claim format is left to its own holder and never rewritten.
/// Files are read as UTF-8, as the hooks write them.
///
/// Its name says only what it asks for now. What it asked for, when, and
/// on whose account goes into `history`, the one file every revision's
/// holder writes, under the lock: a line each time the verdict changes or
/// the claims it holds for do — `<ms> r<revision> <pid> holding
/// <reasons>`, `… released -`, and `… quit -` as it ends, whichever way.
/// The reasons are sorted and set apart by commas: `turn:<session>` (a
/// reply or a call of the session's or of a subagent's counts),
/// `process:<session>` (a shell call's process runs and no `turn` of that
/// session counts — past its stop, or let through a prompt) and
/// `wake:<session>` (a wake-up scheduled); `-` stands for a hold kept
/// because the claims changed under the reading. They change with a
/// turn's start and end, not with each call. A line that could not be
/// written is written at the next reading. The file is cut back to its
/// newer half at [`HISTORY_BYTES`].
///
/// A file another process holds open — a scanner, a reader that lets
/// others only read — costs a reading at most, never the holder's run: a
/// file that cannot be read leaves the reading, what was asked for
/// standing, and a write or a removal refused is tried at a later one. A
/// holder ended by it would ask for nothing until a hook found its name
/// stale.
///
/// A claim's own calls are its session's claim's calls with its `who` —
/// `-`, or the subagent's id after `~` in its name. A call with a result
/// in one of its session's transcripts has ended, whether or not a hook of
/// this revision said so — a user entry's `tool_result` names its call
/// (`tool_use_id`) — and a shell call's span ends at the result's
/// `timestamp`. The running calls of a session's claims are counted
/// together, less one on a server for each of its pending requests for
/// input there. A hook's own process is a child of the Claude process too:
/// one started within a shell call's span — the hook of the call's end, or
/// of another call — counts while it runs, so a release can wait for the
/// next tick.
///
/// A subagent stopped from outside runs no SubagentStop (measured: a
/// background one stopped). It has stopped once its session's transcript
/// says so after the subagent's last hook: a background task's notice
/// naming it (`<task-id>`), or a result for the call it runs under — which
/// its `.meta.json` names (`toolUseId`) — other than a background start's
/// acknowledgement (`async_launched`). Its claim goes and its open calls
/// end then, as a SubagentStop would have done; one sent on later claims
/// again with its next hook. A call still open is never timed out.
///
/// The session's transcript tells what no hook of this repository is sure
/// to. A turn the user interrupts — at a prompt, too: turning a tool down
/// ends the turn — ends with a user entry of its own, `[Request
/// interrupted by user]` (`… for tool use]`), matched as the entry's whole
/// content as JSON: the same words quoted inside a tool's result are
/// escaped there. A user entry whose `origin` is not a person's — a
/// background task's notice, another session's message — is written
/// between turns, and the model's reply follows (all but a few in this
/// repository's transcripts); whether UserPromptSubmit runs for it is not
/// known. One written since the claim's last hook is taken for a turn's
/// start: the claim is rewritten `working`, as of the notice's
/// `timestamp`, and stays so until a hook writes otherwise — the turn's
/// Stop — however much the reply writes on the way. A notice written while
/// the claim works already moves only its time. A subagent's transcript is
/// not read for an interruption: what a subagent does after one is not
/// known, and it may reply on.
// waits(paced): the lock's retry — `FileStream.Lock` has no blocking form — and a held machine's readings, spaced by `PACE` on a clock that only runs forward
#[cfg(windows)]
pub(super) const SCRIPT: &str = r#"$ErrorActionPreference = 'Stop'
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class PggAwake {
  [DllImport("kernel32.dll")] public static extern uint SetThreadExecutionState(uint flags);
  [DllImport("kernel32.dll")] static extern void GetSystemTimePreciseAsFileTime(out long time);
  public static long Now() { long time; GetSystemTimePreciseAsFileTime(out time); return (time - 116444736000000000L) / 10000; }
  public static long Ticks() { return System.Diagnostics.Stopwatch.GetTimestamp() / (System.Diagnostics.Stopwatch.Frequency / 1000); }
}
'@
$dir = '@DIR@'
$me = "$PID"
$revision = '@REVISION@'
$format = '@FORMAT@'
$name = "$dir\holder-r$revision"
$reserved = '@RESERVED@ @STARTING@'
$born = @{}
$holding = $false
$previous = 0
$applied = 0
$readat = 0
$told = $null
$toldWhy = ''
$paced = [int64]0
$script:tails = @{}
@PROCESSES@
@DESKTOP@
$desktopPath = [IO.Path]::Combine("$env:LOCALAPPDATA", 'Claude', 'logs', 'main.log')
if (Test-Path -LiteralPath "$dir\desktop-log.path") { $desktopPath = [IO.File]::ReadAllText("$dir\desktop-log.path").Trim() }
$desktop = New-DesktopLog $desktopPath
$desktopWatch = $null
try {
  if (Test-Path -LiteralPath "$dir\desktop-log.off") { throw 'disabled' }
  $desktopWatch = New-Object IO.FileSystemWatcher -ArgumentList ([IO.Path]::GetDirectoryName($desktopPath)), 'main*.log'
  foreach ($kind in 'Created', 'Changed', 'Deleted', 'Renamed') {
    [void](Register-ObjectEvent -InputObject $desktopWatch -EventName $kind -SourceIdentifier "desktop-$kind")
  }
  $desktopWatch.EnableRaisingEvents = $true
} catch { $desktop.Status='watcher unavailable; polling' }
$delegated = @(@DELEGATED@)
$shells = @(@STARTS_PROCESSES@)
$interrupted = '"content":\[\{"type":"text","text":"\[Request interrupted by user( for tool use)?\]"\}\]'
$notice = '"type":"user".*"origin":\{"kind":"(?!human")'
$watch = New-Object IO.FileSystemWatcher -ArgumentList $dir, '*.claim'
$watch.NotifyFilter = [IO.NotifyFilters]'FileName, LastWrite, Size'
foreach ($kind in 'Created', 'Changed', 'Deleted', 'Renamed') {
  [void](Register-ObjectEvent -InputObject $watch -EventName $kind -SourceIdentifier "claim-$kind")
}
$watch.EnableRaisingEvents = $true
function Enter-Lock {
  $file = [IO.File]::Open("$dir\lock", 'OpenOrCreate', 'ReadWrite', 'ReadWrite, Delete')
  while ($true) {
    try { $file.Lock(0, 1); return $file } catch [IO.IOException] { Start-Sleep -Milliseconds 20 }
  }
}
function Exit-Lock($file) {
  $file.Unlock(0, 1)
  $file.Dispose()
}
function Get-Stamp($line) {
  if ($line -match '"timestamp":"([^"]+)"') { return [DateTimeOffset]::Parse($Matches[1]).ToUnixTimeMilliseconds() }
  return [int64]0
}
function Read-Tail($path, $earlier) {
  $tail = @{ Mark = ''; Lines = @(); Ended = @{}; Notice = [int64]0; Last = '' }
  if ((-not $path) -or ($path -eq '-') -or -not (Test-Path -LiteralPath $path)) { return $tail }
  $stream = [IO.File]::Open($path, 'Open', 'Read', 'ReadWrite, Delete')
  try {
    $tail.Mark = "$($stream.Length) $([IO.File]::GetLastWriteTimeUtc($path).Ticks)"
    if ($earlier -and ($earlier.Mark -eq $tail.Mark)) { return $earlier }
    $start = [Math]::Max(0, $stream.Length - 262144)
    [void]$stream.Seek($start, 'Begin')
    $take = [int]($stream.Length - $start)
    $bytes = New-Object byte[] $take
    $tail.Lines = [Text.Encoding]::UTF8.GetString($bytes, 0, $stream.Read($bytes, 0, $take)) -split "`n"
  } finally { $stream.Dispose() }
  foreach ($line in $tail.Lines) {
    if ($line -match '"type":"(user|assistant)"') { $tail.Last = $line }
    if ($line -notmatch '"type":"user"') { continue }
    if ($line -match $notice) { $tail.Notice = Get-Stamp $line }
    $found = [regex]::Matches($line, '"tool_use_id":"([^"]+)"')
    if ($found.Count -eq 0) { continue }
    $at = Get-Stamp $line
    foreach ($one in $found) { $tail.Ended[$one.Groups[1].Value] = $at }
  }
  return $tail
}
function Get-Tail($path) {
  if (-not $script:tails.ContainsKey("$path")) { $script:tails["$path"] = Read-Tail $path $script:earlier["$path"] }
  return $script:tails["$path"]
}
function Get-Whose($base) {
  return $base.Substring($base.IndexOf('.') + 1)
}
function Add-History($verdict, $reasons) {
  $path = "$dir\history"
  try {
    [IO.File]::AppendAllText($path, "$([PggAwake]::Now()) r$revision $me $verdict $reasons`n")
    if ((New-Object IO.FileInfo $path).Length -gt @HISTORY@) {
      $all = [IO.File]::ReadAllText($path)
      [IO.File]::WriteAllText($path, $all.Substring($all.IndexOf("`n", [int]($all.Length / 2)) + 1))
    }
    return $true
  } catch [IO.IOException], [UnauthorizedAccessException] { return $false }
}
function Read-Word($path) {
  return [IO.File]::ReadAllText($path).Trim()
}
function Write-Word($path, $text) {
  try { [IO.File]::WriteAllText($path, $text) } catch [IO.IOException], [UnauthorizedAccessException] { }
}
function Remove-Word($path) {
  try { [IO.File]::Delete($path); return $true } catch [IO.IOException], [UnauthorizedAccessException] { return $false }
}
function Test-Named {
  if (-not (Test-Path -LiteralPath $name)) { return $false }
  $said = Read-Word $name
  return ((($said -split ' ')[0] -eq $me) -or ($said -eq $reserved))
}
function Get-Heard($build) {
  $path = "$dir\hooks-r$build"
  if (-not (Test-Path -LiteralPath $path)) { return [int64]0 }
  $said = [int64]0
  if ([int64]::TryParse((Read-Word $path), [ref]$said)) { return $said }
  return [int64]::MaxValue
}
function Test-Retiring($nowms) {
  $quiet = [int64]@RETIRE@ * 1000
  if (($nowms - (Get-Heard $revision)) -lt $quiet) { return $false }
  foreach ($other in Get-ChildItem -LiteralPath $dir -Filter 'holder-r*') {
    if ($other.Name -notmatch '^holder-r(\d+)$') { continue }
    $theirs = [int]$Matches[1]
    if (($theirs -eq [int]$revision) -or (([DateTime]::UtcNow - $other.LastWriteTimeUtc).TotalSeconds -ge @STALE@)) { continue }
    if ((($nowms - (Get-Heard $theirs)) -lt $quiet) -or ($theirs -gt [int]$revision)) { return $true }
  }
  return $false
}
function Get-Stop($transcript, $agent) {
  if ((-not $transcript) -or ($transcript -eq '-')) { return [int64]0 }
  $parent = [IO.Path]::GetDirectoryName([IO.Path]::GetDirectoryName($transcript)) + '.jsonl'
  $meta = [IO.Path]::ChangeExtension($transcript, '.meta.json')
  $launch = ''
  if ((Test-Path -LiteralPath $meta) -and ((Read-Word $meta) -match '"toolUseId":"([^"]+)"')) { $launch = $Matches[1] }
  $stop = [int64]0
  foreach ($line in (Get-Tail $parent).Lines) {
    if ($line -notmatch '"type":"user"') { continue }
    $named = ($line -match $notice) -and $line.Contains("<task-id>$agent</task-id>")
    $answered = $launch -and $line.Contains("`"tool_use_id`":`"$launch`"") -and ($line -notmatch '"status":"async_launched"')
    if ($named -or $answered) { $stop = [Math]::Max($stop, (Get-Stamp $line)) }
  }
  return $stop
}
function Get-Children($owner) {
  if ($null -eq $script:spawned) { $script:spawned = Get-Processes }
  if (-not $script:kids.ContainsKey($owner)) { $script:kids[$owner] = @($script:spawned.Children($owner)) }
  return $script:kids[$owner]
}
try {
  while ($true) {
    $nowms = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
    $now = [int64][Math]::Floor($nowms / 1000)
    $script:kids = @{}
    $script:spawned = $null
    $script:earlier = $script:tails
    $script:tails = @{}
    $keep = $holding
    $why = @{}
    $seen = $null
    $minded = @{}
    $lock = Enter-Lock
    try {
      try {
        if (-not (Test-Named)) { break }
        $readms = [PggAwake]::Now()
        $seen = @()
        foreach ($claim in Get-ChildItem -LiteralPath $dir -Filter "r$format.*.claim") { $seen += ,@($claim, (Read-Word $claim.FullName)) }
        foreach ($claim in Get-ChildItem -LiteralPath $dir -Filter "r$format.*.claim") {
          if ($claim.Name -match '^r\d+\.([^~]+)~') { $minded[$Matches[1]] = $true }
        }
      } catch [IO.IOException], [UnauthorizedAccessException] { $seen = $null }
    } finally { Exit-Lock $lock }
    $changes = @()
    if ($null -ne $seen) {
      try {
        $keep = $false
        $script:processes = Get-Processes
        $entries = @()
        $known = @{}
        foreach ($pair in $seen) {
          $claim, $text = $pair
          $lead, $body = $text -split ' ', 2
          if ($lead -ne "r$format") {
            $changes += ,@($claim.FullName, $text, $null)
            continue
          }
          $word = "$body" -split ' ', 8
          if ($word.Count -ne 8) {
            $changes += ,@($claim.FullName, $text, $null)
            continue
          }
          $owner = [uint32]$word[1]
          $mark = "$($claim.Name) $owner"
          $birth = [PggProcesses]::Born($owner)
          if (-not $birth) { $birth = $born[$mark] }
          if (($script:processes.Image($owner) -ne 'claude.exe') -or ($born[$mark] -and ($born[$mark] -ne $birth))) {
            $changes += ,@($claim.FullName, $text, $null)
            continue
          }
          $known[$mark] = $birth
          $session, $who = $claim.BaseName -split '~', 2
          if (-not $who) { $who = '-' }
          $entries += ,@($claim, $text, $word, $session, $who, (Get-Tail $word[7]))
        }
        $born = $known
        $ended = @{}
        $stopped = @{}
        foreach ($entry in $entries) {
          $claim, $text, $word, $session, $who, $tail = $entry
          if (-not $ended.ContainsKey($session)) { $ended[$session] = @{}; $stopped[$session] = @{} }
          foreach ($id in $tail.Ended.Keys) { $ended[$session][$id] = $tail.Ended[$id] }
        }
        $counted = @()
        foreach ($entry in $entries) {
          $claim, $text, $word, $session, $who, $tail = $entry
          if ($who -ne '-') {
            $stop = Get-Stop $word[7] $who
            if ($stop -gt [int64]$word[2]) {
              $stopped[$session][$who] = $stop
              $changes += ,@($claim.FullName, $text, $null)
              continue
            }
          }
          $counted += ,$entry
        }
        $calls = @{}
        $requests = @{}
        foreach ($entry in $counted) {
          $claim, $text, $word, $session, $who, $tail = $entry
          if ($who -ne '-') { continue }
          $owner = [uint32]$word[1]
          $changed = $false
          if ($word[5] -ne '-') {
            $kept = @()
            foreach ($call in $word[5] -split ',') {
              $part = $call -split ':'
              if (([int64]$part[5] -eq 0) -and $stopped[$session].ContainsKey($part[0])) {
                $part[5] = "$($stopped[$session][$part[0]])"
                $call = $part -join ':'
                $changed = $true
              }
              $answered = $ended[$session].ContainsKey($part[1])
              $closed = [int64]$part[5]
              if (($closed -eq 0) -and $answered) { $closed = $ended[$session][$part[1]] }
              $running = $false
              if ($shells -contains $part[2]) {
                $from = [int64]$part[4] - @SLACK@
                $until = [int64]::MaxValue
                if ($closed -ne 0) { $until = $closed + @SLACK@ }
                foreach ($started in Get-Children $owner) { if (($started -ge $from) -and ($started -le $until)) { $running = $true } }
              }
              if ($running) { $keep = $true; $why["process:$(Get-Whose $session)"] = $true }
              if ($running -or (([int64]$part[5] -eq 0) -and -not $answered)) { $kept += $call } else { $changed = $true }
            }
            $word[5] = '-'
            if ($kept.Count -gt 0) { $word[5] = $kept -join ',' }
          }
          if ($word[5] -ne '-') { $calls[$session] = @($word[5] -split ',') }
          if ($word[6] -ne '-') {
            $requests[$session] = @{}
            foreach ($request in $word[6] -split ',') {
              $server = ($request -split ':')[0]
              $requests[$session][$server] = 1 + [int]$requests[$session][$server]
            }
          }
          if ($tail.Notice -gt [int64]$word[2]) {
            $word[0] = 'working'
            $word[2] = "$($tail.Notice)"
            $changed = $true
          }
          if ($changed) { $changes += ,@($claim.FullName, $text, ("r$format " + (($word[0..7]) -join ' '))) }
        }
        $candidates=@()
        foreach ($session in $calls.Keys) {
          foreach ($call in $calls[$session]) {
            $part=$call -split ':'
            if (([int64]$part[5] -eq 0) -and -not $ended[$session].ContainsKey($part[1])) {
              $candidates += @{ Key="$session/$($part[0])/$($part[1])/$($part[4])"; Session=(Get-Whose $session); Part=$part }
            }
          }
        }
        Update-DesktopLog $desktop $candidates $nowms
        $runs = @{}
        foreach ($entry in $counted) {
          $claim, $text, $word, $session, $who, $tail = $entry
          if ([int64]$word[4] -gt $now) { $keep = $true; $why["wake:$(Get-Whose $session)"] = $true }
          if ($word[0] -ne 'working') { continue }
          if (($who -eq '-') -and ($tail.Last -match $interrupted)) { continue }
          $open = @()
          foreach ($call in @($calls[$session])) {
            if (-not $call) { continue }
            $part = $call -split ':'
            if (($part[0] -ne $who) -or ([int64]$part[5] -ne 0) -or $ended[$session].ContainsKey($part[1])) { continue }
            $open += ,$part
          }
          $bare = Get-Whose $session
          foreach ($part in $open) {
            $starting = ($delegated -contains $part[2]) -and -not $minded[$bare]
            if ((-not (Get-DesktopAsked $desktop $session $part)) -and ($starting -or ($delegated -notcontains $part[2]))) { $runs[$session] = @($runs[$session]) + $part[2] }
          }
          $fresh = ($nowms - [int64]$word[2]) -lt @TTL@ * 1000
          if (($open.Count -eq 0) -and $fresh) { $keep = $true; $why["turn:$bare"] = $true }
        }
        foreach ($session in @($runs.Keys)) {
          $running = 0
          $held = @{}
          $asking = $requests[$session]
          foreach ($called in $runs[$session]) {
            if (-not $called) { continue }
            $server = $null
            if ($asking) { foreach ($asker in $asking.Keys) { if ($called.StartsWith("mcp__$($asker)__")) { $server = $asker } } }
            if ($server) { $held[$server] = 1 + [int]$held[$server] } else { $running++ }
          }
          foreach ($server in $held.Keys) { $running += [Math]::Max(0, $held[$server] - $asking[$server]) }
          if ($running -gt 0) { $keep = $true; $why["turn:$(Get-Whose $session)"] = $true }
        }
        foreach ($reason in @($why.Keys)) {
          if ($reason.StartsWith('process:') -and $why.ContainsKey("turn:$($reason.Substring(8))")) { $why.Remove($reason) }
        }
        $readat = $readms
      } catch [IO.IOException], [UnauthorizedAccessException] {
        $seen = $null
        $keep = $holding
      }
    }
    $lock = Enter-Lock
    try {
      try {
        if (-not (Test-Named)) { break }
        if ($null -ne $seen) {
          $current = @(Get-ChildItem -LiteralPath $dir -Filter "r$format.*.claim")
          $unchanged = $current.Count -eq $seen.Count
          foreach ($pair in $seen) {
            $file, $was = $pair
            if ((-not (Test-Path -LiteralPath $file.FullName)) -or ((Read-Word $file.FullName) -ne $was)) { $unchanged = $false; break }
          }
          if (-not $unchanged) { $keep = $true; $changes = @(); $readat = 0; $why = @{} }
          foreach ($change in $changes) {
            $file, $was, $becomes = $change
            if ((Test-Path -LiteralPath $file) -and ((Read-Word $file) -eq $was)) {
              if ($null -eq $becomes) { [void](Remove-Word $file) } else { Write-Word $file $becomes }
            }
          }
          if (($seen.Count -eq 0) -and ($current.Count -eq 0) -and (Remove-Word $name)) { break }
        }
        if ((Test-Retiring $nowms) -and (Remove-Word $name)) { break }
        if ($keep -ne $holding) {
          if ($keep) { $previous = [PggAwake]::SetThreadExecutionState([uint32]@HOLD@) }
          else { $previous = [PggAwake]::SetThreadExecutionState([uint32]@LET_GO@) }
          if ($previous -ne 0) { $holding = $keep; $applied = [PggAwake]::Now() } else { $keep = $holding }
        }
        $verdict = 'released'
        if ($keep) { $verdict = 'holding' }
        $reasons = '-'
        if ($keep -and $why.Count) { $reasons = (@($why.Keys) | Sort-Object) -join ',' }
        if ((($keep -ne $told) -or ($keep -and $why.Count -and ($reasons -ne $toldWhy))) -and (Add-History $verdict $reasons)) {
          $told = $keep
          $toldWhy = $reasons
        }
        Write-Word $name "$me $verdict $readat $previous $applied"
        $transport='polling'
        if ($desktopWatch -and $desktopWatch.EnableRaisingEvents) { $transport='file-events' }
        Write-Word "$dir\desktop-r$revision" "$me $nowms $($desktop.Status) transport=$transport"
      } catch [IO.IOException], [UnauthorizedAccessException] { $keep = $holding }
    } finally { Exit-Lock $lock }
    $woken = Wait-Event -Timeout @TICK@
    if ($holding -and $woken) {
      $ahead = $paced - [PggAwake]::Ticks() - @BURST@ * @PACE@
      if ($ahead -gt 0) { Start-Sleep -Milliseconds $ahead }
      $paced = [Math]::Max($paced, [PggAwake]::Ticks()) + @PACE@
    }
    Get-Event | Remove-Event
  }
} finally {
  [void][PggAwake]::SetThreadExecutionState([uint32]@LET_GO@)
  try {
    $parting = Enter-Lock
    try { [void](Add-History 'quit' '-') } finally { Exit-Lock $parting }
  } catch [IO.IOException], [UnauthorizedAccessException] { }
  Close-DesktopLog $desktop
  if ($null -ne $desktopWatch) { $desktopWatch.Dispose() }
}
"#;

/// The script for a holder in `dir`, started under the reservation of the
/// hook whose pid is `reserved`.
#[cfg(windows)]
pub(super) fn script(dir: &Path, reserved: u32) -> String {
    SCRIPT
        .replace("@PROCESSES@", super::processes::SCRIPT)
        .replace("@DESKTOP@", super::desktop::SCRIPT)
        .replace("@DIR@", &dir.display().to_string().replace('\'', "''"))
        .replace("@RESERVED@", &reserved.to_string())
        .replace("@STARTING@", super::STARTING)
        .replace("@HOLD@", &HOLD.to_string())
        .replace("@LET_GO@", &LET_GO.to_string())
        .replace("@TTL@", &super::WORKING_TTL.as_secs().to_string())
        .replace("@RETIRE@", &super::RETIRE_AFTER.as_secs().to_string())
        .replace("@STALE@", &super::HOLDER_STALE.as_secs().to_string())
        .replace("@TICK@", &TICK.as_secs().to_string())
        .replace("@PACE@", &PACE.as_millis().to_string())
        .replace("@BURST@", &BURST.to_string())
        .replace("@HISTORY@", &HISTORY_BYTES.to_string())
        .replace("@SLACK@", &SPAN_SLACK_MS.to_string())
        .replace("@DELEGATED@", DELEGATED)
        .replace("@STARTS_PROCESSES@", STARTS_PROCESSES)
        .replace("@REVISION@", &super::REVISION.to_string())
        .replace("@FORMAT@", &super::CLAIM_FORMAT.to_string())
}

/// Starts the holder, hidden, under the reservation of the hook whose pid
/// is `reserved`, and answers its pid.
///
/// WMI's provider host creates it, so it inherits nothing of the hook's.
/// Claude Code reads a hook's output until every copy of the pipe closes,
/// and a process started from the hook takes every inheritable handle —
/// the copies `cargo run` makes of its own among them — so a holder the
/// hook started would keep the hook running for as long as it held. The
/// PowerShell that asks WMI takes those copies too, and ends within the
/// hook.
///
/// The script is handed over in a file of its own under `dir`, named by
/// its content — the command lines have no room for it — which the holder
/// reads and removes as it starts. A start that fails removes it here.
#[cfg(windows)]
pub(super) fn spawn(dir: &Path, reserved: u32) -> Option<u32> {
    use std::process::Stdio;
    let path = script_path(dir, reserved);
    std::fs::write(&path, script(dir, reserved)).ok()?;
    let pid = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &create_command(dir, reserved),
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()
        .and_then(|out| String::from_utf8_lossy(&out.stdout).trim().parse().ok());
    if pid.is_none() {
        let _ = std::fs::remove_file(&path);
    }
    pid
}

#[cfg(not(windows))]
pub(super) fn spawn(_dir: &Path, _reserved: u32) -> Option<u32> {
    None
}

/// What the PowerShell that asks WMI runs: a command line of its own, as
/// the holder's is inside it, so both are held to Windows' 32,767
/// characters. The holder's command reads its script from [`spawn`]'s
/// file, drops the file, and runs the script as a script block — no
/// script file runs, so no execution policy has a say, as with any
/// `-EncodedCommand`.
#[cfg(windows)]
pub(super) fn create_command(dir: &Path, reserved: u32) -> String {
    let bootstrap = format!(
        "$installed = '{}'; $body = [IO.File]::ReadAllText($installed); \
         try {{ [IO.File]::Delete($installed) }} catch {{ }}; \
         & ([ScriptBlock]::Create($body))",
        script_path(dir, reserved)
            .display()
            .to_string()
            .replace('\'', "''")
    );
    let holder = format!(
        "powershell -NoProfile -NonInteractive -WindowStyle Hidden -EncodedCommand {}",
        encoded(&bootstrap)
    );
    format!(
        "$hidden = New-CimInstance -ClassName Win32_ProcessStartup -ClientOnly \
           -Property @{{ ShowWindow = [uint16]0 }}; \
         $made = Invoke-CimMethod -ClassName Win32_Process -MethodName Create \
           -Arguments @{{ CommandLine = '{holder}'; ProcessStartupInformation = $hidden }}; \
         if ($made.ReturnValue -eq 0) {{ $made.ProcessId }}"
    )
}
#[cfg(windows)]
fn script_path(dir: &Path, reserved: u32) -> std::path::PathBuf {
    let hash = super::digest(&script(dir, reserved));
    dir.join(format!("holder-{hash}.ps1"))
}
/// `script` as `-EncodedCommand` takes it: UTF-16LE, in base64 — a
/// command line WMI passes on with no quoting left to get wrong.
#[cfg(windows)]
pub(super) fn encoded(script: &str) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let word = chunk.iter().enumerate().fold(0u32, |word, (at, byte)| {
            word | u32::from(*byte) << (16 - 8 * at)
        });
        for at in 0..4 {
            if at <= chunk.len() {
                out.push(char::from(ALPHABET[(word >> (18 - 6 * at) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}
