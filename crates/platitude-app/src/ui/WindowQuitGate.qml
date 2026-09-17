import QtQuick
import platitude
import platitude.ui

// The close that arrived while git was still writing, and the wait that makes good on it. A part of its own so the
// whole of the quit wait is read in one place (and `Main` keeps its size): the window's `onClosing` asks the gate,
// the gate vetoes and stands the dialog up, the dialog takes itself down when the queue settles
// (`QuitWaitDialog`), and the gate closes the window on its word.
//
// The user gives this the window's whole face (`anchors.fill`): the dialog's sizing reads its parent, and an
// unsized seat hands it a 0x0 to fill — the words came out bare in the top-left corner (the trap
// `WindowDialogSeat` has already written down, measured again here).
Item {
    id: gate

    /// The window whose close this gates — `close()` is re-entered when the wait is over, and passes then.
    property var window
    /// A finished headless run is the one caller the gate steps aside for (`Main.finishAutoAct`): its shots are
    /// saved, nothing after them needs the window, and the shutdown joining the write loops (`Hub::shutdown`) is
    /// what waits the seeded write out. Latched — the yield is an event in the run's life — and
    /// spent by the close it lets through, so a process that outlives its run gets its gate back.
    property bool yields: false
    /// The harness's door to the dialog (`WindowAutoActDriver`).
    readonly property QuitWaitDialog dialog: waitDialog

    function yieldToRun() {
        gate.yields = true
    }

    /// Asked from the window's `onClosing`, with the close event in hand: accepts by doing nothing, or vetoes and
    /// stands the dialog up. Every road out of the app arrives there — the band's ✕ and the ☰'s Exit meet at
    /// `root.close()`, and even the harness's `Qt.quit()` is routed through the windows' close by this runner
    /// (measured: a vetoed run's first quit left the loop turning, and the watchdog's second one, fired after the
    /// write had landed, took the window down).
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
