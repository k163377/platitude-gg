//! Display-mode evidence, outside the timed app process. Never change a mode.
use std::path::Path;

pub(super) fn capture(directory: &Path, phase: &str) -> Result<(), String> {
    let evidence = match query() {
        Ok(evidence) => evidence,
        Err(error) => {
            println!(
                "  display modes unavailable: {error}; fps comparison needs separate evidence."
            );
            format!("unavailable: {error}\n")
        }
    };
    std::fs::write(directory.join(format!("display-{phase}.txt")), evidence)
        .map_err(|e| e.to_string())
}

#[cfg(windows)]
fn query() -> Result<String, String> {
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", WINDOWS_QUERY])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        return Err("no active display answered".into());
    }
    Ok(text)
}

#[cfg(not(windows))]
fn query() -> Result<String, String> {
    Err("native display-mode capture is currently implemented only on Windows".into())
}

// DEVMODEW's Unicode layout is fixed in wingdi.h on both x86 and x64.
// Only read fields owned by EnumDisplaySettingsW, with dmSize initialized.
// The setting is an integer nominal Hz, not fractional timing or VRR scanout.
// https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-enumdisplaysettingsw
#[cfg(windows)]
const WINDOWS_QUERY: &str = r#"
$ErrorActionPreference='Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class PerfDisplayModes {
    [StructLayout(LayoutKind.Explicit, CharSet=CharSet.Unicode, Size=220)]
    public struct Mode {
        [FieldOffset(68)] public ushort Size;
        [FieldOffset(72)] public uint Fields;
        [FieldOffset(168)] public uint BitsPerPixel;
        [FieldOffset(172)] public uint Width;
        [FieldOffset(176)] public uint Height;
        [FieldOffset(180)] public uint Flags;
        [FieldOffset(184)] public uint Frequency;
    }
    [DllImport("user32.dll", CharSet=CharSet.Unicode, ExactSpelling=true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool EnumDisplaySettingsW(string device, int mode, ref Mode value);
    public static Mode Read(string device) {
        var value = new Mode();
        value.Size = (ushort)Marshal.SizeOf(typeof(Mode));
        if (!EnumDisplaySettingsW(device, -1, ref value))
            throw new InvalidOperationException("EnumDisplaySettingsW failed for " + device);
        return value;
    }
}
'@
$modes = @([System.Windows.Forms.Screen]::AllScreens | Sort-Object DeviceName | ForEach-Object {
    $mode = [PerfDisplayModes]::Read($_.DeviceName)
    $hz = $null
    if (($mode.Fields -band 0x400000) -ne 0 -and $mode.Frequency -gt 1) { $hz = $mode.Frequency }
    [ordered]@{
        name=$_.DeviceName; primary=$_.Primary;
        width_px=$mode.Width; height_px=$mode.Height;
        nominal_hz=$hz; frequency_raw=$mode.Frequency;
        bits_per_pixel=$mode.BitsPerPixel; fields=$mode.Fields; flags=$mode.Flags
    }
})
if ($modes.Count -eq 0) { throw 'no active display answered' }
ConvertTo-Json -InputObject $modes -Depth 3 -Compress
"#;
