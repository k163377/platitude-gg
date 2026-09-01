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
// nowhere else to go).
//
// **It asks nothing and takes no answer**: the quit is already decided, so this is a loading mode, not a question.
// There is no way back — no button, no Escape (`closePolicy` below) — and the modal is what seals the rest: a press
// that queued one more write would move the very moment this window is waiting for.
AppDialog {
    id: quitWaitDialog

    /// Nothing git was asked to write is left running: the close this dialog stood for can go through
    /// (`WindowQuitGate` closes the window on it).
    signal settled()

    /// Escape would take this down. **Read off `closePolicy` itself** rather than written beside it, so the verb
    /// that reports it (`quit-locked`) goes red the day somebody hands the policy back its `CloseOnEscape` bit —
    /// a second spelling would go on saying `false` while the key worked.
    readonly property bool escapes: (quitWaitDialog.closePolicy & Popup.CloseOnEscape) !== 0

    /// How many closes this lock has turned away. It takes no answer, so a close that arrives while it stands —
    /// Alt+F4, or anything else reaching the window rather than the screen the modal covers — is refused exactly
    /// the way the first one was, and this is where `quit-locked` reads that (`WindowDialogActs`). Raised **from
    /// inside the handler the window's `onClosing` calls** (`WindowQuitGate.gateClose`): a tally kept beside the
    /// veto would go on rising after the veto itself was gone.
    property int vetoes: 0
    function noteVeto() {
        quitWaitDialog.vetoes++
    }

    // The way out is taken away rather than left and refused: `AppDialog` opens Escape (`CloseOnEscape`), and a
    // loading mode that Escape dismisses is a question wearing a ring. Nothing else in the window needs a guard —
    // **a modal popup seals the window's own `Shortcut`s while it stands** (measured, qmltestrunner: F5 and Cancel
    // both fire with no popup up and under a *modeless* one, and neither fires under a modal one, focus or not).
    // So F5's refresh, Ctrl+F's find and the two bars' Escape are already out of reach here, and holding them down
    // again in `Main` would be code that never runs.
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
    }

    // The sampling beat, not the verdict: the answer is the backend's (`AppBackend.readyToQuit`), asked once per
    // beat while the dialog stands. A beat rather than an event, because the last write can end in a session a
    // closed tab left behind — nothing of it reaches this window's models.
    //
    // Held from answering during a headless run: the photograph is of the wait itself, and the seeded hook's
    // write can land while the shot pipeline is still grabbing — the close this would fire takes the window, and
    // the PNGs, down with it (`finishAutoAct` ends a run, never a window). The gate and the veto stay real; only
    // the self-close is the run's to forgo.
    SampleTimer {
        running: quitWaitDialog.opened && AppBackend.autoAct === ""
        onTriggered: if (AppBackend.readyToQuit()) quitWaitDialog.settled()
    }
}
