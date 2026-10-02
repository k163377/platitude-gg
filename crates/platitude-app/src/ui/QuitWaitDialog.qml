import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// The window was asked to close while git is still writing: the close waits for local writes to settle
// (`RepoSession::close`) and says so here. A dialog, because the subject is the window itself
// (デザイン規約 §可否・警告の出し場所).
//
// **A loading mode, not a question**: no button, no Escape, and modal — a press that queued another write would move
// the moment this waits for (rules-refs/app-ui.md「アプリ終了の close ゲートは 1 本」).
AppDialog {
    id: quitWaitDialog

    /// The writes have settled: the dialog has closed itself, and `WindowQuitGate` closes the window on this.
    signal settled()

    /// Escape would take this down. Read off `closePolicy` itself, so `quit-locked` goes red if the bit comes back.
    readonly property bool escapes: (quitWaitDialog.closePolicy & Popup.CloseOnEscape) !== 0

    /// How many closes this lock has turned away (Alt+F4 and the like; `quit-locked` reads it). Raised **from inside
    /// `WindowQuitGate.gateClose`** — a tally kept beside the veto would go on rising after the veto was gone.
    property int vetoes: 0
    function noteVeto() {
        quitWaitDialog.vetoes++
    }

    /// Close the window on settling. Cleared from outside where the wait itself is being looked at — a write landing
    /// mid-look would take the window down.
    property bool selfCloses: true

    // Overrides `AppDialog`'s `CloseOnEscape`. Nothing else needs a guard: a modal popup already seals the window's
    // `Shortcut`s (F5, Ctrl+F, the bars' Escape), so guarding them in `Main` would be dead code.
    closePolicy: Popup.NoAutoClose

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
                weight: Theme.fontWeightStrong
                text: qsTr("Closing when git finishes")
            }
        }
        // One line (§長さ). The command is not named: the write may belong to a tab already gone.
        CardText {
            Layout.fillWidth: true
            color: Theme.textSecondary
            text: qsTr("A git command is still writing to a repository.")
        }
    }

    // Polls `AppBackend.readyToQuit`: the last write can end in a session a closed tab left behind, which no model of
    // this window hears. Off without `selfCloses`; the gate and the veto stay real either way.
    SampleTimer {
        running: quitWaitDialog.opened && quitWaitDialog.selfCloses
        onTriggered: {
            if (!AppBackend.readyToQuit())
                return
            quitWaitDialog.close()
            quitWaitDialog.settled()
        }
    }
}
