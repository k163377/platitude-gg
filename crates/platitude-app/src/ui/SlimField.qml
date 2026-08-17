import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// Slim single-line input shared by the toolbar search and the in-place
// name boxes: thin frame, compact height.
TextField {
    id: slim
    /// What is typed cannot be accepted (§可否・警告の出し場所: the frame
    /// says so where the typing is, and the reason waits in the tooltip).
    property bool refused: false
    implicitHeight: Theme.iconLg
    font.pixelSize: Theme.fontMd
    leftPadding: Theme.spaceSm
    rightPadding: Theme.spaceSm
    topPadding: 0
    bottomPadding: 0
    background: Rectangle {
        color: Theme.bgBase
        radius: Theme.radiusSm
        border.color: slim.refused ? Theme.warning : slim.activeFocus ? Theme.borderFocus : Theme.borderDefault
        border.width: Theme.borderWidth
    }
}
