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

/// `SC_MAXIMIZE` / `SC_RESTORE` (winuser.h), the two the band's
/// double-click and the window menu send.
const SC_MAXIMIZE: usize = 0xF030;
const SC_RESTORE: usize = 0xF120;

pub(crate) fn set_maximized(maximized: bool) {
    WANT_MAXIMIZED.set(maximized);
    // SAFETY: as in `square_corners` — the same walk, and the callback
    // only posts a message to the window it is handed.
    unsafe {
        EnumThreadWindows(GetCurrentThreadId(), command_one, 0);
    }
}

thread_local! {
    /// Which of the two commands the walk is carrying.
    static WANT_MAXIMIZED: Cell<bool> = const { Cell::new(false) };
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
        let command = if WANT_MAXIMIZED.get() {
            SC_MAXIMIZE
        } else {
            SC_RESTORE
        };
        PostMessageW(window, WM_SYSCOMMAND, command, 0);
    }
    1
}

/// Fits every windowed top-level window into its monitor's work
/// area, and says whether any of them moved.
pub(crate) fn fit_to_work_area() -> bool {
    MOVED.set(false);
    // SAFETY: as in `square_corners` — the same walk, and the callback
    // only reads the window it is handed and repositions it.
    unsafe {
        EnumThreadWindows(GetCurrentThreadId(), fit_one, 0);
    }
    MOVED.get()
}

thread_local! {
    /// Whether the fit had anything to do, for the caller to pass on:
    /// a window that was just moved is not one to measure the frame
    /// slop against (`WindowShape.settleTimer`).
    static MOVED: Cell<bool> = const { Cell::new(false) };
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
        GetMonitorInfoW(monitor, &mut info) != 0
    };
    if !known {
        return 1;
    }
    let work = info.work;
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
