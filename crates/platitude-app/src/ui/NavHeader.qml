import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Fixed sidebar section header: fold arrow, tinted icon, caption and count, plus the optional tags-in-graph toggle.
Rectangle {
    id: header
    property string caption
    property string iconKind: "branch"
    property color iconTint: Theme.textSecondary
    property int count: 0
    property bool expanded: true
    /// Whether this header's section can be closed from here. False on the one the folded rail opens beside itself:
    /// that list is already the only thing on screen, so an arrow offering to close it would be offering to leave
    /// nothing.
    property bool foldable: true
    property bool showTagToggle: false
    property bool tagsShown: true
    signal toggled()
    signal tagsToggled(bool shown)

    Layout.fillWidth: true
    implicitHeight: Theme.rowHeight
    color: Theme.bgElevated
    // A HoverHandler rather than the MouseArea's hover: it also fires over the tag toggle, so the row highlight covers
    // the header's full clickable surface.
    HoverHandler {
        id: headerHover
    }
    Rectangle {
        anchors.fill: parent
        color: Theme.bgHover
        visible: headerHover.hovered && header.foldable
    }
    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceSm
        anchors.rightMargin: Theme.spaceXs
        spacing: Theme.spaceXs
        NavIcon {
            visible: header.foldable
            width: Theme.iconSm
            height: Theme.iconSm
            kind: "chevron"
            rotation: header.expanded ? 90 : 0
            tint: Theme.textSecondary
        }
        NavIcon {
            kind: header.iconKind
            tint: header.iconTint
            width: Theme.iconMd
            height: Theme.iconMd
        }
        Label {
            text: header.caption
            color: Theme.textSecondary
            font.pixelSize: Theme.fontSm
            font.weight: Font.DemiBold
        }
        Label {
            text: "(" + header.count + ")"
            color: Theme.textMuted
            font.pixelSize: Theme.fontSm
        }
        Item { Layout.fillWidth: true }
        // Whether the graph is drawing tags. An eye rather than a flag: a flag is a mark on a commit, which is what a
        // tag already is — what this switches is whether they are looked at. Told apart by the tint, the way the panes'
        // tree/flat switches are, not by fading the whole control (§暗く落とした段).
        HoverToolButton {
            visible: header.showTagToggle
            checkable: true
            checked: header.tagsShown
            padding: 0
            implicitWidth: Theme.iconLg
            implicitHeight: Theme.iconLg
            contentItem: NavIcon {
                kind: header.tagsShown ? "eye" : "eye-off"
                tint: header.tagsShown ? Theme.refTag : Theme.refTagDim
            }
            tip: header.tagsShown ? qsTr("Hide tags in the graph") : qsTr("Show tags in the graph")
            onToggled: header.tagsToggled(checked)
        }
    }
    MouseArea {
        anchors.fill: parent
        // Leave the toggle button clickable.
        anchors.rightMargin: header.showTagToggle ? Theme.spaceXl : 0
        enabled: header.foldable
        onClicked: header.toggled()
    }
}
