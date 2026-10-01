//! The calls the rest of the app makes; each is a no-op off Windows.

#[cfg(windows)]
use super::win32;

/// Turns corner rounding off for every top-level window this thread owns.
/// Call it once the window is up — before that it quietly does nothing.
pub fn square_corners() {
    #[cfg(windows)]
    win32::square_corners();
}

/// Hands those same windows the app's own icon (title bar, taskbar,
/// Alt+Tab).
pub fn set_icon() {
    #[cfg(windows)]
    win32::set_icon();
}

/// Puts back the style bits Qt drops with the button and frameless hints
/// (`WS_MINIMIZEBOX` / `WS_MAXIMIZEBOX` / `WS_SYSMENU` / `WS_THICKFRAME`):
/// without them the platform refuses Win+Arrow, the taskbar's menu and
/// click-to-minimise, and resizing at the edges. They bring back the
/// capability, not the drawn buttons or frame.
pub fn keep_system_gestures() {
    #[cfg(windows)]
    win32::keep_system_gestures();
}

// A frameless window has no non-client area for `DWMWA_BORDER_COLOR` or
// `DWMWA_CAPTION_COLOR` to paint, so the edge line is drawn in the scene
// (`Main`).

/// Maximises the window, or puts it back, through the platform's own
/// command — the one the band's double-click sends. Qt maximises a
/// frameless window by resizing it, which leaves Windows no maximised
/// state to put back from.
pub fn set_maximized(maximized: bool) {
    #[cfg(windows)]
    win32::set_maximized(maximized);
    #[cfg(not(windows))]
    let _ = maximized;
}

/// Minimises through the platform's own command, as the window menu
/// does. `QWindow::setVisibility(Minimized)` clears
/// `WPF_RESTORETOMAXIMIZED` (`QWindowsWindow::setWindowState_sys`), so a
/// maximised window would come back an ordinary one.
pub fn minimize() {
    #[cfg(windows)]
    win32::minimize();
}

/// Pulls the window back inside the work area of the monitor `screen`
/// names (`\\.\DISPLAY2`, as `QScreen::name` spells it), and answers
/// whether it had to. Windowed windows only; moves where moving is
/// enough, and shrinks only as far as it must.
///
/// Named, because the platform first puts a window on the pointer's
/// screen, and fitting to that one moves a window saved on another
/// monitor. An empty name falls back to each window's nearest monitor
/// (a first run, or a monitor unplugged since the save).
///
/// Not in the scene: QML knows neither the work area nor the frame,
/// which is wider than the window says, and the frame is what must fit.
#[cfg(windows)]
pub fn fit_to_work_area(screen: &str) -> bool {
    win32::fit_to_work_area(screen)
}

/// Off Windows the window manager places its own windows.
#[cfg(not(windows))]
pub fn fit_to_work_area(screen: &str) -> bool {
    let _ = screen;
    false
}

// `WM_NCCALCSIZE` stays Qt's (rules-refs/app-ui.md「不採用: `WM_NCCALCSIZE` の横取り」);
// the way out is the window's custom margins, which the bridge does not expose.

/// Takes `WM_NCHITTEST` away from Qt for the windows that are up: every
/// point is client except the resize borders and the strips
/// `set_caption_strips` describes, which get the platform's own caption
/// behaviour (drag, snap, double-click, window menu).
///
/// Qt's own answer (`QWindowsWindow::handleNonClientHitTest`, 6.10–6.12)
/// synthesises presses into the scene from a `GetAsyncKeyState` poll; a
/// press whose release this window does not see leaves a phantom press
/// that swallows the next click and pins hover. Only a real mouse
/// reproduces it (`PostMessage` never moves `GetAsyncKeyState`). Unlike
/// `WM_NCCALCSIZE`, this is a pure query with no Qt state behind it.
pub fn take_frame_hit_test() {
    #[cfg(windows)]
    win32::take_frame_hit_test();
}

// The resize border's width is `hit_test`'s alone, in device pixels — not
// a scene value (`Main.qml`, above `flags`).

/// Where the band's empty runs sit, in logical scene pixels: `x0..x1`
/// each, from the window's top down to `bottom`. Scaled per hit test, so
/// a DPI change needs no new report. Every report carries all of them; a
/// run that has gone comes back empty.
pub fn set_caption_strips(runs: [(f64, f64); CAPTION_RUNS], bottom: f64) {
    #[cfg(windows)]
    win32::set_caption_strips(runs, bottom);
    #[cfg(not(windows))]
    let _ = (runs, bottom);
}

/// The band stretches answered as caption: the empty run past the last
/// tab, and the seam before the window's own buttons.
pub const CAPTION_RUNS: usize = 2;
