import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Editor for one commit's message. The commit's current message rides
// in through the details model (the row that opened the menu was
// selected by the same click, so its details are on their way); saving
// before that would replace a real message with an empty editor.
AppDialog {
    id: rewordDialog

    // Details model of the owning page; supplies the current message.
    required property var details
    property string oid: ""
    property bool published: false
    // First 8 chars, for the explanatory line.
    readonly property string shortSha: oid.substring(0, 8)
    /// The new message, ready to run (the owner adds rewrite warnings).
    signal submitted(string oid, string subject, string body, bool published)

    /// The current message has been copied in.
    property bool filled: false

    onOpened: {
        rewordDialog.filled = false
        editSubject.text = ""
        editBody.text = ""
        rewordDialog.fill()
        editSubject.forceActiveFocus()
    }
    function fill() {
        if (rewordDialog.filled || !rewordDialog.visible
                || rewordDialog.details.shaHex !== rewordDialog.oid)
            return
        rewordDialog.filled = true
        editSubject.text = rewordDialog.details.messageSubject
        editBody.text = rewordDialog.details.messageBody
    }
    Connections {
        target: rewordDialog.details
        function onChanged() { rewordDialog.fill() }
    }
    function submit() {
        if (!rewordDialog.filled || editSubject.text.trim() === "")
            return
        const oid = rewordDialog.oid
        const published = rewordDialog.published
        const subject = editSubject.text
        const body = editBody.text
        rewordDialog.close()
        rewordDialog.submitted(oid, subject, body, published)
    }
    contentItem: ColumnLayout {
        spacing: Theme.spaceLg
        Label {
            text: qsTr("Edit commit message")
            font.pixelSize: Theme.fontXl
            font.weight: Font.DemiBold
        }
        Label {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            color: Theme.textSecondary
            text: qsTr("%1 — the newest commit is amended in place; an older "
                       + "one is replayed, which gives every commit after it a "
                       + "new identity.").arg(rewordDialog.shortSha)
        }
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: editSubject.implicitHeight + Theme.spaceSm
            color: Theme.bgBase
            radius: Theme.radiusMd
            border.color: Theme.borderDefault
            border.width: Theme.borderWidth
            TextArea {
                id: editSubject
                anchors.fill: parent
                anchors.margins: Theme.spaceXs
                wrapMode: TextArea.Wrap
                placeholderText: qsTr("Commit summary")
                font.pixelSize: Theme.fontLg
                font.weight: Font.DemiBold
                background: null
                padding: 0
            }
        }
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 120
            color: Theme.bgBase
            radius: Theme.radiusMd
            border.color: Theme.borderSubtle
            border.width: Theme.borderWidth
            ScrollView {
                anchors.fill: parent
                anchors.margins: Theme.spaceXs
                TextArea {
                    id: editBody
                    wrapMode: TextArea.Wrap
                    placeholderText: qsTr("Description")
                    font.pixelSize: Theme.fontMd
                    color: Theme.textSecondary
                    background: null
                    padding: 0
                }
            }
        }
        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: Theme.spaceSm
            HoverButton {
                implicitHeight: Theme.controlHeight
                text: qsTr("Cancel")
                onClicked: rewordDialog.close()
            }
            HoverButton {
                implicitHeight: Theme.controlHeight
                highlighted: true
                text: qsTr("Save message")
                enabled: rewordDialog.filled && editSubject.text.trim() !== ""
                onClicked: rewordDialog.submit()
            }
        }
    }
}
