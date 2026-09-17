//! The screens this machine has, outside the timed app process. Read only.
//!
//! Read for two things. It is evidence — what the modes were on
//! either side of a run — and it is also what the run is *placed* against:
//! the window is pinned onto one named screen (`perf::screen`), and that
//! screen's nominal Hz is the ceiling the delivered frames are read
//! against. A machine whose monitors do not all run at one rate answers
//! a different fps for the same application on each of them otherwise
//! (the rig's own screens: ci/baseline/perf-windows-x64.md §計測条件).
use std::path::Path;

/// One screen, as both the OS mode table and the virtual desktop see it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Screen {
    pub(super) name: String,
    pub(super) primary: bool,
    /// Virtual-desktop bounds, which is the coordinate space a window's
    /// `x` / `y` are in.
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) width: i32,
    pub(super) height: i32,
    /// Nominal refresh, or 0 where the mode table did not own one.
    pub(super) hz: u32,
}

/// What the screens are right now: the evidence text, and the same thing
/// read. Always answers — a machine whose modes cannot be read still
/// takes a measurement, it just cannot pin the window or name a refresh
/// rate, and the note says so once.
pub(super) fn survey() -> (String, Vec<Screen>) {
    match query() {
        Ok(evidence) => {
            let screens = parse(&evidence);
            (evidence, screens)
        }
        Err(error) => {
            println!(
                "  display modes unavailable: {error}; fps comparison needs separate evidence."
            );
            (format!("unavailable: {error}\n"), Vec::new())
        }
    }
}

pub(super) fn capture(directory: &Path, phase: &str) -> Result<(), String> {
    let (evidence, _) = survey();
    std::fs::write(directory.join(format!("display-{phase}.txt")), evidence)
        .map_err(|e| e.to_string())
}

/// The screen a run is placed on: the one named, or the primary.
pub(super) fn choose<'a>(screens: &'a [Screen], named: &str) -> Result<&'a Screen, String> {
    if named.is_empty() {
        return screens
            .iter()
            .find(|screen| screen.primary)
            .or_else(|| screens.first())
            .ok_or_else(|| "no screen answered, so the window cannot be placed".to_string());
    }
    screens
        .iter()
        .find(|screen| screen.name == named)
        .ok_or_else(|| {
            format!(
                "no screen is called {named} — this machine has {}",
                screens
                    .iter()
                    .map(|screen| screen.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
}

/// One `key=value` line per screen, in the same shape as the app's own log
/// so one reader serves both.
fn parse(text: &str) -> Vec<Screen> {
    text.lines()
        .filter_map(|line| {
            let field = |key: &str| super::reading::token(line, key);
            let number = |key: &str| field(key).and_then(|v| v.parse::<i32>().ok());
            Some(Screen {
                name: field("name=")?.to_string(),
                primary: field("primary=") == Some("true"),
                x: number("x=")?,
                y: number("y=")?,
                width: number("width=")?,
                height: number("height=")?,
                hz: field("hz=").and_then(|v| v.parse().ok()).unwrap_or(0),
            })
        })
        .collect()
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
// The setting is an integer nominal Hz.
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
$lines = @([System.Windows.Forms.Screen]::AllScreens | Sort-Object DeviceName | ForEach-Object {
    $mode = [PerfDisplayModes]::Read($_.DeviceName)
    $hz = 0
    if (($mode.Fields -band 0x400000) -ne 0 -and $mode.Frequency -gt 1) { $hz = $mode.Frequency }
    $b = $_.Bounds
    ('name={0} primary={1} x={2} y={3} width={4} height={5} hz={6} bpp={7} mode_width={8} mode_height={9} fields={10} flags={11}' -f `
        $_.DeviceName, $_.Primary.ToString().ToLower(), $b.X, $b.Y, $b.Width, $b.Height, $hz,
        $mode.BitsPerPixel, $mode.Width, $mode.Height, $mode.Fields, $mode.Flags)
})
if ($lines.Count -eq 0) { throw 'no active display answered' }
$lines -join "`n"
"#;

#[cfg(test)]
mod tests {
    use super::{Screen, choose, parse};

    const THREE: &str = "name=\\\\.\\DISPLAY1 primary=false x=-1920 y=0 width=1920 height=1080 hz=100 bpp=32\n\
                         name=\\\\.\\DISPLAY2 primary=true x=0 y=0 width=1920 height=1080 hz=180 bpp=32\n\
                         name=\\\\.\\DISPLAY3 primary=false x=1920 y=0 width=1920 height=1080 hz=0 bpp=32\n";

    #[test]
    fn a_screen_line_carries_its_place_and_its_rate() {
        let screens = parse(THREE);
        assert_eq!(screens.len(), 3);
        assert_eq!(
            screens[0],
            Screen {
                name: "\\\\.\\DISPLAY1".into(),
                primary: false,
                x: -1920,
                y: 0,
                width: 1920,
                height: 1080,
                hz: 100,
            }
        );
        assert_eq!(screens[1].hz, 180);
        assert!(screens[1].primary);
        // A mode table that owned no frequency answers 0.
        assert_eq!(screens[2].hz, 0);
    }

    #[test]
    fn an_unreadable_capture_yields_no_screens() {
        assert!(parse("unavailable: no active display answered\n").is_empty());
    }

    #[test]
    fn the_primary_is_the_default_and_a_name_wins() {
        let screens = parse(THREE);
        assert_eq!(choose(&screens, "").map(|s| s.hz), Ok(180));
        assert_eq!(choose(&screens, "\\\\.\\DISPLAY3").map(|s| s.x), Ok(1920));
        let unknown = choose(&screens, "\\\\.\\DISPLAY9").unwrap_err();
        assert!(unknown.contains("DISPLAY1"), "{unknown}");
        assert!(choose(&[], "").is_err());
    }
}
