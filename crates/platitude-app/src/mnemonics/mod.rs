//! Qt's mnemonics, off for the whole run: an `&` in a control's text is
//! never a key.
//!
//! On, Qt makes the letter after an `&` in a button's `text` that button's
//! key: Alt with the letter presses it from anywhere in its window
//! (`QQuickAbstractButton::buttonChange` takes the key as the text is set),
//! and in an open menu the letter alone presses the row it is in
//! (`QQuickMenu::keyPressEvent`, through `click()`, which asks `isEnabled`
//! alone — a greyed row that stays enabled runs as well). The texts carry
//! the names the user chose: a branch `r&d` makes D press every row that
//! names it. No text of the product marks a key of its own with `&`, so
//! none is given up. Off, Alt does not close an open menu either (Qt closes
//! one on Alt only while mnemonics are on); Escape does. macOS has them off
//! from the start.
//!
//! What a screen reader is told is apart: a control with no
//! `Accessible.name` of its own loses its text's first `&` there, whatever
//! the switch (`QQuickAbstractButton::buttonChange`, `qt_accStripAmp`) —
//! `AppMenuItem` states its name.
//!
//! Qt Gui exports the switch and documents it beside `QKeySequence`; its
//! header declares it for the docs alone, so `mnemonics.h` does.

#![expect(
    unsafe_code,
    reason = "the CXX bridge to Qt Gui's mnemonic switch is unsafe code of its own"
)]

use qtbridge::qtbridge_type_lib::QGuiApplication;

#[cfg(test)]
mod tests;

/// The application as the product makes it, mnemonics off before any QML
/// loads — a button takes its key as its text is set, so one whose text
/// came first would keep it. `main` makes it here and so does the test's
/// child, which holds the switch where the product throws it.
pub(crate) fn application() -> cxx::UniquePtr<QGuiApplication> {
    let app = QGuiApplication::new();
    ffi::turn_off();
    app
}

#[cxx_qt::bridge]
mod ffi {
    #[namespace = "platitude::mnemonics"]
    unsafe extern "C++" {
        include!("platitude-app/src/mnemonics/mnemonics.h");
        #[rust_name = "turn_off"]
        fn turnOff();
    }
}
