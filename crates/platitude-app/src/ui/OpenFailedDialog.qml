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
    /// here or in any folder above) or `bare` (one with no work tree).
    /// Nothing else opens this — a check that could not say what went
    /// wrong opens the folder as a tab instead, where git's own words
    /// already are.
    property string path: ""
    property string kind: ""
    /// The folder to bring the picker back up at, decided in Rust.
    property string near: ""

    /// Show me the picker again, at `near`.
    signal chooseAnother(string near)

    function show(path, kind, near) {
        openFailedDialog.path = path
        openFailedDialog.kind = kind
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
                  : qsTr("Not a git repository")
        }
        // The picker is gone by now, so the folder it landed on has
        // nothing else left to name it. The middle goes first: the leaf
        // is what tells two candidates apart.
        Label {
            Layout.fillWidth: true
            color: Theme.textSecondary
            elide: Text.ElideMiddle
            text: openFailedDialog.path
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
                text: qsTr("Choose another…")
                onClicked: openFailedDialog.retry()
            }
        }
    }
}
