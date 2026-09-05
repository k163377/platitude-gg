import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Fixed header band at the top of a pane (COMMIT / DIFF / ...).
Rectangle {
    id: band
    property alias text: headerLabel.text
    /// A tally beside the caption, at the caption's own step — the way every heading band in this window carries one
    /// (`DetailsChangesBand`, NavHeader, WipBucketHeader; 規約 §タイポグラフィ says the weight and the colour are what
    /// make it read as a count). -1 for the bands that count nothing, which is most of them.
    property int count: -1
    Layout.fillWidth: true
    implicitHeight: Theme.headerHeight
    color: Theme.bgElevated
    Label {
        id: headerLabel
        anchors.verticalCenter: parent.verticalCenter
        x: Theme.spaceXs
        font.pixelSize: Theme.fontMd
        font.weight: Font.DemiBold
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
