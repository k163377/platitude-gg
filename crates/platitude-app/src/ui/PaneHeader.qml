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
        font.pixelSize: Theme.fontSm
        font.weight: Font.DemiBold
        color: Theme.textSecondary
    }
}
