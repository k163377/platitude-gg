import QtQuick
import platitude.ui

// The hairline a band closes with, drawn along the bottom edge of whatever it is put in.
//
// The band closes with it the way the sidebar's own band always has. Without it a header sitting on another
// `bgElevated` row — a diff that opens on a hunk heading, the stash actions under COMMIT — has no edge at all: the two
// bands share one value (`bgElevated` and `diffHunkHeaderBg` are both #0F172A), so they read as a single 56px box
// holding both their words (デザイン規約 §diff の中のステージ).
//
// `color` is the resting one. A rule that answers a state — a focused filter, a lit name, the tone of an ask — says so
// by overriding it.
Rectangle {
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.bottom: parent.bottom
    height: Theme.borderWidth
    color: Theme.borderSubtle
}
