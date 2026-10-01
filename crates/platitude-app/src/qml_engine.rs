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

/// [`load`] against a real engine, offscreen, in a copy of this binary
/// started for it (as `encode::wire_qml`): a root that does not load is
/// named, a later one that does is not, and the engine and the
/// application come down in `main`'s order after either.
#[cfg(test)]
mod tests {
    #![expect(
        clippy::print_stdout,
        reason = "the child says what it saw on stdout, where the parent reads it"
    )]

    use std::process::Command;

    use qtbridge::qtbridge_type_lib::{QGuiApplication, QQmlApplicationEngine};

    const BROKEN: &str = "import QtQml\nQtObject {\n    property int n: 1\n    NoSuchType {}\n}\n";
    const SOUND: &str = "import QtQml\nQtObject {}\n";

    #[test]
    #[ignore = "started by a_root_that_does_not_load_is_named_and_comes_down_in_order, offscreen"]
    fn engine_side() {
        if std::env::var_os("QML_ENGINE_CHILD").is_none() {
            return;
        }
        let dir = std::env::temp_dir().join(format!("pgg-qml-engine-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a directory for the QML");
        let broken = dir.join("Broken.qml");
        let sound = dir.join("Sound.qml");
        std::fs::write(&broken, BROKEN).expect("the broken QML");
        std::fs::write(&sound, SOUND).expect("the sound QML");
        let app = QGuiApplication::new();
        let mut engine = QQmlApplicationEngine::new();
        super::arm(engine.pin_mut());
        for file in [&broken, &sound] {
            let answer = super::load(engine.pin_mut(), &crate::urlpath::file_url(file));
            println!(
                "ENGINE\t{}",
                answer.map_or_else(|failed| failed.url, |()| "loaded".into())
            );
        }
        drop(engine);
        qtbridge::collect_garbage();
        drop(app);
        println!("ENGINE\tdown");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_root_that_does_not_load_is_named_and_comes_down_in_order() {
        let out = Command::new(std::env::current_exe().expect("this test binary"))
            .args([
                "qml_engine::tests::engine_side",
                "--exact",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("QML_ENGINE_CHILD", "1")
            .env("QT_QPA_PLATFORM", "offscreen")
            .output()
            .expect("the child starts");
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success(),
            "{text}\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let said: Vec<&str> = text
            .lines()
            .filter_map(|line| line.split_once("ENGINE\t"))
            .map(|(_, rest)| rest)
            .collect();
        assert_eq!(said.len(), 3, "{text}");
        assert!(
            said[0].starts_with("file:") && said[0].ends_with("/Broken.qml"),
            "{said:?}"
        );
        assert_eq!(said[1..], ["loaded", "down"], "{text}");
    }
}
