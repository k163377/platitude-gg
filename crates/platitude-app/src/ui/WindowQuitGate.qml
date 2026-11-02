import QtQuick
import platitude
import platitude.ui

// The close that arrived while git was still writing, and the wait that makes good on it: `onClosing` asks the gate,
// the gate vetoes and stands the dialog up, the dialog takes itself down when the queue settles (`QuitWaitDialog`),
// and the gate closes the window.
//
// The app has one close gate (user decision — ask before changing: local writes run to completion). The ☰'s Exit and
// the band's ✕ both go `TopBar.closeRequested` → `root.close()`, and a stop from outside (a launcher's console on
// Windows, `app_backend/console.rs`; a signal elsewhere, `app_backend/signals.rs`) goes `AppBackend.exitRequested` →
// `root.close()`, so every way out reaches `Main.onClosing`, which asks `AppBackend.readyToQuit()`
// (= `Hub::writes_settled`: the open tabs' `local_writes_pending` and the `parked_writes` closed tabs left behind). The
// gate, the dialog and the yield are this one part. The harness's `Qt.quit()` comes through close as well: an
// automated run is let through only after `finishAutoAct` (`yields`), and writes after that are guarded by the
// `take_write_join` join in `Hub::shutdown`.
// `readyToQuit` is a slot that reads hidden state, the recorded exception: only events and timers call it. A qproperty
// cannot work — a write a closed tab left behind has no drain to notify it. Headless: `quit-waits` / `quit-locked`.
//
// The user must `anchors.fill` it over the window: the dialog sizes off its parent (the `WindowDialogSeat` trap).
Item {
    id: gate

    /// The window whose close this gates — `close()` is re-entered when the wait is over, and passes then.
    property var window
    /// Set by a finished headless run (`Main.finishAutoAct`): its shots are saved and `Hub::shutdown` waits the
    /// write out. Spent by the close it lets through, so a process that outlives its run gets its gate back.
    property bool yields: false
    /// The harness's door to the dialog (`WindowAutoActDriver`).
    readonly property QuitWaitDialog dialog: waitDialog

    function yieldToRun() {
        gate.yields = true
    }

    /// From the window's `onClosing`. Every way out arrives here — ✕, the ☰'s Exit, and even the harness's
    /// `Qt.quit()`, which this runner routes through close.
    function gateClose(close) {
        if (gate.yields) {
            gate.yields = false
            return
        }
        if (AppBackend.readyToQuit())
            return
        close.accepted = false
        waitDialog.noteVeto()
        waitDialog.open()
    }

    QuitWaitDialog {
        id: waitDialog
        onSettled: gate.window.close()
    }
}
