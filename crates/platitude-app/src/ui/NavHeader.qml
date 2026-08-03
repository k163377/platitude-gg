import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Fixed sidebar section header: fold arrow, tinted icon, caption and
// count, plus the optional tags-in-graph toggle.
Rectangle {
    id: header
    property string caption
    property string iconKind: "branch"
    property color iconTint: Theme.textSecondary
    property int count: 0
    property bool expanded: true
    property bool showTagToggle: false
    property bool tagsShown: true
    signal toggled()
    signal tagsToggled(bool shown)

    Layout.fillWidth: true
    implicitHeight: Theme.rowHeight
    color: Theme.bgElevated
    // A HoverHandler rather than the MouseArea's hover: it also fires
    // over the tag toggle, so the row highlight covers the header's
    // full clickable surface.
    HoverHandler {
        id: headerHover
    }
    Rectangle {
        anchors.fill: parent
        color: Theme.bgHover
        visible: headerHover.hovered
    }
    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceSm
        anchors.rightMargin: Theme.spaceXs
        spacing: Theme.spaceXs
        Label {
            text: header.expanded ? "▾" : "▸"
            color: Theme.textSecondary
            font.pixelSize: Theme.fontSm
        }
        NavIcon {
            kind: header.iconKind
            tint: header.iconTint
            width: Theme.iconSm + 2
            height: Theme.iconSm + 2
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
        HoverToolButton {
            visible: header.showTagToggle
            checkable: true
            checked: header.tagsShown
            text: "⚑"
            opacity: checked ? 1.0 : 0.35
            padding: 0
            implicitWidth: Theme.iconLg
            implicitHeight: Theme.iconLg
            ToolTip.visible: hovered
            ToolTip.delay: 600
            ToolTip.text: header.tagsShown ? qsTr("Hide tags in the graph")
                                           : qsTr("Show tags in the graph")
            onToggled: header.tagsToggled(checked)
        }
    }
    MouseArea {
        anchors.fill: parent
        // Leave the toggle button clickable.
        anchors.rightMargin: header.showTagToggle ? Theme.spaceXl : 0
        onClicked: header.toggled()
    }
}
