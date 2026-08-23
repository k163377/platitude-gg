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
    /// The frame's colour while the box has the keyboard. The plain focus
    /// ring for a box that is only a box (the search band, a URL); the
    /// **kind's own colour** for one that is naming a ref, which is the
    /// rule the chip it becomes already follows (§ref の種別: 枠 = 種別).
    /// A refusal outranks it — that is a different axis, and the one the
    /// reader has to answer before anything else.
    property color focusTone: Theme.borderFocus
    implicitHeight: Theme.iconLg
    font.pixelSize: Theme.fontMd
    leftPadding: Theme.spaceSm
    rightPadding: Theme.spaceSm
    topPadding: 0
    bottomPadding: 0
    background: Rectangle {
        color: Theme.bgBase
        radius: Theme.radiusSm
        border.color: slim.refused ? Theme.warning : slim.activeFocus ? slim.focusTone : Theme.borderDefault
        border.width: Theme.borderWidth
    }
}
