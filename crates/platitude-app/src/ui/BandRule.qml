import QtQuick
import platitude.ui

// The hairline a band closes with, along the bottom edge of whatever it is put in: without it a header over another
// `bgElevated` row has no edge (デザイン規約 §diff の中のステージ). `color` is the resting one; a state overrides `color`
// and nothing else. A band that has to come forward over an overlap adds `z: 1` where it is used.
Rectangle {
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.bottom: parent.bottom
    height: Theme.borderWidth
    color: Theme.borderSubtle
}
