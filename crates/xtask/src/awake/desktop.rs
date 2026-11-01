//! Optional interpretation of Desktop's private permission log. It never
//! changes hook claims: losing evidence drops the overlay, not hook facts.
//! Claude's own permission decisions are never touched either, and the
//! overlay lives in each holder alone.
//!
//! It covers the two spans no hook tells of: a call that starts no process
//! (MCP, WebFetch, WebSearch) from a person's grant to its end, and a
//! dialog the desktop app raises itself (`browser:open_file` and the like),
//! during which the call waits on a person. It is no stand-in for a proper
//! notice: read after the fact, it leaves the span from a grant to the
//! reading that sees it, and input at the machine is no proof that span is
//! empty. A grant given remotely, a clock change, another call within the
//! same second, a wait the log does not show, an unknown tool and an app
//! update can all go unseen. A holder without this overlay keeps holding
//! while a call waits on a person, and holders of other revisions judge by
//! scripts of their own — any one holding keeps the machine up — so this
//! holder letting go is not the machine sleeping.
//!
//! The log is `%LOCALAPPDATA%/Claude/logs/main.log`, or the absolute path
//! in `.awake/desktop-log.path` (read as the holder starts; tests point at
//! their own log so), and `main1.log` beside it, the generation before: the
//! app rotates at about 10 MB, `main.log` to `main1.log` to `main2.log`.
//! Four kinds of line count — a session mapped, a request emitted, a
//! response received, a request aborted; the fixtures in
//! `tests/holder/desktop.rs` keep their observed grammar. The CLI session
//! is the hook's `session_id`, and a CLI permission spells the tool as the
//! hook's `tool_name` does. The app's own dialogs name a kind, not a tool,
//! and the log does not say which call raised one, so a kind binds to any
//! call of the servers that raise it: `browser:*` (`open_file`,
//! `open_site`, `domain_transition`, `submit_credentials` and more — a
//! local file or a site opened by `navigate`, `preview_start` or
//! `browser_batch`, a change of domain, credentials sent) to
//! `mcp__Claude_Browser__*` and `mcp__claude-in-chrome__*`, and
//! `computer:*` (`request_access`) to `mcp__computer-use__*`. The kinds are
//! those of app 2.19675.0's `app.asar` (`permissionSurface` and the callers
//! of `handleToolPermission`).
//!
//! A request binds to a call when its session has exactly one open call of
//! a matching tool started by the end of the request's second (the log
//! keeps local time, to the second). With more than one — a parent and a
//! subagent — it binds to none (`ambiguous`) and never moves to another
//! candidate later. A reading binds from the claims it copied at its start,
//! where a call that started and asked since is missing and the call before
//! it on the same server may still stand open; when they changed meanwhile,
//! or could not be compared (a name or a claim unreadable), what it bound
//! is undone and bound again by the next reading, which the change sets
//! off. While a request is pending only its bound call waits on a person;
//! other sessions' and background processes' holds stand. Only `once` and `always` grant, and a
//! call resumes once every request on it is resolved. An unknown answer, or a tool or a time that
//! does not fit, is no grant: the call goes by the hooks (`unknown`). `aborted` — a dialog taken
//! down unanswered, as when an OS shutdown ends the CLI process under it — is no grant either, but
//! nobody waits on it any more, so it resolves. An answer for a call that has ended resumes no
//! other call. Permission to run `AskUserQuestion` is not the person's answer to it, and an MCP
//! elicitation stays the hooks' own wait.
//!
//! A file is followed by its Windows file ID, not its name or size: once
//! `main.log` becomes `main1.log` the reading goes on where it was, and the
//! new `main.log` is read from its start, however large. A line counts once
//! its newline is written; a partial one waits for the next reading. When
//! every file followed is replaced, or one shrinks, the overlay is dropped
//! and rebuilt from the logs left (`log-gap-replayed`). An unreadable log,
//! a line of the four kinds in a grammar not known, or a limit passed
//! (32 MiB a file, 1 MiB a line, 10,000 entries a history) drops the
//! overlay for the hooks alone (`fallback`) until a reading finds the log
//! sound again. The holder reads on its watcher's events for `main*.log`,
//! and at every `holder::TICK` for what the watcher misses. A call the hooks say
//! is asked about, with no request bound to it, counts as `unmatched` from
//! 5 s after it is first seen — status only, the verdict stays the hooks'.
//!
//! What a holder made of its last reading is `desktop-r<n>`, as
//! `<pid> <ms> <status> transport=<file-events|polling>`, which
//! `cargo xtask awake` shows per holder; a stale holder's last reading says
//! nothing of now. `disabled`: `desktop-log.off` stands, the hooks alone
//! judge. `watching overlay=N`: a sound reading, N the calls a request is
//! bound to (no claim that every wait was matched), then the counts of
//! `unmatched`, `ambiguous`, `unknown` and `unmapped` (calls started by the
//! request's time, none of a matching tool), and `log-gap-replayed` after a
//! rebuild. `fallback: <why>`: a read, grammar or limit problem; the hooks
//! alone judge.
//!
//! When the app changes its log's grammar, fix this parser and the fixtures
//! together; when it moves the log, write the new `main.log`'s absolute
//! path to `.awake/desktop-log.path`. Quote the request, response and
//! mapping lines only, never input text or a whole log.
//! `cargo xtask awake log off` sends every holder to the hooks alone from
//! its next reading (a `holder::TICK` at most), changing no power setting
//! and no claim; a holder started while it was off watches no file, so
//! after `log on` it reads only on the tick — check the event-driven
//! response with a holder started afterwards. A change here is checked on a real dialog too —
//! waiting, granted, ended — and where other holds stand, this holder's
//! release and the machine's sleep are judged apart.

#[cfg(windows)]
pub(super) const SCRIPT: &str = r#"
Add-Type -TypeDefinition @'
using System;
using System.IO;
using System.Runtime.InteropServices;
public static class PggDesktopLog {
  [DllImport("kernel32.dll", SetLastError=true)]
  static extern bool GetFileInformationByHandleEx(IntPtr handle, int kind, byte[] data, uint size);
  public static string Key(FileStream file) {
    byte[] data = new byte[24];
    if (!GetFileInformationByHandleEx(file.SafeFileHandle.DangerousGetHandle(), 18, data, 24))
      throw new IOException("Cannot identify the Desktop log", Marshal.GetLastWin32Error());
    return Convert.ToBase64String(data);
  }
}
'@
function New-DesktopLog($path) {
  return @{ Path=$path; Files=@{}; Sessions=@{}; Requests=@{}; Answers=@{}; Asked=@{};
    Overlay=@{}; Decided=@(); Status='starting'; Warning=''; Enabled=$false; Seen=0 }
}
function Close-DesktopLog($state) {
  foreach ($file in $state.Files.Values) { $file.Reader.Dispose() }
  $state.Files = @{}
}
function Reset-DesktopLog($state) {
  Close-DesktopLog $state
  $state.Sessions=@{}; $state.Requests=@{}; $state.Answers=@{}; $state.Asked=@{}
  $state.Overlay=@{}; $state.Decided=@(); $state.Enabled=$false; $state.Warning=''
}
# A reading binds from the claims it copied at its start; a call that
# started and asked after the copy is missing from them, so what a reading
# bound is bound again unless it found its claims unchanged at its end.
# Ambiguity stands: a second choice made as candidates end could pick the
# call that runs.
function Undo-DesktopDecisions($state) {
  foreach ($id in $state.Decided) {
    if ($state.Requests.ContainsKey($id)) { $state.Requests[$id].Bound='' }
  }
  $state.Decided=@()
}
function Read-DesktopLine($state, $line) {
  if ($line -notmatch '^(\d{4}-\d\d-\d\d \d\d:\d\d:\d\d) \[info\] (.*)\r?$') { return }
  $when = [DateTime]::ParseExact($Matches[1], 'yyyy-MM-dd HH:mm:ss', [Globalization.CultureInfo]::InvariantCulture)
  $at = ([DateTimeOffset]::new($when)).ToUnixTimeMilliseconds()
  $body = $Matches[2].TrimEnd("`r")
  if ($body -match '^Mapping internal session (local_[\w-]+) to CLI session ([\w-]+)$') {
    $state.Sessions[$Matches[1]] = $Matches[2]; return
  }
  if ($body -match '^Emitted tool permission request ([\w-]+) for ([\w:.-]+) in session (local_[\w-]+)$') {
    $id=$Matches[1]; $tool=$Matches[2]; $session=$Matches[3]
    if (-not $state.Requests.ContainsKey($id)) {
      $state.Requests[$id] = @{ Session=$session; Tool=$tool; At=$at; Bound=''; Ambiguous=$false }
    }
    return
  }
  if ($body -match '^Received permission response for ([\w-]+): ([\w-]+) \(tool: ([\w:.-]+)\)$') {
    $state.Answers[$Matches[1]] = @{ Decision=$Matches[2]; Tool=$Matches[3]; At=$at }; return
  }
  # A dialog taken down unanswered — its CLI process ended under it — no
  # longer waits on anybody.
  if ($body -match '^Permission request ([\w-]+) for ([\w:.-]+) aborted$') {
    $state.Answers[$Matches[1]] = @{ Decision='aborted'; Tool=$Matches[2]; At=$at }; return
  }
  if ($body -match 'permission request|permission response|Mapping internal session') { $state.Warning='unrecognized-log-format' }
}
function Read-DesktopFiles($state) {
  $parent = [IO.Path]::GetDirectoryName($state.Path)
  $paths = @((Join-Path $parent 'main1.log'), $state.Path)
  $opened = @()
  try {
    foreach ($path in $paths) {
      if (-not (Test-Path -LiteralPath $path)) { continue }
      $stream = [IO.File]::Open($path, 'Open', 'Read', 'ReadWrite, Delete')
      if ($stream.Length -gt 33554432) { $stream.Dispose(); throw 'log-too-large; check log rotation' }
      try { $key = [PggDesktopLog]::Key($stream) } catch { $stream.Dispose(); throw }
      $opened += ,@($key, $stream)
    }
    if ($opened.Count -eq 0) { throw 'log-unavailable' }
    $overlap = @($opened | Where-Object { $state.Files.ContainsKey($_[0]) }).Count
    $truncated = @($opened | Where-Object { $state.Files.ContainsKey($_[0]) -and $_[1].Length -lt $state.Files[$_[0]].Reader.BaseStream.Position }).Count
    if ((($state.Files.Count -gt 0) -and ($overlap -eq 0)) -or ($truncated -gt 0)) {
      Reset-DesktopLog $state
      $state.Warning='log-gap-replayed'
    }
    # Read the renamed file before its successor; identity, not length,
    # preserves the cursor even when the successor is already larger.
    foreach ($pair in $opened) {
      $key, $stream = $pair
      if (-not $state.Files.ContainsKey($key)) {
        $state.Files[$key] = @{ Reader=[IO.StreamReader]::new($stream, [Text.Encoding]::UTF8); Tail='' }
        $pair[1] = $null
      }
      $cursor=$state.Files[$key]
      $text=$cursor.Tail + $cursor.Reader.ReadToEnd()
      $lines=$text -split "`n"
      $cursor.Tail=$lines[-1]
      for ($i=0; $i -lt $lines.Length-1; $i++) {
        if (($lines[$i].IndexOf('permission', [StringComparison]::OrdinalIgnoreCase) -ge 0) -or $lines[$i].Contains('Mapping internal session')) { Read-DesktopLine $state $lines[$i] }
      }
      if ($cursor.Tail.Length -gt 1048576) { throw 'log-line-too-long' }
    }
    foreach ($key in @($state.Files.Keys)) {
      if (@($opened | Where-Object { $_[0] -eq $key }).Count -eq 0) {
        $state.Files[$key].Reader.Dispose(); $state.Files.Remove($key)
      }
    }
  } finally {
    foreach ($pair in $opened) { if ($null -ne $pair[1]) { $pair[1].Dispose() } }
  }
}
function Test-DesktopTool($logged, $called) {
  # Permission for AskUserQuestion is not the person's answer to it.
  if ($called -eq 'AskUserQuestion') { return $false }
  if ($logged -eq $called) { return $true }
  # Desktop names its own dialogs by kind, not by the tool that raised one:
  # opening a file asks from navigate, preview_start and browser_batch alike.
  $servers = switch -Regex ($logged) {
    '^browser:' { 'mcp__Claude_Browser__', 'mcp__claude-in-chrome__' }
    '^computer:' { 'mcp__computer-use__' }
  }
  foreach ($server in $servers) { if ($called.StartsWith($server)) { return $true } }
  return $false
}
function Update-DesktopLog($state, $candidates, $nowms) {
  $state.Overlay=@{}; $state.Decided=@()
  if (Test-Path -LiteralPath "$dir\desktop-log.off") {
    Reset-DesktopLog $state; $state.Status='disabled'; return
  }
  try {
    Read-DesktopFiles $state
    if ($state.Warning -eq 'unrecognized-log-format') { throw 'unrecognized-log-format' }
    $state.Enabled=$true; $state.Seen=$nowms
    $active=@{}; $covered=@{}; $blocked=@{}; $unknown=0; $ambiguous=0; $unmapped=0
    foreach ($candidate in $candidates) { $active[$candidate.Key]=$candidate }
    foreach ($id in @($state.Requests.Keys)) {
      $request=$state.Requests[$id]
      $cli=$state.Sessions[$request.Session]
      if (-not $cli) { continue }
      if ($request.Bound -and -not $active.ContainsKey($request.Bound)) {
        $request.Bound='finished'
      }
      if (-not $request.Bound -and -not $request.Ambiguous) {
        $matchesCall=@($candidates | Where-Object {
          ($_.Session -eq $cli) -and ([int64]$_.Part[4] -le $request.At+999) -and
          (Test-DesktopTool $request.Tool $_.Part[2])
        })
        if ($matchesCall.Count -eq 1) { $request.Bound=$matchesCall[0].Key; $state.Decided += $id }
        elseif ($matchesCall.Count -gt 1) { $request.Ambiguous=$true }
      }
      if ($request.Ambiguous) { $ambiguous++; continue }
      if (-not $active.ContainsKey($request.Bound)) {
        if (-not $request.Bound -and @($candidates | Where-Object {
          ($_.Session -eq $cli) -and ([int64]$_.Part[4] -le $request.At+999)
        }).Count -gt 0) { $unmapped++ }
        continue
      }
      $key=$request.Bound
      $covered[$key]=$true
      $answer=$state.Answers[$id]
      $asked=$true
      if ($answer) {
        if (($answer.Tool -ne $request.Tool) -or ($answer.At -lt $request.At)) { $unknown++; $blocked[$key]=$true; continue }
        if ($answer.Decision -in @('once','always','aborted')) { $asked=$false }
        else { $unknown++; $blocked[$key]=$true; continue }
      }
      # Every pending dialog on this call must resolve before resuming.
      if ($state.Overlay.ContainsKey($key)) { $asked=$asked -or $state.Overlay[$key] }
      $state.Overlay[$key]=$asked
    }
    foreach ($key in $blocked.Keys) { $state.Overlay.Remove($key) }
    $unmatched=0
    foreach ($candidate in $candidates) {
      $key=$candidate.Key
      if (($candidate.Part[6] -eq '1') -and ($candidate.Part[2] -ne 'AskUserQuestion') -and -not $covered[$key]) {
        if (-not $state.Asked.ContainsKey($key)) { $state.Asked[$key]=$nowms }
        if ($nowms-$state.Asked[$key] -ge 5000) { $unmatched++ }
      } else { $state.Asked.Remove($key) }
    }
    foreach ($key in @($state.Asked.Keys)) { if (-not $active.ContainsKey($key)) { $state.Asked.Remove($key) } }
    # Bound history without turning an evicted request into permission.
    if ($state.Requests.Count -gt 10000 -or $state.Answers.Count -gt 10000 -or $state.Sessions.Count -gt 10000) { throw 'log-history-limit; check log rotation' }
    $state.Status="watching overlay=$($state.Overlay.Count) unmatched=$unmatched ambiguous=$ambiguous unknown=$unknown unmapped=$unmapped $($state.Warning)"
  } catch {
    Reset-DesktopLog $state
    $state.Status='fallback: ' + $_.Exception.Message
  }
}
function Get-DesktopAsked($state, $session, $part) {
  $key="$session/$($part[0])/$($part[1])/$($part[4])"
  if ($state.Enabled -and $state.Overlay.ContainsKey($key)) { return [bool]$state.Overlay[$key] }
  return $part[6] -eq '1'
}
"#;
