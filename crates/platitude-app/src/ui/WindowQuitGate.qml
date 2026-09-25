import QtQuick
import platitude
import platitude.ui

// The close that arrived while git was still writing, and the wait that makes good on it: `onClosing` asks the gate,
// the gate vetoes and stands the dialog up, the dialog takes itself down when the queue settles (`QuitWaitDialog`),
// and the gate closes the window.
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
