import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Fixed header band at the top of a pane (COMMIT / DIFF / ...).
Rectangle {
    property alias text: headerLabel.text
    Layout.fillWidth: true
    implicitHeight: Theme.headerHeight
    color: Theme.bgElevated
    Label {
        id: headerLabel
        anchors.verticalCenter: parent.verticalCenter
        x: Theme.spaceSm
        font.pixelSize: Theme.fontMd
        font.weight: Font.DemiBold
        color: Theme.textSecondary
    }
    // The band closes with a hairline, the way the sidebar's own band always has. Without it a header sitting on
    // another `bgElevated` row — a diff that opens on a hunk heading, the stash actions under COMMIT — has no edge at
    // all: the two bands share one value (`bgElevated` and `diffHunkHeaderBg` are both #0F172A), so they read as a
    // single 56px box holding both their words (デザイン規約 §diff の中のステージ).
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: Theme.borderWidth
        color: Theme.borderSubtle
    }
}
