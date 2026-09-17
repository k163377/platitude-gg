//! The few the rest of the application calls. Each one is the whole
//! story on mac and Ubuntu, where there is nothing to say.

#[cfg(windows)]
use super::win32;

/// Turns the corner rounding off for every top-level window this thread
/// owns. Call it once the window is up: before that there is no handle
/// to set the attribute on, and the call quietly does nothing.
pub fn square_corners() {
    #[cfg(windows)]
    win32::square_corners();
}

/// Hands those same windows the app's own icon — the one the title bar,
/// the taskbar button and Alt+Tab all read.
pub fn set_icon() {
    #[cfg(windows)]
    win32::set_icon();
}

/// Puts back the styles that say what the system may do with the window.
///
/// The app draws the window's own buttons, so Qt is asked for none — and
/// Qt drops `WS_MINIMIZEBOX`, `WS_MAXIMIZEBOX` and `WS_SYSMENU` along with
/// them, and the frameless hints drop `WS_THICKFRAME` too. None of the
/// four is about drawing: without them the platform refuses Win+Arrow,
/// the taskbar's own menu, clicking the taskbar button to minimise, and
/// resizing at the edges. Putting them back does not bring the drawn
/// buttons or a visible frame back with them (measured: capability bits
/// return, pixels do not), because the caption they would be drawn in is
/// no longer part of this window's frame.
pub fn keep_system_gestures() {
    #[cfg(windows)]
    win32::keep_system_gestures();
}

// A frameless window has no non-client area for `DWMWA_BORDER_COLOR` or
// `DWMWA_CAPTION_COLOR` to reach — measured on a small one, the pixel
// outside the window is simply what was behind it — so the line the
// design wants is drawn in the scene, at the client's own edge (`Main`).

/// Maximises the window, or puts it back, through the platform's own
/// command — the same one the band's double-click sends.
///
/// Qt maximises a frameless window by resizing it, which leaves
/// Windows holding no maximised state at all (measured: the window
/// covers the work area with `IsZoomed` false), and putting it back
/// then has nothing to put back. Sending the command keeps one answer
/// for both gestures (`take_frame_hit_test`).
pub fn set_maximized(maximized: bool) {
    #[cfg(windows)]
    win32::set_maximized(maximized);
    #[cfg(not(windows))]
    let _ = maximized;
}

/// Puts the window down onto the taskbar through the platform's own
/// command — again the same one the window menu sends.
///
/// The reason is sharper than the maximise's:
/// `QWindow::setVisibility(Minimized)` carries a window state of
/// *minimised alone*, so Qt reads the maximise as being given up at the
/// same moment and clears `WPF_RESTORETOMAXIMIZED` off the placement to
/// match (`QWindowsWindow::setWindowState_sys`, Qt 6.10). The window
/// goes down maximised and comes back an ordinary one (observed).
/// Windows minimises on its own command without touching the flag, and
/// Qt keeps the maximised bit when it sees the resize (`SIZE_MINIMIZED`
/// adds to the state it holds), so both sides still agree on the way
/// back up.
pub fn minimize() {
    #[cfg(windows)]
    win32::minimize();
}

/// Pulls the window back inside the work area of the monitor `screen`
/// names (`\\.\DISPLAY2`, as `QScreen::name` spells it), and answers
/// whether it had to. Windowed windows only — a maximised one is the
/// platform's own arrangement.
///
/// **Named, because the right monitor is the saved one.**
/// Where the platform first puts a window is the pointer's screen, and
/// fitting the restore to that one is how a window saved on another
/// monitor ended up on this one (2026-09-05 実測, P3-確認事項). An empty
/// name falls back to each window's own nearest monitor, which is what a
/// first run and a monitor unplugged since the save both want.
///
/// The scene cannot do this itself: it knows neither the work area (QML
/// reports no screen's) nor the frame, which is wider than the window
/// says it is — and it is the *frame* that has to fit (measured: a
/// remembered 1920 came back as a 1936-wide frame at x=-5 on a 1920
/// monitor).
///
/// Moves wherever moving is enough, and shrinks only as far as it
/// must.
#[cfg(windows)]
pub fn fit_to_work_area(screen: &str) -> bool {
    win32::fit_to_work_area(screen)
}

/// Left to the window manager on the other two platforms, which place
/// their own windows.
#[cfg(not(windows))]
pub fn fit_to_work_area(screen: &str) -> bool {
    let _ = screen;
    false
}

// `WM_NCCALCSIZE` ("the client is the whole window") is Qt's to
// answer. From a subclass, Qt never sees the message and keeps the
// frame margins it cached at creation, so the client grows and the
// scene does not (measured: frame and client both 1450x908 with the
// scene still drawing 1434x900, black down the right edge and along
// the bottom). Nudging the size, forcing a recalculation and letting
// the message through first were all tried; none make Qt re-measure.
// What would is a way to set the window's custom margins, which the
// bridge does not expose.

/// Takes `WM_NCHITTEST` away from Qt for the windows that are up, and
/// answers it from the strips `set_caption_strips` describes.
///
/// This is the bug fix. Qt 6.10's own answer for an
/// `ExpandedClientAreaHint` + `CustomizeWindowHint` window
/// (`QWindowsWindow::handleNonClientHitTest`) polls `GetAsyncKeyState`
/// on *every* hit test, compares it against one `static` button state,
/// and on an edge delivers a synthesised press or release straight into
/// the scene — the hit-test answer is then whatever that synthetic event
/// came back with, `HTCAPTION` if nothing accepted it. That static
/// desyncs whenever a press and its release are not both seen by this
/// window (a click that lands in another window, a popup open at press
/// time), and from then on the scene holds a phantom press: the next
/// real click is swallowed as a caption click (logged live: the dead
/// clicks answered `WM_MOUSEACTIVATE` with hit=HTCAPTION), and the
/// phantom grab keeps hover pinned to one item, which is the wash that
/// stopped following the pointer and the highlights that stayed lit.
/// Synthetic clicks (`PostMessage`) never move `GetAsyncKeyState`, so
/// none of this reproduces under automation — only under a hand.
///
/// Answering the message ourselves starves that whole branch: every
/// point is client except the resize borders and the strips the QML
/// side says are grab-run, and those get the platform's own caption
/// behaviour — drag, snap, double-click, and the window menu on
/// right-click — through the front door.
///
/// This agrees with the `WM_NCCALCSIZE` finding above: that message
/// feeds frame metrics Qt caches and must keep seeing, while this one
/// is a pure query answered fresh every time, with no Qt state behind
/// it.
pub fn take_frame_hit_test() {
    #[cfg(windows)]
    win32::take_frame_hit_test();
}

// The resize border's width stays here: `hit_test` measures the edges
// by it, in device pixels, and is the only reader (written up on
// `Main.mainUi`).

/// Where the band's empty runs sit, in logical scene pixels: each from
/// its `x0` to its `x1`, all of them reaching down from the window's top
/// edge to `bottom` — they stand in the one band, so the one bottom
/// carries them. The subclass turns them into device pixels itself, per
/// hit test, so a DPI change needs no new report.
///
/// All of them together, on every report: a run that has gone comes
/// back as an empty one.
pub fn set_caption_strips(runs: [(f64, f64); CAPTION_RUNS], bottom: f64) {
    #[cfg(windows)]
    win32::set_caption_strips(runs, bottom);
    #[cfg(not(windows))]
    let _ = (runs, bottom);
}

/// How many stretches of the band the hit test answers caption for: the
/// empty run past the last tab, and the one the divider before the
/// window's buttons stands in.
pub const CAPTION_RUNS: usize = 2;
