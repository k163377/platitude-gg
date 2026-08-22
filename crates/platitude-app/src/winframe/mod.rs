//! What Windows has to be told about the window that Qt has no words
//! for: square corners, the icon, the styles behind Win+Arrow, the
//! maximise geometry of a frameless window, and the non-client hit test
//! the merged chrome answers itself.
//!
//! Windows 11 rounds every top-level window and leaves the corner pixels
//! transparent, so whatever sits behind the window shows through them.
//! And a window that carries no icon of its own is given the shell's
//! generic one. Qt exposes a switch for none of it, so this is the single
//! place the app speaks Win32 directly instead of taking a dependency
//! for a handful of signatures.
//!
//! On the other two platforms nothing here runs.

mod api;
#[cfg(windows)]
mod win32;

pub use api::{
    fit_to_work_area, keep_system_gestures, minimize, set_caption_strip, set_icon, set_maximized,
    square_corners, take_frame_hit_test,
};
