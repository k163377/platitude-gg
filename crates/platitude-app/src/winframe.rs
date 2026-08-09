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

/// Paints the two things Windows 11 draws around the window: the hairline
/// border, and the strip of frame between it and the client area.
///
/// Left alone both are the system's, which on a light system is a white
/// line and a white strip beside it — and this window has no title bar to
/// explain either, so they read as stray edges around the band. Both are
/// `0xRRGGBB` and both come from the design tokens, because this is the
/// app's own edge and not a system one.
pub fn set_border_color(border: u32, frame: u32) {
    #[cfg(windows)]
    win32::set_border_color(border, frame);
    #[cfg(not(windows))]
    let _ = (border, frame);
}

// Taking the frame over — answering `WM_NCCALCSIZE` with "the client is
// the whole window" — does remove the one strip of system-coloured frame
// that no attribute reaches, and the resize edges can be answered from a
// subclass in its place (both measured, and both worked).
//
// It is not here because Qt cannot be told. The subclass sits in front of
// Qt's own window procedure, so Qt never sees the message and keeps the
// frame margins it cached at creation: the client grew and the scene did
// not, leaving the strip it gained unpainted (measured: frame and client
// both 1450x908 with the scene still drawing 1434x900, black down the
// right-hand edge and along the bottom). Nudging the size, asking for the
// frame to be recalculated and letting the message through first were all
// tried; none of them make Qt re-measure. What would is a way to set the
// window's custom margins, which the bridge does not expose
// (P3-確認事項 §ウィンドウ chrome).

/// Takes `WM_NCHITTEST` away from Qt for the windows that are up, and
/// answers it from the strip `set_caption_strip` describes.
///
/// This is not an optimisation, it is the bug fix. Qt 6.10's own answer
/// for an `ExpandedClientAreaHint` + `CustomizeWindowHint` window
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
/// point is client except the resize borders and the one strip the QML
/// side says is grab-run, and those get the platform's own caption
/// behaviour — drag, snap, double-click, and the window menu on
/// right-click — through the front door.
///
/// This does not cross the `WM_NCCALCSIZE` finding above: that message
/// feeds frame metrics Qt caches and must keep seeing, while this one
/// is a pure query answered fresh every time, with no Qt state behind
/// it.
pub fn take_frame_hit_test() {
    #[cfg(windows)]
    win32::take_frame_hit_test();
}

// The scene is never told this border's width, though a maximised frame
// is inflated by it on every side (`hit_test` measures the resize edges
// by it, in device pixels, and that is the only reader). The QML side has
// no use for it: those pixels are non-client, and the client comes back
// deflated to the work area exactly, so nothing the app paints is ever
// out there. The measurement, and the shifted window that came of not
// having taken it, are written up on `Main.mainUi`.

/// Where the band's empty run sits, in logical scene pixels: from `x0`
/// to `x1`, reaching down from the window's top edge to `bottom`. The
/// subclass turns it into device pixels itself, per hit test, so a DPI
/// change needs no new report.
pub fn set_caption_strip(x0: f64, x1: f64, bottom: f64) {
    #[cfg(windows)]
    win32::set_caption_strip(x0, x1, bottom);
    #[cfg(not(windows))]
    let _ = (x0, x1, bottom);
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
    /// `DWMWA_BORDER_COLOR` (dwmapi.h, Windows 11 22000+). Takes a
    /// `COLORREF`, which is `0x00BBGGRR` — the reverse of how a colour is
    /// written everywhere else in this tree.
    const BORDER_COLOR: u32 = 34;
    /// `DWMWA_CAPTION_COLOR` (dwmapi.h, Windows 11 22000+).
    ///
    /// Set alongside the border for the sake of anywhere it does apply.
    /// It is **not** what paints the one white pixel between our border
    /// and the client area: measured, that pixel is unmoved by this, by
    /// the border colour, and by extending the frame across the client.
    /// Taking it needs the non-client area removed in `WM_NCCALCSIZE`,
    /// which means subclassing the window — the same machinery Snap
    /// Layouts wants (P3-確認事項 §ウィンドウ chrome).
    const CAPTION_COLOR: u32 = 35;

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

    /// `RECT` (windef.h).
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    /// `TrackPopupMenu` flags: hand the choice back rather than posting it,
    /// and take a right-button press as a choice (winuser.h).
    const TPM_RETURNCMD: u32 = 0x0100;
    const TPM_RIGHTBUTTON: u32 = 0x0002;
    /// `WM_SYSCOMMAND` (winuser.h).
    const WM_SYSCOMMAND: u32 = 0x0112;

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

    // SAFETY: as above.
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetSystemMenu(window: *mut c_void, revert: i32) -> *mut c_void;
        fn TrackPopupMenu(
            menu: *mut c_void,
            flags: u32,
            x: i32,
            y: i32,
            reserved: i32,
            window: *mut c_void,
            rect: *const Rect,
        ) -> i32;
        fn PostMessageW(window: *mut c_void, message: u32, wparam: usize, lparam: isize) -> i32;
        fn SetForegroundWindow(window: *mut c_void) -> i32;
    }

    /// `POINT` (windef.h).
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Point {
        x: i32,
        y: i32,
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

    thread_local! {
        /// The two colours the frame walk is handing out, since the
        /// callback takes no argument of its own: the border's, and the
        /// strip of frame between it and the client area.
        static FRAME_COLORS: Cell<(u32, u32)> = const { Cell::new((0, 0)) };
    }

    /// `0x00BBGGRR` from `0xRRGGBB`.
    fn colorref(rgb: u32) -> u32 {
        ((rgb & 0xFF) << 16) | (rgb & 0xFF00) | ((rgb >> 16) & 0xFF)
    }

    pub(super) fn set_border_color(border: u32, frame: u32) {
        FRAME_COLORS.set((colorref(border), colorref(frame)));
        // SAFETY: as in `square_corners` — the same walk, and the callback
        // only writes an attribute on the window it is handed.
        unsafe {
            EnumThreadWindows(GetCurrentThreadId(), paint_one, 0);
        }
    }

    /// Runs for every top-level window the thread owns; the ones with no
    /// border to paint report that they refused.
    extern "system" fn paint_one(window: *mut c_void, _param: isize) -> i32 {
        let (border, frame) = FRAME_COLORS.get();
        for (attribute, wanted) in [(BORDER_COLOR, border), (CAPTION_COLOR, frame)] {
            // SAFETY: `window` is live for the length of this callback,
            // and `wanted` outlives the call, which copies the four bytes
            // it points at.
            let hr = unsafe {
                DwmSetWindowAttribute(
                    window,
                    attribute,
                    std::ptr::from_ref(&wanted).cast(),
                    size_of::<u32>() as u32,
                )
            };
            if hr != 0 {
                tracing::debug!(hresult = hr, attribute, "a window kept the system's colour");
            }
        }
        1
    }

    // ---- the frame's answers, owned (see `take_frame_hit_test`) --------

    const WM_NCHITTEST: u32 = 0x0084;
    const WM_NCRBUTTONUP: u32 = 0x00A5;
    /// The hit-test answers this window hands out (winuser.h). Client,
    /// caption, and the eight resize edges; nothing else exists here —
    /// no drawn system buttons, no icon box.
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
    /// `SM_CXSIZEFRAME` + `SM_CXPADDEDBORDER` (winuser.h) is how wide
    /// the invisible resize border actually is — the first alone is the
    /// pre-Vista number.
    const SM_CXSIZEFRAME: i32 = 32;
    const SM_CXPADDEDBORDER: i32 = 92;
    const FRAME_SUBCLASS_ID: usize = 7;

    /// `SUBCLASSPROC` (commctrl.h).
    type SubclassProc = extern "system" fn(*mut c_void, u32, usize, isize, usize, usize) -> isize;

    // SAFETY: transcribed from commctrl.h; the pair is the documented way
    // to sit in front of a window's procedure without owning it.
    #[link(name = "comctl32")]
    unsafe extern "system" {
        fn SetWindowSubclass(
            window: *mut c_void,
            proc: SubclassProc,
            id: usize,
            data: usize,
        ) -> i32;
        fn DefSubclassProc(
            window: *mut c_void,
            message: u32,
            wparam: usize,
            lparam: isize,
        ) -> isize;
    }

    // SAFETY: as the other user32 blocks — plain integers and pointers
    // the callee only reads or fills.
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetWindowRect(window: *mut c_void, rect: *mut Rect) -> i32;
        fn ScreenToClient(window: *mut c_void, point: *mut Point) -> i32;
        fn IsZoomed(window: *mut c_void) -> i32;
        fn IsWindowVisible(window: *mut c_void) -> i32;
        fn GetDpiForWindow(window: *mut c_void) -> u32;
        fn GetSystemMetricsForDpi(index: i32, dpi: u32) -> i32;
    }

    thread_local! {
        /// The grab-run strip, in logical scene pixels: left edge, right
        /// edge, bottom. Scene x0 is client x0, so no origin shift is
        /// owed — only the DPI scale, taken fresh per hit test.
        static STRIP: Cell<(f64, f64, f64)> = const { Cell::new((0.0, 0.0, 0.0)) };
    }

    pub(super) fn set_caption_strip(x0: f64, x1: f64, bottom: f64) {
        STRIP.set((x0, x1, bottom));
    }

    pub(super) fn take_frame_hit_test() {
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

    /// Answers `WM_NCHITTEST` without letting Qt see it (the point of
    /// the whole exercise — see `take_frame_hit_test`), opens the window
    /// menu on a right-click in the strip, and forwards everything else.
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
        // SAFETY: passing the message on is what a subclass does.
        unsafe { DefSubclassProc(window, message, wparam, lparam) }
    }

    /// Both halves of an `lParam` that carries a screen point — signed,
    /// because a second monitor to the left is negative territory.
    fn screen_point(lparam: isize) -> (i32, i32) {
        let x = (lparam & 0xFFFF) as u16 as i16 as i32;
        let y = ((lparam >> 16) & 0xFFFF) as u16 as i16 as i32;
        (x, y)
    }

    /// Resize borders first, then the strip, then client. The borders
    /// mirror what Qt would have answered: gone while maximised, and
    /// measured at the window's own DPI while not.
    fn hit_test(window: *mut c_void, lparam: isize) -> isize {
        let (x, y) = screen_point(lparam);
        let mut rect = Rect::default();
        // SAFETY: `window` is the live handle this message arrived on,
        // and `rect` is a local the call fills in.
        let (maximized, dpi) = unsafe {
            GetWindowRect(window, &mut rect);
            (IsZoomed(window) != 0, GetDpiForWindow(window))
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
        let (x0, x1, strip_bottom) = STRIP.get();
        let (x, y) = (f64::from(point.x), f64::from(point.y));
        if x1 > x0 && y < strip_bottom * scale && x >= x0 * scale && x < x1 * scale {
            return HTCAPTION;
        }
        HTCLIENT
    }

    /// The window menu — move, size, minimise, maximise, close — where
    /// the pointer is. What a title bar answers a right-click with, and
    /// the strip is one now. Not left to `DefWindowProc`, so showing it
    /// does not depend on the caption behaviour of a window that has no
    /// `WS_CAPTION`.
    fn open_system_menu(window: *mut c_void, x: i32, y: i32) {
        // SAFETY: each call takes plain integers or a handle Windows just
        // handed back, and none of them takes ownership of anything.
        unsafe {
            let menu = GetSystemMenu(window, 0);
            if menu.is_null() {
                return;
            }
            // The menu closes when the window it belongs to loses the
            // foreground, and a menu nobody can dismiss is worse than none.
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
        // Not `WS_CAPTION`, though the window menu's Move and Size want
        // it: with the non-client area still there, saying the window has
        // a caption is saying the platform may draw one over the band.
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
