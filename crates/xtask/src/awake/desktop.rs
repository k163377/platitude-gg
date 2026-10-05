//! Optional interpretation of Desktop's private permission log. It never
//! changes hook claims: losing evidence drops the overlay, not hook facts.

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
        if ($lines[$i].Contains('permission') -or $lines[$i].Contains('Mapping internal session')) { Read-DesktopLine $state $lines[$i] }
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
        if ($answer.Decision -in @('once','always')) { $asked=$false }
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
