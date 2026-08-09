import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The folder the picker handed over cannot be opened.
//
// A window of its own rather than the standing question's bar: there is
// nothing on screen for a bar to stand over — the folder never became a
// tab, and the picker it came from has closed. That puts it with the
// identity and remote forms under デザイン規約 §可否・警告の出し場所,
// among the popups for what has nowhere else to go, rather than among
// the confirmations (this asks for nothing back; it says what happened
// and offers the way on).
AppDialog {
    id: openFailedDialog

    /// What was picked, and what git made of it: `plain` (no repository
    /// here or above), `bare` (one with no work tree), `error` (git had
    /// trouble of its own — its wording is all anyone can say).
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

    // The way on takes the focus: the other button is what Escape
    // already does, and every route out of here is one keystroke.
    onOpened: chooseButton.forceActiveFocus()

    contentItem: ColumnLayout {
        spacing: Theme.spaceLg

        Label {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            font.pixelSize: Theme.fontXl
            font.weight: Font.DemiBold
            // One line, and no second one explaining it (§長さ). What
            // git says about parent directories is already carried by
            // "not a repository" — the folder underneath says which.
            text: openFailedDialog.kind === "bare"
                  ? qsTr("A bare repository has nothing to show")
                  : openFailedDialog.kind === "error"
                    ? qsTr("Could not open this folder")
                    : qsTr("Not a git repository")
        }
        // The picker is gone by now, so the folder it landed on has
        // nothing else left to name it. The middle goes first: the leaf
        // is what tells two candidates apart.
        Label {
            Layout.fillWidth: true
            visible: openFailedDialog.kind !== "error"
            color: Theme.textSecondary
            elide: Text.ElideMiddle
            text: openFailedDialog.path
        }
        Label {
            Layout.fillWidth: true
            visible: openFailedDialog.kind === "error"
            color: Theme.danger
            wrapMode: Text.Wrap
            text: openFailedDialog.message
        }

        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: Theme.spaceSm
            HoverButton {
                text: qsTr("Cancel")
                onClicked: openFailedDialog.close()
            }
            HoverButton {
                id: chooseButton
                highlighted: true
                text: qsTr("Choose another folder…")
                onClicked: openFailedDialog.retry()
            }
        }
    }
}
