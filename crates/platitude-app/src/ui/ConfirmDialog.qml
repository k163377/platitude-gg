import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// One dialog for every "this cannot be taken back" question — the menu
// route to a force push, a hard reset, moving a branch onto a remote
// one. The caller supplies the wording and what to run on yes, so the
// phrasing stays next to the operation it describes.
//
// Confirmation is reserved for the irreversible. Everyday operations
// (stage, commit, switch) ask nothing — a prompt on each of those
// teaches people to dismiss prompts.
AppDialog {
    id: confirmDialog

    property string heading: ""
    property string detail: ""
    property string acceptText: ""
    property var action: null

    function ask(heading, detail, acceptText, action) {
        confirmDialog.heading = heading
        confirmDialog.detail = detail
        confirmDialog.acceptText = acceptText
        confirmDialog.action = action
        confirmDialog.open()
    }

    contentItem: ColumnLayout {
        spacing: Theme.spaceLg
        Label {
            text: confirmDialog.heading
            font.pixelSize: Theme.fontLg
            font.weight: Font.DemiBold
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }
        Label {
            text: confirmDialog.detail
            color: Theme.textSecondary
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }
        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: Theme.spaceSm
            HoverButton {
                implicitHeight: Theme.controlHeight
                text: qsTr("Cancel")
                onClicked: confirmDialog.close()
            }
            HoverButton {
                implicitHeight: Theme.controlHeight
                highlighted: true
                text: confirmDialog.acceptText
                onClicked: {
                    const run = confirmDialog.action
                    confirmDialog.close()
                    if (run)
                        run()
                }
            }
        }
    }
}
