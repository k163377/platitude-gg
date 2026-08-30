import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// The window was asked to close while git is still writing. A local write is waited out, never killed — half a write
// is the one thing worse than a late one (`RepoSession::close`) — so the close is put off, said out loud here, and
// made good the moment the queue settles.
//
// A window of its own rather than a bar: the subject is the window itself, so there is no list for a bar to stand
// over — the seat the folder that would not open sits in (デザイン規約 §可否・警告の出し場所, the popups for what has
// nowhere else to go). It asks for nothing back; the one button is the way to change one's mind, and Escape says the
// same thing. Modal on purpose: a press that queued one more write would move the very moment this window is
// waiting for.
AppDialog {
    id: quitWaitDialog

    /// Nothing git was asked to write is left running: the close this dialog stood for can go through
    /// (`WindowQuitGate` closes the window on it).
    signal settled()

    /// Stay after all: the pending quit is abandoned with the dialog. The button's handler and the harness both
    /// come through here (app-ui.md §UI 自動化の因果性 — the run presses what the hand presses).
    function keepWorking() {
        quitWaitDialog.close()
    }

    // The way back in takes the focus, so either road out of here — the word or Escape — is one keystroke.
    onOpened: actions.acceptButton.forceActiveFocus()

    contentItem: ColumnLayout {
        spacing: Theme.spaceLg

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceSm
            SpinnerIcon {
                spinning: quitWaitDialog.opened
            }
            CardText {
                Layout.fillWidth: true
                pixelSize: Theme.fontXl
                weight: Font.DemiBold
                text: qsTr("Closing when git finishes")
            }
        }
        // One line, and no promise beyond it (§長さ): the heading already says what happens next. Which command it
        // is stays with the command log — this window cannot know it (the write may belong to a tab already gone).
        CardText {
            Layout.fillWidth: true
            color: Theme.textSecondary
            text: qsTr("A git command is still writing to a repository.")
        }

        DialogActions {
            id: actions
            acceptText: qsTr("Keep working")
            onAccepted: quitWaitDialog.keepWorking()
        }
    }

    // The sampling beat, not the verdict: the answer is the backend's (`AppBackend.readyToQuit`), asked once per
    // beat while the dialog stands. A beat rather than an event, because the last write can end in a session a
    // closed tab left behind — nothing of it reaches this window's models.
    //
    // Held from answering during a headless run: the photograph is of the wait itself, and the seeded hook's
    // write can land while the shot pipeline is still grabbing — the close this would fire takes the window, and
    // the PNGs, down with it (`finishAutoAct` ends a run, never a window). The gate, the veto and the way back
    // in stay real; only the self-close is the run's to forgo.
    SampleTimer {
        running: quitWaitDialog.opened && AppBackend.autoAct === ""
        onTriggered: if (AppBackend.readyToQuit()) quitWaitDialog.settled()
    }
}
