import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// The window was asked to close while git is still writing. A local write is waited out — half a write is the one
// thing worse than a late one (`RepoSession::close`) — so the close is put off, said out loud here, and made good
// the moment the queue settles.
//
// A window of its own: the subject is the window itself, so there is no list for a bar to stand over — the seat
// the folder that would not open sits in (デザイン規約 §可否・警告の出し場所, the popups for what has nowhere else
// to go).
//
// **It asks nothing and takes no answer**: the quit is already decided, so this is a loading mode. There is no way
// back — no button, no Escape (`closePolicy` below) — and the modal is what seals the rest: a press that queued one
// more write would move the very moment this window is waiting for.
AppDialog {
    id: quitWaitDialog

    /// Nothing git was asked to write is left running. The dialog has already taken itself down — what it stood for
    /// is over — and the close it stood for can go through (`WindowQuitGate` closes the window on it).
    signal settled()

    /// Escape would take this down. **Read off `closePolicy` itself**, so the verb that reports it (`quit-locked`)
    /// goes red the day somebody hands the policy back its `CloseOnEscape` bit — a second spelling would go on
    /// saying `false` while the key worked.
    readonly property bool escapes: (quitWaitDialog.closePolicy & Popup.CloseOnEscape) !== 0

    /// How many closes this lock has turned away. It takes no answer, so a close that arrives while it stands —
    /// Alt+F4, or anything else reaching the window itself — is refused exactly the way the first one was, and
    /// this is where `quit-locked` reads that (`WindowDialogActs`). Raised **from inside the handler the window's
    /// `onClosing` calls** (`WindowQuitGate.gateClose`): a tally kept beside the veto would go on rising after the
    /// veto itself was gone.
    property int vetoes: 0
    function noteVeto() {
        quitWaitDialog.vetoes++
    }

    /// The dialog closes the window itself the moment the queue settles. Written from outside where the wait itself
    /// is what somebody is looking at — a write landing under a look takes the window down with it, and the wait
    /// stops being a thing that can be seen at all.
    property bool selfCloses: true

    // The way out is taken away: `AppDialog` opens Escape (`CloseOnEscape`), and a loading mode that Escape
    // dismisses is a question wearing a ring. Nothing else in the window needs a guard — **a modal popup seals the
    // window's own `Shortcut`s while it stands** (measured, qmltestrunner: F5 and Cancel both fire with no popup up
    // and under a *modeless* one, and neither fires under a modal one, focus or not). So F5's refresh, Ctrl+F's find
    // and the two bars' Escape are already out of reach here, and holding them down again in `Main` would be code
    // that never runs.
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
        // One line (§長さ): the heading already says what happens next. Which command it is stays with the command
        // log — this window cannot know it (the write may belong to a tab already gone).
        CardText {
            Layout.fillWidth: true
            color: Theme.textSecondary
            text: qsTr("A git command is still writing to a repository.")
        }
    }

    // The sampling beat: the answer is the backend's (`AppBackend.readyToQuit`), asked once per beat while the
    // dialog stands. A beat, because the last write can end in a session a closed tab left behind — nothing of it
    // reaches this window's models.
    //
    // The self-close can be forgone from outside ([`selfCloses`]): where the wait itself is the thing being looked
    // at, a write landing mid-look would take the window down with it. The gate and the veto stay real either way.
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
