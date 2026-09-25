import QtQuick
import platitude.ui

// The hairline a band closes with, along the bottom edge of whatever it is put in: without it a header over another
// `bgElevated` row has no edge (デザイン規約 §diff の中のステージ). `color` is the resting one; a state overrides it.
Rectangle {
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.bottom: parent.bottom
    height: Theme.borderWidth
    color: Theme.borderSubtle
}
