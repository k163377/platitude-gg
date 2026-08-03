import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// A move with uncommitted changes asks what to do with them instead of
// silently carrying them along: "leave them here" stashes first (the
// default — arriving on another branch with unexplained changes is how
// accidents start), "bring them along" is git's own behaviour.
AppDialog {
    id: dirtySwitchDialog

    // Where the move goes ("main", a short sha, ...).
    property string moveLabel: ""
    // Where the changes are now ("this commit" when detached).
    property string stayLabel: ""
    /// stashFirst: true = leave them here (stash first), false = bring.
    signal resolved(bool stashFirst)

    contentItem: ColumnLayout {
        spacing: Theme.spaceLg
        Label {
            text: qsTr("You have uncommitted changes")
            font.pixelSize: Theme.fontLg
            font.weight: Font.DemiBold
        }
        Label {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            color: Theme.textSecondary
            text: qsTr("Switching to %1 can either leave them where they "
                       + "are or take them with you.").arg(dirtySwitchDialog.moveLabel)
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceSm
            HoverButton {
                Layout.fillWidth: true
                implicitHeight: Theme.controlHeight
                highlighted: true
                text: qsTr("Leave my changes on %1").arg(dirtySwitchDialog.stayLabel)
                onClicked: {
                    dirtySwitchDialog.close()
                    dirtySwitchDialog.resolved(true)
                }
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                color: Theme.textMuted
                font.pixelSize: Theme.fontSm
                text: qsTr("Stashes them first; they wait in STASHES until "
                           + "you apply them again.")
            }
            HoverButton {
                Layout.fillWidth: true
                implicitHeight: Theme.controlHeight
                text: qsTr("Bring my changes to %1").arg(dirtySwitchDialog.moveLabel)
                onClicked: {
                    dirtySwitchDialog.close()
                    dirtySwitchDialog.resolved(false)
                }
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                color: Theme.textMuted
                font.pixelSize: Theme.fontSm
                text: qsTr("Carries them over; git refuses the switch if they "
                           + "would collide with what is there.")
            }
        }
        HoverButton {
            Layout.alignment: Qt.AlignRight
            implicitHeight: Theme.controlHeight
            text: qsTr("Cancel")
            onClicked: dirtySwitchDialog.close()
        }
    }
}
