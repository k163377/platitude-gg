//! The app's own icon, built from the renderings in the binary and
//! handed to every window on the thread.

use super::*;

/// `WM_SETICON` and the two `wParam` values it takes (winuser.h).
const WM_SETICON: u32 = 0x0080;
const ICON_SMALL: usize = 0;
const ICON_BIG: usize = 1;

/// `SM_CXSMICON` / `SM_CXICON` (winuser.h): the two sizes this display
/// asks for, which keeps the icon sharp on a scaled screen.
const SM_CXSMICON: i32 = 49;
const SM_CXICON: i32 = 11;

/// The resource version `CreateIconFromResourceEx` wants, not the app's
/// own (winuser.h).
const ICON_RESOURCE_VERSION: u32 = 0x0003_0000;

/// Rendered ahead of time from `assets/platitude*.svg`
/// (デザイン規約 §アプリアイコン). `CreateIconFromResourceEx` reads PNG
/// directly, so there is no `.ico` container and nothing to decode here.
const RENDERED: &[(i32, &[u8])] = &[
    (16, include_bytes!("../../../assets/icon-16.png")),
    (20, include_bytes!("../../../assets/icon-20.png")),
    (24, include_bytes!("../../../assets/icon-24.png")),
    (32, include_bytes!("../../../assets/icon-32.png")),
    (48, include_bytes!("../../../assets/icon-48.png")),
    (64, include_bytes!("../../../assets/icon-64.png")),
];

thread_local! {
    /// The pair the walk hands to each window, built once and never
    /// destroyed: the windows hold them while up, and exit releases them.
    static WEARING: Cell<(*mut c_void, *mut c_void)> =
        const { Cell::new((std::ptr::null_mut(), std::ptr::null_mut())) };
}

pub(crate) fn set_icon() {
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

/// A system metric, or `fallback` if Windows declines to answer
/// (it returns zero).
fn metric(index: i32, fallback: i32) -> i32 {
    // SAFETY: reads one system-wide integer.
    let asked = unsafe { GetSystemMetrics(index) };
    if asked > 0 { asked } else { fallback }
}

/// A handle from the rendering closest to `size`; Windows scales from
/// there when nothing matches exactly.
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

/// Windows takes the small icon for the title bar and the big one for
/// the taskbar and Alt+Tab, so both are set.
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
