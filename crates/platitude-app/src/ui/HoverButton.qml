import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// Fusion's built-in hover feedback is a few-percent tint that vanishes
// on this dark palette, so every clickable control layers the theme's
// hover wash on top instead (デザイン規約: ホバーは bgHover の重ね色).
// The wash lifts while pressed so Fusion's darker pressed face stays
// visible.
Button {
    id: hoverButtonSelf
    Rectangle {
        anchors.fill: parent
        radius: Theme.radiusSm
        color: Theme.bgHover
        visible: hoverButtonSelf.enabled && hoverButtonSelf.hovered
                 && !hoverButtonSelf.down
    }
}
