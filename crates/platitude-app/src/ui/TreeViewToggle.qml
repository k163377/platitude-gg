import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The tree ⇄ paths view toggle both file lists carry in their header
// band. The one not in use is still the way to switch, so it keeps the
// hue and drops a step rather than falling to the muted colour, which
// would read as unavailable (デザイン規約 §暗く落とした段 / §無効).
RowLayout {
    id: toggle

    /// Which half of the pair the list is showing.
    property bool treeView: false
    signal chosen(bool tree)

    spacing: Theme.spaceXs

    HoverToolButton {
        padding: 0
        implicitWidth: Theme.iconLg
        implicitHeight: Theme.iconLg
        tip: qsTr("Tree view")
        onClicked: toggle.chosen(true)
        contentItem: NavIcon {
            kind: "hier"
            tint: toggle.treeView ? Theme.accent : Theme.accentDim
        }
    }
    HoverToolButton {
        padding: 0
        implicitWidth: Theme.iconLg
        implicitHeight: Theme.iconLg
        tip: qsTr("Paths view")
        onClicked: toggle.chosen(false)
        contentItem: NavIcon {
            kind: "list"
            tint: toggle.treeView ? Theme.accentDim : Theme.accent
        }
    }
}
