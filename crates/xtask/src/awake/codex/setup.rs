//! One-time Desktop logging setup, with an exact previous-value ledger.
//! It changes neither CLI selection nor approval policy and launches no app.

use crate::command::{Command, Permission, Where};

pub(in crate::awake) static SETUP: Command = Command {
    id: "awake.codex.setup",
    call: "awake codex setup",
    purpose: "enable Desktop debug logs once for normal-icon sleep observation",
    run_in: Where::Either,
    needs: &["Windows Codex Desktop; restart it after setup"],
    permission: Permission::Plain,
};
pub(in crate::awake) static UNDO: Command = Command {
    id: "awake.codex.setup.undo",
    call: "awake codex setup --undo",
    purpose: "restore the previous Desktop log level without overwriting a later user change",
    run_in: Where::Either,
    needs: &["this repository's logging setup ledger"],
    permission: Permission::Plain,
};

pub(super) fn run(args: &[String]) -> Result<(), String> {
    let undo = match args {
        [] => false,
        [flag] if flag == "--undo" => true,
        _ => return Err("awake codex setup accepts only --undo".into()),
    };
    run_windows(undo)
}

#[cfg(not(windows))]
fn run_windows(_undo: bool) -> Result<(), String> {
    Err("Codex Desktop sleep setup is Windows-only".into())
}

#[cfg(windows)]
fn run_windows(undo: bool) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let dir = crate::awake::dir_of(&cwd.to_string_lossy()).ok_or("not in this repository")?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let dir = dir.to_string_lossy().replace('\'', "''");
    let script = format!(
        "$ErrorActionPreference='Stop'\n{SCRIPT}\n{WINDOWS}\nInvoke-CodexSetup '{dir}' ${undo}",
    );
    let status = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("Codex logging setup failed; see its diagnostic".into())
    }
}

#[cfg(windows)]
const WINDOWS: &str = r#"
function Read-CodexLevel { return [Environment]::GetEnvironmentVariable('CODEX_MAX_LOG_LEVEL','User') }
function Write-CodexLevel($value) { [Environment]::SetEnvironmentVariable('CODEX_MAX_LOG_LEVEL',$value,'User') }
function Notify-CodexLevel {
  Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class PggCodexEnvironment {
  [DllImport("user32.dll", CharSet=CharSet.Unicode)]
  public static extern IntPtr SendMessageTimeout(IntPtr window, uint message, IntPtr wparam,
    string lparam, uint flags, uint timeout, out IntPtr result);
}
'@
  $result=[IntPtr]::Zero
  [void][PggCodexEnvironment]::SendMessageTimeout([IntPtr]65535,26,[IntPtr]::Zero,'Environment',2,2000,[ref]$result)
}
"#;

#[cfg(windows)]
pub(in crate::awake) const SCRIPT: &str = r#"
function Invoke-CodexSetup($dir,$undo) {
  $path=Join-Path $dir 'codex-log-level.previous'
  $guard=[IO.File]::Open((Join-Path $dir 'codex-setup.lock'),'OpenOrCreate','ReadWrite','ReadWrite')
  try {
    $guard.Lock(0,1)
    $current=Read-CodexLevel
    if ($undo) {
      if (Test-Path -LiteralPath $path) {
        $text=[IO.File]::ReadAllText($path)
        if ($text -eq 'missing') { $previous=$null }
        elseif ($text.StartsWith('value:')) { $previous=[Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($text.Substring(6))) }
        else { throw 'Invalid Codex logging ledger; no settings were changed' }
        if ($current -eq 'debug') {
          Write-CodexLevel $previous; Notify-CodexLevel
          Write-Output 'Restored the previous Desktop log level.'
        } else { Write-Output 'Preserved the log level changed after setup.' }
        [IO.File]::Delete($path)
      } else { Write-Output 'No log-level change belongs to this setup.' }
      [IO.File]::WriteAllText((Join-Path $dir 'codex-log.off'),'disabled')
    } else {
      if (Test-Path -LiteralPath $path) {
        if ($current -ne 'debug') { throw 'Desktop log level changed since setup; run setup --undo before enabling again' }
      } elseif ($current -ne 'debug') {
        $saved='missing'
        if ($null -ne $current) { $saved='value:'+ [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($current)) }
        [IO.File]::WriteAllText($path,$saved)
        try { Write-CodexLevel 'debug'; Notify-CodexLevel }
        catch {
          if ((Read-CodexLevel) -ne 'debug') { [IO.File]::Delete($path) }
          throw
        }
      }
      [IO.File]::Delete((Join-Path $dir 'codex-log.off'))
      Write-Output 'Desktop debug logging is configured. Fully quit Desktop and launch its normal icon.'
      Write-Output 'Review the project activity hooks in Desktop Settings > Hooks (CLI: /hooks).'
    }
  } finally { $guard.Dispose() }
}
"#;
