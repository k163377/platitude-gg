//! The signatures, the `#[repr(C)]` mirrors and the message numbers
//! the three sides below share. Every `extern` block the module needs
//! is here, and the three reach them through `use super::*`.
#![expect(
    unsafe_code,
    reason = "neither the DWM corner attribute nor WM_SETICON has a safe \
              binding, and a handful of signatures do not earn a dependency"
)]

use std::cell::Cell;
use std::ffi::c_void;

use super::api::CAPTION_RUNS;

mod frame;
mod icon;
mod window;

pub(super) use frame::{set_caption_strips, take_frame_hit_test};
pub(super) use icon::set_icon;
pub(super) use window::{
    fit_to_work_area, keep_system_gestures, minimize, set_maximized, square_corners,
};

/// `RECT` (windef.h).
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

/// `MINMAXINFO` (winuser.h). Only the two the maximised placement is
/// made of are read here; the rest is filled in by whoever ran before
/// us and passed straight back.
#[repr(C)]
#[derive(Clone, Copy)]
struct MinMaxInfo {
    reserved: Point,
    max_size: Point,
    max_position: Point,
    min_track: Point,
    max_track: Point,
}
const WM_GETMINMAXINFO: u32 = 0x0024;

/// `MONITORINFO` (winuser.h), and the flag that asks for the monitor
/// a window is most on rather than none at all.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct MonitorInfo {
    size: u32,
    monitor: Rect,
    work: Rect,
    flags: u32,
}
const MONITOR_DEFAULTTONEAREST: u32 = 2;

/// `CCHDEVICENAME` (wingdi.h): how long a display device's name is.
const MONITOR_NAME_LEN: usize = 32;

/// `MONITORINFOEXW` (winuser.h) — `MONITORINFO` with the device name
/// after it. Filled by the same call, which reads `size` to know which
/// of the two it is being handed.
#[repr(C)]
#[derive(Clone, Copy)]
struct MonitorInfoEx {
    info: MonitorInfo,
    device: [u16; MONITOR_NAME_LEN],
}

/// `MONITORENUMPROC` (winuser.h). Returning zero stops the walk early.
type MonitorEnumProc = extern "system" fn(*mut c_void, *mut c_void, *const Rect, isize) -> i32;

/// `WM_SYSCOMMAND` (winuser.h).
const WM_SYSCOMMAND: u32 = 0x0112;

/// `WNDENUMPROC` (winuser.h). Returning zero stops the walk early.
type EnumProc = extern "system" fn(*mut c_void, isize) -> i32;

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

/// `SUBCLASSPROC` (commctrl.h).
type SubclassProc = extern "system" fn(*mut c_void, u32, usize, isize, usize, usize) -> isize;

// SAFETY: transcribed from commctrl.h; the pair is the documented way
// to sit in front of a window's procedure without owning it.
#[link(name = "comctl32")]
unsafe extern "system" {
    fn SetWindowSubclass(window: *mut c_void, proc: SubclassProc, id: usize, data: usize) -> i32;
    fn DefSubclassProc(window: *mut c_void, message: u32, wparam: usize, lparam: isize) -> isize;
}

// SAFETY: as the other blocks — both only read, into a local of the
// shape they document.
#[link(name = "user32")]
unsafe extern "system" {
    fn MonitorFromWindow(window: *mut c_void, flags: u32) -> *mut c_void;
    // Takes the wider `MONITORINFOEXW` too — which of the two it is
    // being handed is what the struct's own `size` field says, so the
    // pointer is cast at each call site.
    fn GetMonitorInfoW(monitor: *mut c_void, info: *mut MonitorInfo) -> i32;
    fn EnumDisplayMonitors(
        dc: *mut c_void,
        clip: *const Rect,
        callback: MonitorEnumProc,
        param: isize,
    ) -> i32;
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
