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
    /// How far the caption moves up to stand its capitals on the band's middle (`WordFace.capitalDrop`; デザイン規約
    /// §アイコンの席「大文字だけの見出しの語」); its count goes with it.
    readonly property real capsLift: -WordFace.capitalDrop(headerLabel)
    Layout.fillWidth: true
    implicitHeight: Theme.headerHeight
    color: Theme.bgElevated
    Label {
        id: headerLabel
        anchors.verticalCenter: parent.verticalCenter
        transform: Translate { y: band.capsLift }
        x: Theme.spaceXs
        font.pixelSize: Theme.fontMd
        font.weight: Theme.fontWeightStrong
        color: Theme.textSecondary
    }
    Label {
        anchors.verticalCenter: parent.verticalCenter
        anchors.left: headerLabel.right
        anchors.leftMargin: Theme.spaceXs
        transform: Translate { y: band.capsLift }
        visible: band.count >= 0
        text: "(" + band.count + ")"
        font.pixelSize: Theme.fontMd
        color: Theme.textMuted
    }
    BandRule {}
}
