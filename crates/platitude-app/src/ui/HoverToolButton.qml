import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// ToolButton with the same explicit hover wash as HoverButton: Fusion's
// own feedback is invisible on this dark palette, and a checked
// ToolButton shows its panel all the time, hiding the hover entirely.
ToolButton {
    id: hoverToolButtonSelf
    /// Wear a button's frame at rest. For the ones that stand on a pane's
    /// own ground with nothing of their own kind beside them: without it
    /// they read as part of whatever they are standing on (the fold
    /// control inside the filter's frame read as part of the input).
    property bool framed: false
    Rectangle {
        anchors.fill: parent
        visible: hoverToolButtonSelf.framed
        color: "transparent"
        radius: Theme.radiusSm
        border.color: Theme.borderSubtle
        border.width: Theme.borderWidth
    }
    Rectangle {
        anchors.fill: parent
        radius: Theme.radiusSm
        color: Theme.bgHover
        visible: hoverToolButtonSelf.enabled && hoverToolButtonSelf.hovered
                 && !hoverToolButtonSelf.down
    }
}
