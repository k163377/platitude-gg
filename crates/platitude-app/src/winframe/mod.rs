//! What Windows has to be told about the window that Qt has no words
//! for: square corners, the styles behind Win+Arrow, the maximise
//! geometry of a frameless window, and the non-client hit test the
//! merged chrome answers itself. The one place the app speaks Win32
//! directly, over signatures it declares itself; nothing here runs on
//! the other platforms.
//!
//! Not the icon: the exe's resource hands it to the window from its
//! creation (`assets/platitude.rc`).

mod api;
#[cfg(test)]
mod resource_tests;
#[cfg(windows)]
mod win32;

pub use api::{
    fit_to_work_area, keep_system_gestures, minimize, set_caption_strips, set_maximized,
    square_corners, take_frame_hit_test,
};
