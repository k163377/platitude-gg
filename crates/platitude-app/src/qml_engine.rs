//! The QML engine `main` holds, beside the application: what qtbridge's
//! `QApp` would do with it, minus the one thing `QApp` cannot — answer when
//! the window's QML does not load (`QApp::load_qml_from_file` returns
//! nothing either way, and keeps its engine to itself).
//!
//! `main` holds the application and the engine as values (`QGuiApplication`,
//! `QQmlApplicationEngine`, which the bridge's type library re-exports from
//! cxx-qt-lib) and drops them in `QApp`'s order: the engine, a last
//! collection (`qtbridge::collect_garbage`), then the application.

use std::pin::Pin;
use std::sync::{Arc, Mutex};

use qtbridge::qtbridge_type_lib::QQmlApplicationEngine;

/// Arms the bridge's collection on the engine's garbage collection — what
/// `QApp` does to the engine it makes, and asks of anyone driving their own
/// (`qtbridge_runtime::registry::install_gc_sentinel`).
pub fn arm(engine: Pin<&mut QQmlApplicationEngine>) {
    qtbridge::qtbridge_runtime::registry::install_gc_sentinel(engine);
}

/// The window's QML did not load. Qt has written why — one line for the
/// failure, one per error — before this comes back.
#[derive(Debug)]
pub struct LoadFailed {
    /// The component Qt names (`objectCreationFailed`).
    pub url: String,
}

/// Loads the root file at `url`. A `qrc:` load completes inside the call,
/// so `objectCreationFailed` has been emitted by the time it returns if it
/// is going to be; the connection ends with the call.
pub fn load(mut engine: Pin<&mut QQmlApplicationEngine>, url: &str) -> Result<(), LoadFailed> {
    let failed: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let heard = Arc::clone(&failed);
    let _listening = engine.as_mut().on_object_creation_failed(move |_, url| {
        if let Ok(mut slot) = heard.lock() {
            *slot = Some(url.to_string());
        }
    });
    engine.load(&url.into());
    let said = match failed.lock() {
        Ok(slot) => slot.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    };
    match said {
        Some(url) => Err(LoadFailed { url }),
        None => Ok(()),
    }
}
