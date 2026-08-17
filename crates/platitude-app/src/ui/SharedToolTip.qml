import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

/// The shared tooltip — the one popup in this app nobody declares. The attached property builds it from the style, so
/// it arrives in Fusion's own clothes: a pale yellow ground, a frame that reads the *text* role (so the palette cannot
/// separate the two), and a drawn shadow. Reaching it through `host` is the only way to dress it, and dressing it once
/// carries to every `ToolTip.text` in the tree.
///
/// An Item, not a QtObject: the two Components below are children, and a QtObject has nowhere to put a child.
Item {
    id: shared

    /// The item the attached tooltip is read off — one is enough for the whole tree. Required: `sharedTip` below is
    /// read while this is built.
    required property Item host

    readonly property var sharedTip: shared.host.ToolTip.toolTip

    /// Puts the app's own card on it (デザイン規約 §背景 names `bgElevated` as the tooltip's ground).
    function dressToolTip() {
        shared.sharedTip.background = tipGround.createObject(shared.sharedTip)
        shared.sharedTip.contentItem = tipWord.createObject(shared.sharedTip)
        shared.sharedTip.padding = Theme.spaceSm
    }

    Component {
        id: tipGround
        Rectangle {
            color: Theme.bgElevated
            radius: Theme.radiusMd
            border.color: Theme.borderDefault
            border.width: Theme.borderWidth
        }
    }
    Component {
        id: tipWord
        Text {
            text: shared.sharedTip.text
            font: shared.sharedTip.font
            color: Theme.textPrimary
            wrapMode: Text.Wrap
        }
    }
}
