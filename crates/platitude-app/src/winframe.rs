//! What Windows has to be told about the window: that its corners stay
//! square, and which icon it wears.
//!
//! Windows 11 rounds every top-level window and leaves the corner pixels
//! transparent, so whatever sits behind the window shows through them.
//! And a window that carries no icon of its own is given the shell's
//! generic one. Qt exposes a switch for neither, so this is the single
//! place the app speaks Win32 directly instead of taking a dependency
//! for a handful of signatures.
//!
//! On the other two platforms nothing here runs.

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
/// them. Those three are not about buttons: without them the platform
/// refuses Win+Arrow, the taskbar's own menu, and clicking the taskbar
/// button to minimise. Putting them back does not bring the drawn buttons
/// back with them (measured: style 0x96040000 → 0x960F0000, nothing
/// appears), because the caption they would be drawn in is no longer part
/// of this window's frame.
pub fn keep_system_gestures() {
    #[cfg(windows)]
    win32::keep_system_gestures();
}

#[cfg(windows)]
mod win32 {
    #![expect(
        unsafe_code,
        reason = "neither the DWM corner attribute nor WM_SETICON has a safe \
                  binding, and a handful of signatures do not earn a dependency"
    )]

    use std::cell::Cell;
    use std::ffi::c_void;

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
    const WS_SYSMENU: i32 = 0x0008_0000;
    const SWP_NOSIZE: u32 = 0x0001;
    const SWP_NOMOVE: u32 = 0x0002;
    const SWP_NOZORDER: u32 = 0x0004;
    const SWP_FRAMECHANGED: u32 = 0x0020;

    /// `WM_SETICON` and the two `wParam` values it takes (winuser.h).
    const WM_SETICON: u32 = 0x0080;
    const ICON_SMALL: usize = 0;
    const ICON_BIG: usize = 1;

    /// `SM_CXSMICON` / `SM_CXICON` (winuser.h): the two sizes this display
    /// asks for. Reading them is what keeps the icon sharp on a screen
    /// that is not at 100%.
    const SM_CXSMICON: i32 = 49;
    const SM_CXICON: i32 = 11;

    /// The *resource* version `CreateIconFromResourceEx` wants, which has
    /// nothing to do with the app's own (winuser.h).
    const ICON_RESOURCE_VERSION: u32 = 0x0003_0000;

    /// The icon, rendered ahead of time from `assets/platitude*.svg` at
    /// the sizes Windows asks for (デザイン規約 §アプリアイコン — the 16
    /// and 20 come from the small cut, the rest from the master). PNG is
    /// a format `CreateIconFromResourceEx` reads directly, so there is no
    /// `.ico` container in the way and nothing to decode here.
    const RENDERED: &[(i32, &[u8])] = &[
        (16, include_bytes!("../assets/icon-16.png")),
        (20, include_bytes!("../assets/icon-20.png")),
        (24, include_bytes!("../assets/icon-24.png")),
        (32, include_bytes!("../assets/icon-32.png")),
        (48, include_bytes!("../assets/icon-48.png")),
        (64, include_bytes!("../assets/icon-64.png")),
    ];

    /// `WNDENUMPROC` (winuser.h). Returning zero stops the walk early.
    type EnumProc = extern "system" fn(*mut c_void, isize) -> i32;

    thread_local! {
        /// The pair the icon walk hands to each window, so the two
        /// handles are built once rather than once per window. Nothing
        /// destroys them: the windows read them for as long as they are
        /// up, and the process exiting is what releases them.
        static WEARING: Cell<(*mut c_void, *mut c_void)> =
            const { Cell::new((std::ptr::null_mut(), std::ptr::null_mut())) };
    }

    // SAFETY: every signature below is transcribed from the Win32 headers.
    // Each argument is a plain integer or a pointer the callee only reads,
    // and none of them takes or hands back ownership of memory.
    #[link(name = "dwmapi")]
    unsafe extern "system" {
        fn DwmSetWindowAttribute(
            window: *mut c_void,
            attribute: u32,
            value: *const c_void,
            size: u32,
        ) -> i32;
    }

    // SAFETY: as above.
    #[link(name = "user32")]
    unsafe extern "system" {
        fn EnumThreadWindows(thread: u32, callback: EnumProc, param: isize) -> i32;
        fn GetSystemMetrics(index: i32) -> i32;
        fn GetWindowLongW(window: *mut c_void, index: i32) -> i32;
        fn SetWindowLongW(window: *mut c_void, index: i32, value: i32) -> i32;
        fn SetWindowPos(
            window: *mut c_void,
            after: *mut c_void,
            x: i32,
            y: i32,
            cx: i32,
            cy: i32,
            flags: u32,
        ) -> i32;
        fn CreateIconFromResourceEx(
            bits: *const u8,
            size: u32,
            icon: i32,
            version: u32,
            cx: i32,
            cy: i32,
            flags: u32,
        ) -> *mut c_void;
        fn SendMessageW(window: *mut c_void, message: u32, wparam: usize, lparam: isize) -> isize;
    }

    // SAFETY: as above.
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentThreadId() -> u32;
    }

    pub(super) fn square_corners() {
        // SAFETY: both calls read thread-local state and nothing else.
        // The callback is a real `extern "system"` function of the shape
        // the enumeration expects, and the parameter it carries is unused.
        unsafe {
            EnumThreadWindows(GetCurrentThreadId(), square_one, 0);
        }
    }

    pub(super) fn set_icon() {
        let small = rendered_at(metric(SM_CXSMICON, 16));
        let big = rendered_at(metric(SM_CXICON, 32));
        if small.is_null() && big.is_null() {
            tracing::debug!("no icon could be built; the shell default stays");
            return;
        }
        WEARING.set((small, big));
        // SAFETY: as in `square_corners` — the walk is the same one, and
        // the handles it reads are already in place.
        unsafe {
            EnumThreadWindows(GetCurrentThreadId(), wear_one, 0);
        }
    }

    pub(super) fn keep_system_gestures() {
        // SAFETY: as in `square_corners` — the same walk, and the callback
        // only reads and writes the window it is handed.
        unsafe {
            EnumThreadWindows(GetCurrentThreadId(), allow_one, 0);
        }
    }

    /// Runs for every top-level window the thread owns. Windows that
    /// already carry the bits are left alone, so the frame is not told to
    /// change for nothing.
    extern "system" fn allow_one(window: *mut c_void, _param: isize) -> i32 {
        let wanted = WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_SYSMENU;
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

    /// A system metric, or `fallback` if Windows declines to answer (it
    /// returns zero rather than an error).
    fn metric(index: i32, fallback: i32) -> i32 {
        // SAFETY: reads one system-wide integer.
        let asked = unsafe { GetSystemMetrics(index) };
        if asked > 0 { asked } else { fallback }
    }

    /// Builds a handle from whichever rendering is closest to `size`.
    /// Windows scales from there when nothing matches exactly, which is
    /// why the odd sizes a 125% display asks for still land sharp.
    fn rendered_at(size: i32) -> *mut c_void {
        let Some((_, png)) = RENDERED.iter().min_by_key(|(at, _)| (at - size).abs()) else {
            return std::ptr::null_mut();
        };
        // SAFETY: the PNG is a `'static` slice in the binary, so it
        // outlives the call that reads it, and its length is its own.
        unsafe {
            CreateIconFromResourceEx(
                png.as_ptr(),
                png.len() as u32,
                1,
                ICON_RESOURCE_VERSION,
                size,
                size,
                0,
            )
        }
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
        // Keep walking: the window people look at is not always the first.
        1
    }

    /// The same walk, handing each window the icon. Windows takes the
    /// small one for the title bar and the big one for the taskbar
    /// button and Alt+Tab, so both are set.
    extern "system" fn wear_one(window: *mut c_void, _param: isize) -> i32 {
        let (small, big) = WEARING.get();
        // SAFETY: `window` is live for the length of the callback, and
        // both handles were built above and are never destroyed, so the
        // window can hold them for as long as it is up.
        unsafe {
            if !small.is_null() {
                SendMessageW(window, WM_SETICON, ICON_SMALL, small as isize);
            }
            if !big.is_null() {
                SendMessageW(window, WM_SETICON, ICON_BIG, big as isize);
            }
        }
        1
    }
}
