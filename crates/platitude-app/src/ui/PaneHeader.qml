import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Fixed header band at the top of a pane (COMMIT / DIFF / ...).
Rectangle {
    id: band
    property alias text: headerLabel.text
    /// A tally beside the caption, at the caption's step (規約 §タイポグラフィ「数えは語と同じ段」). -1 counts nothing.
    property int count: -1
    Layout.fillWidth: true
    implicitHeight: Theme.headerHeight
    color: Theme.bgElevated
    Label {
        id: headerLabel
        anchors.verticalCenter: parent.verticalCenter
        x: Theme.spaceXs
        font.pixelSize: Theme.fontMd
        font.weight: Theme.fontWeightStrong
        color: Theme.textSecondary
    }
    Label {
        anchors.verticalCenter: parent.verticalCenter
        anchors.left: headerLabel.right
        anchors.leftMargin: Theme.spaceXs
        visible: band.count >= 0
        text: "(" + band.count + ")"
        font.pixelSize: Theme.fontMd
        color: Theme.textMuted
    }
    BandRule {}
}
