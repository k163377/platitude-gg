import QtQuick
import platitude.ui

// The `✕` that closes a pane, a bar or a tab: an iconLg seat around an
// iconSm mark — a step under what it closes (デザイン規約 §寸法). The
// caller writes only what varies: onClicked, and an Accessible.name or an
// opacity where its host has one to add.
HoverToolButton {
    padding: 0
    implicitWidth: Theme.iconLg
    implicitHeight: Theme.iconLg
    contentItem: Item {
        NavIcon {
            anchors.centerIn: parent
            width: Theme.iconSm
            height: Theme.iconSm
            kind: "close"
            tint: Theme.textSecondary
        }
    }
}
