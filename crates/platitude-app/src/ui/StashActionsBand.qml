import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Stash actions when the selected row is a stash: the reflog selector
// named in mono, and the two things git can do with it from here.
// (Delete lives on the row's menu, with the question held on the row —
// rules-refs/app-ui.md §stash.)
Rectangle {
    id: band

    /// Reflog selector ("" hides the band).
    property string stashRef: ""

    signal applyRequested(string selector)
    signal popRequested(string selector)

    visible: band.stashRef !== ""
    implicitHeight: Theme.headerHeight
    color: Theme.bgElevated
    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceSm
        anchors.rightMargin: Theme.spaceXs
        spacing: Theme.spaceXs
        NavIcon {
            kind: "stash"
            tint: Theme.textSecondary
            width: Theme.iconMd
            height: Theme.iconMd
        }
        Label {
            text: band.stashRef
            font.family: Theme.monoFamily
            font.pixelSize: Theme.fontSm
            color: Theme.textSecondary
            elide: Text.ElideRight
            Layout.fillWidth: true
        }
        HoverToolButton {
            text: qsTr("Apply")
            font.pixelSize: Theme.fontSm
            tip: qsTr("Apply this stash, keeping it")
            onClicked: band.applyRequested(band.stashRef)
        }
        HoverToolButton {
            text: qsTr("Pop")
            font.pixelSize: Theme.fontSm
            tip: qsTr("Apply this stash and drop it")
            onClicked: band.popRequested(band.stashRef)
        }
    }
}
