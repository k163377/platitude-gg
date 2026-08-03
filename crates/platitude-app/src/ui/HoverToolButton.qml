import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// ToolButton with the same explicit hover wash as HoverButton: Fusion's
// own feedback is invisible on this dark palette, and a checked
// ToolButton shows its panel all the time, hiding the hover entirely.
ToolButton {
    id: hoverToolButtonSelf
    Rectangle {
        anchors.fill: parent
        radius: Theme.radiusSm
        color: Theme.bgHover
        visible: hoverToolButtonSelf.enabled && hoverToolButtonSelf.hovered
                 && !hoverToolButtonSelf.down
    }
}
