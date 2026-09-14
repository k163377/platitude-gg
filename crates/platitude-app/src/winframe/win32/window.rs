//! The window itself: its corners, the styles the system gestures
//! need, where it sits, and whether it is maximised.

use super::*;

/// `DWMWA_WINDOW_CORNER_PREFERENCE` (dwmapi.h).
const CORNER_PREFERENCE: u32 = 33;
/// `DWMWCP_DONOTROUND` (dwmapi.h).
const DO_NOT_ROUND: u32 = 1;
/// `GWL_STYLE` and the three style bits the drawn buttons took with
/// them, plus the `SetWindowPos` flags that mean "nothing but the
/// frame changed" (winuser.h).
const GWL_STYLE: i32 = -16;
const WS_MAXIMIZEBOX: i32 = 0x0001_0000;
const WS_MINIMIZEBOX: i32 = 0x0002_0000;
const WS_THICKFRAME: i32 = 0x0004_0000;
const WS_SYSMENU: i32 = 0x0008_0000;
const SWP_NOSIZE: u32 = 0x0001;
const SWP_NOMOVE: u32 = 0x0002;
const SWP_NOZORDER: u32 = 0x0004;
const SWP_NOACTIVATE: u32 = 0x0010;
const SWP_FRAMECHANGED: u32 = 0x0020;

pub(crate) fn square_corners() {
    // SAFETY: both calls read thread-local state and nothing else.
    // The callback is a real `extern "system"` function of the shape
    // the enumeration expects, and the parameter it carries is unused.
    unsafe {
        EnumThreadWindows(GetCurrentThreadId(), square_one, 0);
    }
}

/// `SC_MAXIMIZE` / `SC_RESTORE` / `SC_MINIMIZE` (winuser.h), the three
/// the band's double-click, its buttons and the window menu send.
const SC_MAXIMIZE: usize = 0xF030;
const SC_RESTORE: usize = 0xF120;
const SC_MINIMIZE: usize = 0xF020;

pub(crate) fn set_maximized(maximized: bool) {
    post_command(if maximized { SC_MAXIMIZE } else { SC_RESTORE });
}

pub(crate) fn minimize() {
    post_command(SC_MINIMIZE);
}

/// Hands one system command to every top-level window the thread owns.
fn post_command(command: usize) {
    COMMAND.set(command);
    // SAFETY: as in `square_corners` — the same walk, and the callback
    // only posts a message to the window it is handed.
    unsafe {
        EnumThreadWindows(GetCurrentThreadId(), command_one, 0);
    }
}

thread_local! {
    /// Which command the walk is carrying.
    static COMMAND: Cell<usize> = const { Cell::new(0) };
}

/// Runs for every top-level window the thread owns. Posted rather
/// than sent: this arrives from a QML slot, and the platform's own
/// handling of it wants a turn of the message loop rather than a
/// call from inside one.
extern "system" fn command_one(window: *mut c_void, _param: isize) -> i32 {
    // SAFETY: `window` is live for the callback and both calls only
    // read it or post to it.
    unsafe {
        if IsWindowVisible(window) == 0 {
            return 1;
        }
        PostMessageW(window, WM_SYSCOMMAND, COMMAND.get(), 0);
    }
    1
}

/// Fits every windowed top-level window into its monitor's work
/// area, and says whether any of them moved.
pub(crate) fn fit_to_work_area(screen: &str) -> bool {
    MOVED.set(false);
    HOME.set(work_area_of(screen));
    // SAFETY: as in `square_corners` — the same walk, and the callback
    // only reads the window it is handed and repositions it.
    unsafe {
        EnumThreadWindows(GetCurrentThreadId(), fit_one, 0);
    }
    HOME.set(None);
    MOVED.get()
}

/// The work area of the display device `screen` names (`\\.\DISPLAY2`,
/// which is what Qt calls a screen on Windows), or `None` where nothing
/// answers to it — a first run with no saved place, and a monitor
/// unplugged since the run that wrote one.
///
/// **A name rather than a point.** Qt's coordinates are its own: with
/// two monitors at different scale factors the number a window reports
/// is not the number Windows would take, so a point handed across would
/// pick the wrong monitor exactly where the mixed-DPI desktop needs it
/// most. The device name is the one spelling both sides already agree
/// on (`QScreen::name` is `DISPLAY_DEVICE.DeviceName`).
fn work_area_of(screen: &str) -> Option<Rect> {
    if screen.is_empty() {
        return None;
    }
    WANTED.with(|wanted| wanted.replace(screen.encode_utf16().collect()));
    FOUND.set(None);
    // SAFETY: the callback is a real `extern "system"` function of the
    // shape `EnumDisplayMonitors` expects; it only fills a local of the
    // documented shape and writes thread-local state. A null device
    // context and a null clip rectangle are the documented way to walk
    // every display.
    unsafe {
        EnumDisplayMonitors(std::ptr::null_mut(), std::ptr::null(), named_one, 0);
    }
    FOUND.get()
}

/// Runs for every display until the wanted one answers.
extern "system" fn named_one(
    monitor: *mut c_void,
    _dc: *mut c_void,
    _rect: *const Rect,
    _param: isize,
) -> i32 {
    let mut info = MonitorInfoEx {
        info: MonitorInfo {
            size: size_of::<MonitorInfoEx>() as u32,
            ..MonitorInfo::default()
        },
        device: [0; MONITOR_NAME_LEN],
    };
    // SAFETY: `monitor` is live for the callback, and the call fills a
    // local whose `size` field says how much of it there is.
    let known = unsafe { GetMonitorInfoW(monitor, (&raw mut info).cast()) != 0 };
    if !known {
        return 1;
    }
    let name: Vec<u16> = info
        .device
        .iter()
        .take_while(|c| **c != 0)
        .copied()
        .collect();
    if WANTED.with(|wanted| *wanted.borrow() != name) {
        return 1;
    }
    FOUND.set(Some(info.info.work));
    0
}

thread_local! {
    /// Whether the fit had anything to do, for the caller to pass on:
    /// a window that was just moved is not one to measure the frame
    /// slop against (`WindowShape.settleTimer`).
    static MOVED: Cell<bool> = const { Cell::new(false) };
    /// The work area the walk is fitting to, or `None` to take each
    /// window's own nearest monitor.
    static HOME: Cell<Option<Rect>> = const { Cell::new(None) };
    /// The display device name [`work_area_of`] is looking for, and
    /// what it found.
    static WANTED: std::cell::RefCell<Vec<u16>> = const { std::cell::RefCell::new(Vec::new()) };
    static FOUND: Cell<Option<Rect>> = const { Cell::new(None) };
}

/// Runs for every top-level window the thread owns. A maximised one
/// is left alone: where it sits is the platform's arrangement, not a
/// remembered shape.
extern "system" fn fit_one(window: *mut c_void, _param: isize) -> i32 {
    // SAFETY: `window` is live for the callback; both calls only read.
    let skip = unsafe { IsWindowVisible(window) == 0 || IsZoomed(window) != 0 };
    if skip {
        return 1;
    }
    let mut rect = Rect::default();
    let mut info = MonitorInfo {
        size: size_of::<MonitorInfo>() as u32,
        ..MonitorInfo::default()
    };
    // SAFETY: as above; both calls fill locals of the documented shape.
    let known = unsafe {
        GetWindowRect(window, &mut rect);
        let monitor = MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST);
        GetMonitorInfoW(monitor, (&raw mut info).cast()) != 0
    };
    // The monitor the saved place named, where it named one that is
    // still here. Otherwise the window's own nearest, which is what a
    // first run and an unplugged monitor both want: somewhere on the
    // desktop rather than off it.
    let work = match HOME.get() {
        Some(work) => work,
        None if known => info.work,
        None => return 1,
    };
    let width = (rect.right - rect.left).min(work.right - work.left);
    let height = (rect.bottom - rect.top).min(work.bottom - work.top);
    let x = rect.left.min(work.right - width).max(work.left);
    let y = rect.top.min(work.bottom - height).max(work.top);
    if (x, y, width, height)
        == (
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
        )
    {
        return 1;
    }
    tracing::debug!(
        from = format!(
            "{},{} {}x{}",
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top
        ),
        to = format!("{x},{y} {width}x{height}"),
        "the remembered window did not fit its screen"
    );
    MOVED.set(true);
    // SAFETY: as above. Nothing but the geometry changes — the window
    // keeps its place in the stack and does not take focus.
    unsafe {
        SetWindowPos(
            window,
            std::ptr::null_mut(),
            x,
            y,
            width,
            height,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
    1
}

pub(crate) fn keep_system_gestures() {
    // SAFETY: as in `square_corners` — the same walk, and the callback
    // only reads and writes the window it is handed.
    unsafe {
        EnumThreadWindows(GetCurrentThreadId(), allow_one, 0);
    }
}

/// Runs for every top-level window the thread owns. Windows that
/// already carry the bits are left alone.
extern "system" fn allow_one(window: *mut c_void, _param: isize) -> i32 {
    // Not `WS_CAPTION`, though the window menu's Move and Size want
    // it: with the non-client area still there, saying the window has
    // a caption is saying the platform may draw one over the band.
    let wanted = WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_SYSMENU | WS_THICKFRAME;
    // SAFETY: `window` is live for the length of this callback, and
    // both calls take and return a plain integer.
    let style = unsafe { GetWindowLongW(window, GWL_STYLE) };
    if style == 0 || style & wanted == wanted {
        return 1;
    }
    // SAFETY: as above. The reposition moves and resizes nothing; it
    // is how Windows is told to read the style again.
    unsafe {
        SetWindowLongW(window, GWL_STYLE, style | wanted);
        SetWindowPos(
            window,
            std::ptr::null_mut(),
            0,
            0,
            0,
            0,
            SWP_NOSIZE | SWP_NOMOVE | SWP_NOZORDER | SWP_FRAMECHANGED,
        );
    }
    1
}

/// Runs for every top-level window the thread owns — Qt keeps more of
/// them than the one people look at, and the ones that have no frame
/// to square just report that they refused.
extern "system" fn square_one(window: *mut c_void, _param: isize) -> i32 {
    let wanted = DO_NOT_ROUND;
    // SAFETY: `window` is a live handle for the length of this
    // callback — that is what the enumeration hands out — and `wanted`
    // outlives the call, which copies the four bytes it points at.
    let hr = unsafe {
        DwmSetWindowAttribute(
            window,
            CORNER_PREFERENCE,
            std::ptr::from_ref(&wanted).cast(),
            size_of::<u32>() as u32,
        )
    };
    if hr != 0 {
        tracing::debug!(hresult = hr, "a window kept its rounded corners");
    }
    1
}
