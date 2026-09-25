//! The subclass that owns `WM_NCHITTEST`, and the two other frame
//! messages it takes with it.

use super::*;

/// `TrackPopupMenu` flags: hand the choice back, and take a
/// right-button press as a choice (winuser.h).
const TPM_RETURNCMD: u32 = 0x0100;
const TPM_RIGHTBUTTON: u32 = 0x0002;

/// Whether the window covers the whole work area — this window's
/// "maximised". Not `IsZoomed`: a frameless window Qt maximises is not
/// zoomed, and the resize edges only need to know whether there is
/// anywhere left to drag an edge to.
fn fills_work_area(window: *mut c_void) -> bool {
    let mut rect = Rect::default();
    let mut info = MonitorInfo {
        size: size_of::<MonitorInfo>() as u32,
        ..MonitorInfo::default()
    };
    // SAFETY: `window` is live for this call; both fill locals of the
    // shape they document.
    let known = unsafe {
        GetWindowRect(window, &mut rect);
        let monitor = MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST);
        GetMonitorInfoW(monitor, &mut info) != 0
    };
    let work = info.work;
    known
        && rect.left <= work.left
        && rect.top <= work.top
        && rect.right >= work.right
        && rect.bottom >= work.bottom
}

// ---- the frame's answers, owned (see `take_frame_hit_test`) --------

const WM_NCHITTEST: u32 = 0x0084;
const WM_NCRBUTTONUP: u32 = 0x00A5;
/// The hit-test answers this window hands out (winuser.h).
const HTCLIENT: isize = 1;
const HTCAPTION: isize = 2;
const HTLEFT: isize = 10;
const HTRIGHT: isize = 11;
const HTTOP: isize = 12;
const HTTOPLEFT: isize = 13;
const HTTOPRIGHT: isize = 14;
const HTBOTTOM: isize = 15;
const HTBOTTOMLEFT: isize = 16;
const HTBOTTOMRIGHT: isize = 17;
/// `SM_CXSIZEFRAME` + `SM_CXPADDEDBORDER` (winuser.h) is the invisible
/// resize border's width; the first alone is the pre-Vista number.
const SM_CXSIZEFRAME: i32 = 32;
const SM_CXPADDEDBORDER: i32 = 92;
const FRAME_SUBCLASS_ID: usize = 7;

thread_local! {
    /// The strips as `set_caption_strips` reports them. Scene x0 is
    /// client x0, so only the DPI scale is owed, taken fresh per hit test.
    static STRIPS: Cell<([(f64, f64); CAPTION_RUNS], f64)> =
        const { Cell::new(([(0.0, 0.0); CAPTION_RUNS], 0.0)) };
}

pub(crate) fn set_caption_strips(runs: [(f64, f64); CAPTION_RUNS], bottom: f64) {
    STRIPS.set((runs, bottom));
}

pub(crate) fn take_frame_hit_test() {
    // SAFETY: as in `square_corners` — the same walk.
    unsafe {
        EnumThreadWindows(GetCurrentThreadId(), claim_one, 0);
    }
}

/// Only the windows that are up: Qt keeps hidden helper windows on
/// this thread, and those never meet a pointer.
extern "system" fn claim_one(window: *mut c_void, _param: isize) -> i32 {
    // SAFETY: `window` is live for the callback, and the proc is a real
    // `extern "system"` function of the shape the subclass expects.
    unsafe {
        if IsWindowVisible(window) != 0 {
            SetWindowSubclass(window, frame_proc, FRAME_SUBCLASS_ID, 0);
        }
    }
    1
}

extern "system" fn frame_proc(
    window: *mut c_void,
    message: u32,
    wparam: usize,
    lparam: isize,
    _id: usize,
    _data: usize,
) -> isize {
    if message == WM_NCHITTEST {
        return hit_test(window, lparam);
    }
    if message == WM_NCRBUTTONUP && wparam as isize == HTCAPTION {
        let (x, y) = screen_point(lparam);
        open_system_menu(window, x, y);
        return 0;
    }
    // Windows would maximise onto the monitor's rectangle inflated by the
    // resize border, spilling onto the neighbouring monitor;
    // `clamp_maximized` pins it to the work area. Passed on first, so
    // only the two fields this is about change.
    if message == WM_GETMINMAXINFO {
        // SAFETY: passing the message on is what a subclass does.
        let passed = unsafe { DefSubclassProc(window, message, wparam, lparam) };
        clamp_maximized(window, lparam);
        return passed;
    }
    // SAFETY: passing the message on is what a subclass does.
    unsafe { DefSubclassProc(window, message, wparam, lparam) }
}

/// Writes the work area into the `MINMAXINFO` a maximise is about to
/// be made from; the origin is in the monitor's own coordinates.
fn clamp_maximized(window: *mut c_void, lparam: isize) {
    let mut info = MonitorInfo {
        size: size_of::<MonitorInfo>() as u32,
        ..MonitorInfo::default()
    };
    // SAFETY: `window` is the live handle the message arrived on, and
    // the call fills a local of the shape it documents.
    let known = unsafe {
        let monitor = MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST);
        GetMonitorInfoW(monitor, &mut info) != 0
    };
    if !known {
        return;
    }
    let (work, screen) = (info.work, info.monitor);
    // SAFETY: `lparam` on this message is a pointer to a `MINMAXINFO`
    // owned by the caller, which is ours to fill in for the length of
    // the call — that is what the message is for.
    let wanted = unsafe { &mut *(lparam as *mut MinMaxInfo) };
    wanted.max_size = Point {
        x: work.right - work.left,
        y: work.bottom - work.top,
    };
    wanted.max_position = Point {
        x: work.left - screen.left,
        y: work.top - screen.top,
    };
}

/// Both halves of an `lParam` that carries a screen point — signed,
/// because a second monitor to the left is negative territory.
fn screen_point(lparam: isize) -> (i32, i32) {
    let x = (lparam & 0xFFFF) as u16 as i16 as i32;
    let y = ((lparam >> 16) & 0xFFFF) as u16 as i16 as i32;
    (x, y)
}

/// The borders mirror what Qt would have answered: gone while
/// maximised, at the window's own DPI otherwise.
fn hit_test(window: *mut c_void, lparam: isize) -> isize {
    let (x, y) = screen_point(lparam);
    let mut rect = Rect::default();
    // SAFETY: `window` is the live handle this message arrived on,
    // and `rect` is a local the call fills in.
    let (maximized, dpi) = unsafe {
        GetWindowRect(window, &mut rect);
        (fills_work_area(window), GetDpiForWindow(window))
    };
    let dpi = if dpi > 0 { dpi } else { 96 };
    if !maximized {
        // SAFETY: reads two system-wide integers.
        let border = unsafe {
            GetSystemMetricsForDpi(SM_CXSIZEFRAME, dpi)
                + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi)
        };
        let left = x >= rect.left && x < rect.left + border;
        let right = x > rect.right - border && x <= rect.right;
        let top = y >= rect.top && y < rect.top + border;
        let bottom = y > rect.bottom - border && y <= rect.bottom;
        if left {
            return if top {
                HTTOPLEFT
            } else if bottom {
                HTBOTTOMLEFT
            } else {
                HTLEFT
            };
        }
        if right {
            return if top {
                HTTOPRIGHT
            } else if bottom {
                HTBOTTOMRIGHT
            } else {
                HTRIGHT
            };
        }
        if top {
            return HTTOP;
        }
        if bottom {
            return HTBOTTOM;
        }
    }
    let mut point = Point { x, y };
    // SAFETY: as above.
    unsafe {
        ScreenToClient(window, &mut point);
    }
    let scale = f64::from(dpi) / 96.0;
    let (runs, strip_bottom) = STRIPS.get();
    let (x, y) = (f64::from(point.x), f64::from(point.y));
    let in_run = |&(x0, x1): &(f64, f64)| x1 > x0 && x >= x0 * scale && x < x1 * scale;
    if y < strip_bottom * scale && runs.iter().any(in_run) {
        return HTCAPTION;
    }
    HTCLIENT
}

/// The window menu at the pointer, tracked here: the frame has no
/// `WS_CAPTION` for `DefWindowProc` to hang it on.
fn open_system_menu(window: *mut c_void, x: i32, y: i32) {
    // SAFETY: each call takes plain integers or a handle Windows just
    // handed back, and none of them takes ownership of anything.
    unsafe {
        let menu = GetSystemMenu(window, 0);
        if menu.is_null() {
            return;
        }
        // The menu closes when its window loses the foreground, which
        // it can only do once it has it.
        SetForegroundWindow(window);
        let chosen = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            x,
            y,
            0,
            window,
            std::ptr::null(),
        );
        if chosen != 0 {
            PostMessageW(window, WM_SYSCOMMAND, chosen as usize, 0);
        }
    }
}
