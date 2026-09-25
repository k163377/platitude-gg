//! What Windows has to be told about the window that Qt has no words
//! for: square corners, the icon, the styles behind Win+Arrow, the
//! maximise geometry of a frameless window, and the non-client hit test
//! the merged chrome answers itself. The one place the app speaks Win32
//! directly, over signatures it declares itself; nothing here runs on
//! the other platforms.

mod api;
#[cfg(windows)]
mod win32;

pub use api::{
    fit_to_work_area, keep_system_gestures, minimize, set_caption_strips, set_icon, set_maximized,
    square_corners, take_frame_hit_test,
};
