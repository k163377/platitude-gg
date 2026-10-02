import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The folder the picker handed over cannot be opened. A window of its own: the folder never became a tab, so there is
// nothing for a bar to stand over (デザイン規約 §可否・警告の出し場所).
AppDialog {
    id: openFailedDialog

    /// What was picked, and what git made of it: `plain` (no repository here or in any folder above), `bare` (one with
    /// no work tree), or `other` (git had trouble of its own — `message` is its answer).
    property string path: ""
    property string kind: ""
    property string message: ""
    /// The folder to bring the picker back up at, decided in Rust.
    property string near: ""

    /// Show me the picker again, at `near`.
    signal chooseAnother(string near)

    function show(path, kind, message, near) {
        openFailedDialog.path = path
        openFailedDialog.kind = kind
        openFailedDialog.message = message
        openFailedDialog.near = near
        openFailedDialog.open()
    }

    function retry() {
        const near = openFailedDialog.near
        openFailedDialog.close()
        openFailedDialog.chooseAnother(near)
    }

    // The way on takes the focus: the other button is what Escape already does.
    onOpened: actions.acceptButton.forceActiveFocus()

    /// Every gap, padding included, starts a selection (規約 §右のペインの字は掴める) — a modal has no way to the
    /// command log, so this is all there is to copy.
    textContent: failedColumn

    contentItem: ColumnLayout {
        id: failedColumn
        spacing: Theme.spaceLg

        CardText {
            Layout.fillWidth: true
            pixelSize: Theme.fontXl
            weight: Theme.fontWeightStrong
            // One line (§長さ): "not a repository" already says what git says about parent folders. Shared with the
            // tab's failure screen (`Words.openFailure`).
            text: Words.openFailure(openFailedDialog.kind)
        }
        // The folder, wrapped whole: a cut path would hand a dragging reader a `…`.
        CardText {
            Layout.fillWidth: true
            color: Theme.textSecondary
            text: openFailedDialog.path
        }
        // git's words, and the only red here: the gate that reports a git it cannot use draws the line the same way
        // (Main.qml).
        CardText {
            Layout.fillWidth: true
            visible: openFailedDialog.kind === "other"
            color: Theme.danger
            text: openFailedDialog.message
        }

        DialogActions {
            id: actions
            cancelText: Words.cancel
            acceptKind: "folder"
            // `again`, not "another": on `other` nobody knows the folder was the wrong one.
            acceptText: qsTr("Choose again…")
            onCancelled: openFailedDialog.close()
            onAccepted: openFailedDialog.retry()
        }
    }
}
