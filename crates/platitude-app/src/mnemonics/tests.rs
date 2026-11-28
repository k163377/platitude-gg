//! An `&` in a text, pressed through Qt's own keys: a button's Alt with
//! the letter and an open menu's letter alone, sent by QtTest's events into
//! a Qt application of their own — made as `main` makes one
//! ([`super::application`]), and a bare one, which is the probe's witness
//! that it can see a press.
//!
//! A Qt application cannot share a process with the other tests' threads,
//! so each runs offscreen in a copy of this binary started for it
//! ([`qt_side`]), as the wire's QML tests do.

use std::path::Path;
use std::process::{Command, Stdio};

use qtbridge::qtbridge_type_lib::{QGuiApplication, QQmlApplicationEngine};

/// The parent's mark on a child: the probe's file.
const PROBE_FILE: &str = "MNEMONICS_PROBE_FILE";
/// Set when the child makes its application as `main` does.
const AS_SHIPPED: &str = "MNEMONICS_AS_SHIPPED";

/// A button and a menu row whose texts name a branch `r&d`, each counting
/// its presses. The button first, with no menu up to stand between it and
/// its window's keys; then D in the open menu. The window has to be the
/// focus window before a window's key reaches a button. The timer is the
/// ceiling of a child that never gets there, which says so.
const PROBE: &str = r#"
import QtQuick
import QtQuick.Controls.Basic
import QtTest

Window {
    id: probe
    width: 320
    height: 240
    visible: true

    property int buttonPresses: 0
    property int rowPresses: 0
    property bool started: false

    function start() {
        if (probe.started || !probe.active)
            return
        probe.started = true
        keys.keyClick(Qt.Key_D, Qt.AltModifier, -1)
        menu.open()
    }

    function pressRow() {
        keys.keyClickChar("d", Qt.NoModifier, -1)
        console.log("PRESSED\tbutton\t" + probe.buttonPresses)
        console.log("PRESSED\trow\t" + probe.rowPresses)
        Qt.quit()
    }

    onActiveChanged: probe.start()
    Component.onCompleted: probe.start()

    TestEvent {
        id: keys
    }

    Timer {
        interval: 30000
        running: true
        onTriggered: {
            console.log("PRESSED\tstuck\tactive=" + probe.active + " started=" + probe.started)
            Qt.quit()
        }
    }

    Button {
        text: "Switch to r&d"
        onClicked: probe.buttonPresses++
    }

    Menu {
        id: menu
        onOpened: Qt.callLater(probe.pressRow)

        MenuItem {
            text: "Merge topic into r&d"
            onTriggered: probe.rowPresses++
        }
    }
}
"#;

/// How many times each was pressed in one child.
#[derive(Debug, PartialEq)]
struct Pressed {
    button: u32,
    row: u32,
}

/// The child: a Qt application on the parent's probe. Nothing without the
/// parent's mark, so a run of every ignored test does not start one in a
/// process the other tests share.
#[test]
#[ignore = "started by the tests below, offscreen, as a process of its own"]
fn qt_side() {
    let Some(file) = std::env::var_os(PROBE_FILE) else {
        return;
    };
    let mut app = if std::env::var_os(AS_SHIPPED).is_some() {
        super::application()
    } else {
        QGuiApplication::new()
    };
    let mut engine = QQmlApplicationEngine::new();
    crate::qml_engine::arm(engine.pin_mut());
    crate::qml_engine::load(
        engine.pin_mut(),
        &crate::urlpath::file_url(Path::new(&file)),
    )
    .expect("the probe loads");
    app.pin_mut().exec();
    drop(engine);
    qtbridge::collect_garbage();
    drop(app);
}

/// One child on [`PROBE`], in a folder of its own, and what it said.
fn press(as_shipped: bool) -> Pressed {
    let side = if as_shipped { "shipped" } else { "default" };
    let dir = crate::temp::root().join(format!("pgg-mnemonics-{}-{side}", std::process::id()));
    std::fs::create_dir(&dir).expect("a folder of this child's own for its QML");
    let file = dir.join("Probe.qml");
    std::fs::write(&file, PROBE).expect("the QML");
    let mut child = Command::new(std::env::current_exe().expect("this test binary"));
    child
        .args([
            "mnemonics::tests::qt_side",
            "--exact",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(PROBE_FILE, &file)
        .env("QT_QPA_PLATFORM", "offscreen")
        // The QML's `console.log` on stderr, console or not.
        .env("QT_FORCE_STDERR_LOGGING", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if as_shipped {
        child.env(AS_SHIPPED, "1");
    }
    let out = child.output().expect("the child runs");
    std::fs::remove_dir_all(&dir).expect("the child's folder, with the child gone");
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "the child failed:\n{said}");
    let count = |what: &str| -> u32 {
        said.lines()
            .filter_map(|line| line.split_once("PRESSED\t"))
            .filter_map(|(_, rest)| rest.split_once('\t'))
            .find(|(name, _)| *name == what)
            .and_then(|(_, n)| n.trim().parse().ok())
            .unwrap_or_else(|| panic!("the child said no {what} count:\n{said}"))
    };
    Pressed {
        button: count("button"),
        row: count("row"),
    }
}

#[test]
fn an_ampersand_in_a_text_presses_nothing() {
    assert_eq!(press(true), Pressed { button: 0, row: 0 });
}

/// The probe sees a press: without the switch Qt presses both, except on
/// macOS, where it reads no `&` from the start.
#[test]
fn without_the_switch_qt_presses_by_the_ampersand() {
    let on = u32::from(!cfg!(target_os = "macos"));
    assert_eq!(
        press(false),
        Pressed {
            button: on,
            row: on
        }
    );
}
