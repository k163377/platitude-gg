//! Desktop's local debug log supplies actual human waits. Its overlay
//! never changes hook records or lies on the app-server communication path.

#[cfg(windows)]
pub(in crate::awake) const SCRIPT: &str = r#"
function New-CodexLog($root) {
  return @{ Root=$root; Files=@{}; Requests=@{}; Answers=@{}; Overlay=@{}; Pending=@{};
    Decided=@(); Enabled=$false; DebugAt=0; Status='starting'; Warning='' }
}
function Close-CodexLog($state) {
  foreach ($file in $state.Files.Values) { $file.Reader.Dispose() }
  $state.Files=@{}
}
function Reset-CodexLog($state) {
  Close-CodexLog $state
  $state.Requests=@{}; $state.Answers=@{}; $state.Overlay=@{}; $state.Pending=@{}
  $state.Decided=@(); $state.Enabled=$false; $state.DebugAt=0; $state.Warning=''
}
function Undo-CodexDecisions($state) {
  foreach ($key in $state.Decided) {
    if ($state.Requests.ContainsKey($key)) { $state.Requests[$key].Bound=''; $state.Requests[$key].Calls=@() }
  }
  $state.Decided=@()
}
function Read-CodexTags($text) {
  $tags=@{}; $at=0
  while ($at -lt $text.Length) {
    while ($at -lt $text.Length -and $text[$at] -eq ' ') { $at++ }
    if ($at -eq $text.Length) { break }
    $match=[regex]::Match($text.Substring($at), '^([\w.-]+)=')
    if (-not $match.Success) { throw 'unrecognized-codex-tags' }
    $key=$match.Groups[1].Value; $at+=$match.Length; $start=$at
    $depth=0; $quoted=$false; $escaped=$false
    while ($at -lt $text.Length) {
      $ch=$text[$at]
      if ($quoted) {
        if ($escaped) { $escaped=$false }
        elseif ($ch -eq '\') { $escaped=$true }
        elseif ($ch -eq '"') { $quoted=$false }
      } elseif ($ch -eq '"') { $quoted=$true }
      elseif ($ch -eq '{' -or $ch -eq '[') { $depth++ }
      elseif ($ch -eq '}' -or $ch -eq ']') { $depth-- }
      elseif ($ch -eq ' ' -and $depth -eq 0) { break }
      $at++
    }
    if ($quoted -or $depth -ne 0 -or $start -eq $at) { throw 'incomplete-codex-tags' }
    $value=$text.Substring($start,$at-$start)
    if ($tags.ContainsKey($key)) { throw 'duplicate-codex-tag' }
    if ($value.StartsWith('{') -or $value.StartsWith('[') -or $value.StartsWith('"')) {
      $tags[$key]=ConvertFrom-Json -InputObject $value -ErrorAction Stop
    } else { $tags[$key]=$value }
  }
  return $tags
}
function Read-CodexLine($state, $source, $line) {
  if ($line -notmatch '^(\S+) (debug|info|trace|warning|error) \[([^\]]+)\] (.*)\r?$') { return }
  $when=$Matches[1]; $level=$Matches[2]; $logger=$Matches[3]; $body=$Matches[4].TrimEnd("`r")
  $at=[DateTimeOffset]::Parse($when,[Globalization.CultureInfo]::InvariantCulture).ToUnixTimeMilliseconds()
  if ($level -in @('debug','trace')) { $state.DebugAt=[Math]::Max($at,[int64]$state.DebugAt) }
  $kind=''
  if ($body.StartsWith('Received server request ')) { $kind='request'; $body=$body.Substring(24) }
  elseif ($body.StartsWith('Sending server response ')) { $kind='answer'; $body=$body.Substring(24) }
  else { return }
  $tags=Read-CodexTags $body
  $method=$tags['method']
  if ($method -notin @('item/commandExecution/requestApproval','item/fileChange/requestApproval',
      'item/permissions/requestApproval','item/tool/requestUserInput','mcpServer/elicitation/request')) { return }
  if (-not $tags.ContainsKey('id')) { throw 'missing-codex-request-id' }
  $scope="$source/$logger/$($tags['rendererWebContentsId'])/$($tags['hostId'])"
  $key="$scope/$($tags['id'])"
  if ($kind -eq 'answer') {
    if (-not $tags.ContainsKey('response') -or $tags['response'] -isnot [pscustomobject]) { throw 'missing-codex-response-object' }
    if ($method -in @('item/commandExecution/requestApproval','item/fileChange/requestApproval')) {
      if (-not $tags['response'].decision) { throw 'missing-codex-decision' }
    } elseif ($method -eq 'item/permissions/requestApproval') {
      if (-not $tags['response'].permissions) { throw 'missing-codex-permissions' }
    } elseif ($method -eq 'item/tool/requestUserInput') {
      if (-not $tags['response'].answers) { throw 'missing-codex-answers' }
    } elseif (-not $tags['response'].action) { throw 'missing-codex-elicitation-action' }
    $state.Answers[$key]=@{ At=$at; Method=$method }
    return
  }
  $params=$tags['params']
  if (-not $params -or -not $params.threadId) { throw 'missing-codex-thread' }
  if ($method -eq 'item/tool/requestUserInput' -and $params.isBlocking -eq $false) { return }
  if ($method -eq 'mcpServer/elicitation/request' -and $params.mode -eq 'openai/userVerification') { return }
  $item="$($params.itemId)"; $server="$($params.serverName)"
  if (($method -eq 'mcpServer/elicitation/request' -and -not $server) -or
      ($method -ne 'mcpServer/elicitation/request' -and -not $item)) { throw 'missing-codex-call' }
  if ($state.Requests.ContainsKey($key)) {
    $old=$state.Requests[$key]
    if ($old.At -eq $at -and $old.Thread -eq "$($params.threadId)" -and $old.Item -eq $item -and
        $old.Method -eq $method -and $old.Server -eq $server) { return }
    # A reused id cannot make an answer belonging to another connection
    # resume this call. Without a connection identity, leave hook facts.
    $old.Ambiguous=$true; $state.Warning='ambiguous-request-id'; return
  }
  $state.Requests[$key]=@{ At=$at; Thread="$($params.threadId)"; Item=$item; Server=$server;
    Method=$method; Bound=''; Calls=@(); Ambiguous=$false }
}
function Read-CodexFiles($state) {
  $roots=@($state.Root)
  if (Test-Path -LiteralPath "$dir\codex-log.path") { $roots=@([IO.File]::ReadAllText("$dir\codex-log.path").Trim()) }
  $paths=@(); $days=@{}
  foreach ($now in @([DateTime]::Now,[DateTime]::UtcNow)) {
    foreach ($offset in @(0,-1)) { $days[$now.AddDays($offset).ToString('yyyy/MM/dd')]=1 }
  }
  foreach ($root in $roots) {
    foreach ($day in $days.Keys) {
      $folder=Join-Path $root $day
      if (Test-Path -LiteralPath $folder) {
        $paths+=@(Get-ChildItem -LiteralPath $folder -Filter 'codex-desktop-*.log' -File | Sort-Object Name)
      }
    }
  }
  if ($paths.Count -eq 0) { throw 'codex-log-unavailable' }
  if ($paths.Count -gt 128) { throw 'codex-log-file-limit' }
  $opened=@(); $keys=@{}; $lines=@(); $bytes=0
  try {
    foreach ($path in $paths) {
      $stream=[IO.File]::Open($path.FullName,'Open','Read','ReadWrite, Delete')
      if ($stream.Length -gt 33554432) { $stream.Dispose(); throw 'codex-log-too-large' }
      try { $key=[PggDesktopLog]::Key($stream) } catch { $stream.Dispose(); throw }
      if ($keys.ContainsKey($key)) { $stream.Dispose(); continue }
      $keys[$key]=1
      $bytes+=$stream.Length
      if ($bytes -gt 67108864) { $stream.Dispose(); throw 'codex-log-byte-limit' }
      $opened+=,@($key,$stream,$path.Name)
    }
    $overlap=@($opened | Where-Object { $state.Files.ContainsKey($_[0]) }).Count
    $truncated=@($opened | Where-Object { $state.Files.ContainsKey($_[0]) -and $_[1].Length -lt $state.Files[$_[0]].Reader.BaseStream.Position }).Count
    if (($state.Files.Count -gt 0 -and $overlap -eq 0) -or $truncated -gt 0) {
      Reset-CodexLog $state; $state.Warning='codex-log-gap-replayed'
    }
    foreach ($pair in $opened) {
      $key,$stream,$name=$pair
      if ($name -notmatch '^codex-desktop-([\w-]+)-\d+-t\d+-i\d+-\d+-\d+\.log$') { continue }
      $source=$Matches[1]
      if (-not $state.Files.ContainsKey($key)) {
        $state.Files[$key]=@{ Reader=[IO.StreamReader]::new($stream,[Text.Encoding]::UTF8); Tail=''; Source=$source }
        $pair[1]=$null
      }
      $cursor=$state.Files[$key]; $text=$cursor.Tail+$cursor.Reader.ReadToEnd(); $parts=$text -split "`n"
      $cursor.Tail=$parts[-1]
      for ($i=0;$i -lt $parts.Length-1;$i++) {
        if ($parts[$i].Length -gt 1048576) { throw 'codex-log-line-too-long' }
        if ($parts[$i] -match '^(\S+) (debug|trace) ') {
          $debugAt=[DateTimeOffset]::Parse($Matches[1],[Globalization.CultureInfo]::InvariantCulture).ToUnixTimeMilliseconds()
          $state.DebugAt=[Math]::Max($debugAt,[int64]$state.DebugAt)
        }
        if ($parts[$i].Contains('Received server request ') -or $parts[$i].Contains('Sending server response ')) {
          $lines+=@{ Source=$cursor.Source; Text=$parts[$i]; Stamp=($parts[$i] -split ' ',2)[0] }
        }
      }
      if ($cursor.Tail.Length -gt 1048576) { throw 'codex-log-line-too-long' }
    }
    foreach ($line in @($lines | Sort-Object Stamp)) { Read-CodexLine $state $line.Source $line.Text }
    foreach ($key in @($state.Files.Keys)) {
      if (@($opened | Where-Object { $_[0] -eq $key }).Count -eq 0) {
        $state.Files[$key].Reader.Dispose(); $state.Files.Remove($key)
      }
    }
  } finally { foreach ($pair in $opened) { if ($null -ne $pair[1]) { $pair[1].Dispose() } } }
}
function Update-CodexLog($state,$candidates,$nowms) {
  $state.Overlay=@{}; $state.Pending=@{}; $state.Decided=@()
  if (Test-Path -LiteralPath "$dir\codex-log.off") { Reset-CodexLog $state; $state.Status='disabled'; return }
  try {
    Read-CodexFiles $state
    $state.Enabled=$true; $active=@{}; $ambiguous=0; $unmatched=0; $serverWaits=@()
    foreach ($candidate in $candidates) { $active[$candidate.Key]=$candidate }
    foreach ($key in @($state.Requests.Keys)) {
      $request=$state.Requests[$key]; $answer=$state.Answers[$key]
      if ($request.Ambiguous) { $ambiguous++; continue }
      if ($answer -and ($answer.At -lt $request.At -or $answer.Method -ne $request.Method)) { continue }
      $asked=-not $answer
      if ($request.Server) {
        # Elicitation has no item id. Bind only to the calls already open
        # on that thread/server. A changed set cannot release a later call.
        if (-not $request.Bound) {
          $found=@($candidates | Where-Object {
            $_.Thread -eq $request.Thread -and $_.Part[2].StartsWith("mcp__$($request.Server)__") -and
            [int64]$_.Part[4] -le $request.At
          })
          if ($found.Count -eq 1) { $request.Calls=@($found[0].Key); $request.Bound='server'; $state.Decided+=$key }
          elseif ($found.Count -gt 1) { $request.Ambiguous=$true; $ambiguous++; continue }
          elseif (@($candidates | Where-Object { $_.Thread -eq $request.Thread }).Count) { $unmatched++ }
        }
        if ($request.Bound -eq 'server' -and @($request.Calls | Where-Object { -not $active.ContainsKey($_) }).Count) {
          $request.Bound='finished'
        }
        if ($asked -and $request.Bound -eq 'server') {
          $serverWaits+=$request
        }
        continue
      }
      if ($request.Bound -and -not $active.ContainsKey($request.Bound)) { $request.Bound='finished' }
      if (-not $request.Bound) {
        $found=@($candidates | Where-Object {
          $_.Thread -eq $request.Thread -and $_.Item -eq $request.Item -and [int64]$_.Part[4] -le $request.At
        })
        if ($found.Count -eq 1) { $request.Bound=$found[0].Key; $state.Decided+=$key }
        elseif ($found.Count -gt 1) { $request.Ambiguous=$true; $ambiguous++; continue }
      }
      if ($active.ContainsKey($request.Bound)) {
        $bound=$request.Bound
        $state.Overlay[$bound]=$asked -or ($state.Overlay.ContainsKey($bound) -and $state.Overlay[$bound])
      } elseif (-not $request.Bound -and @($candidates | Where-Object { $_.Thread -eq $request.Thread }).Count) { $unmatched++ }
    }
    $waitingCalls=@{}
    foreach ($request in $serverWaits) {
      # An item-level wait may already exclude this call from work. Do not
      # subtract its elicitation again from another call on the server.
      $alreadyAsked=@($request.Calls | Where-Object {
        if ($state.Overlay.ContainsKey($_)) { $state.Overlay[$_] } else { $active[$_].Part[6] -eq '1' }
      }).Count
      if ($alreadyAsked) { continue }
      $serverKey="$($request.Thread)/$($request.Server)"
      $callKey=$request.Calls[0]
      if ($waitingCalls.ContainsKey($callKey)) { continue }
      $waitingCalls[$callKey]=1
      $state.Pending[$serverKey]=1+[int]$state.Pending[$serverKey]
    }
    if ($state.Requests.Count -gt 10000 -or $state.Answers.Count -gt 10000) { throw 'codex-log-history-limit' }
    $debug='not-observed'; if ($state.DebugAt -gt 0) { $debug="observed age-ms=$([Math]::Max(0,$nowms-$state.DebugAt))" }
    $state.Status="watching debug=$debug overlay=$($state.Overlay.Count) pending=$($state.Pending.Count) unmatched=$unmatched ambiguous=$ambiguous $($state.Warning)"
  } catch { Reset-CodexLog $state; $state.Status='fallback: '+$_.Exception.Message }
}
function New-CodexCandidate($session,$part) {
  $root=($session -replace '^c\d+\.','' -split '~',2)[0]
  $thread=$root; if ($part[0] -ne '-') { $thread=$part[0] }
  $item=$part[1]; $prefix=$part[0]+'.'
  if ($item.StartsWith($prefix)) { $item=$item.Substring($prefix.Length) }
  return @{ Key="$session/$($part[0])/$($part[1])/$($part[4])"; Thread=$thread; Item=$item; Part=$part }
}
function Get-CodexAsked($state,$session,$part) {
  $key="$session/$($part[0])/$($part[1])/$($part[4])"
  if ($state.Enabled -and $state.Overlay.ContainsKey($key)) { return [bool]$state.Overlay[$key] }
  return $part[6] -eq '1'
}
function Get-CodexPending($state,$session,$who) {
  $pending=@{}
  if (-not $state.Enabled) { return $pending }
  $thread=($session -replace '^c\d+\.','' -split '~',2)[0]
  if ($who -ne '-') { $thread=$who }
  foreach ($key in $state.Pending.Keys) {
    if ($key.StartsWith($thread+'/')) { $pending[$key.Substring($thread.Length+1)]=$state.Pending[$key] }
  }
  return $pending
}
"#;
