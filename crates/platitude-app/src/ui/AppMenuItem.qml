import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// One AppMenu row: the list row height and body size used everywhere
// else, and its own words as its width. A row elides only where the
// menu has run out of window to grow into; hovering an elided row says
// the whole line.
MenuItem {
    id: menuItem
    padding: Theme.spaceSm
    topPadding: 0
    bottomPadding: 0
    implicitHeight: Theme.rowHeight
    implicitWidth: itemLabel.implicitWidth
                   + menuItem.leftPadding + menuItem.rightPadding
    font.pixelSize: Theme.fontMd

    ToolTip.visible: menuItem.hovered && itemLabel.truncated
    ToolTip.delay: 600
    ToolTip.text: menuItem.text

    contentItem: Label {
        id: itemLabel
        text: menuItem.text
        font: menuItem.font
        elide: Text.ElideRight
        verticalAlignment: Text.AlignVCenter
        // Clear of the arrow the style paints over the row's right
        // edge on a row that opens a submenu.
        rightPadding: menuItem.subMenu && menuItem.arrow
                      ? menuItem.arrow.width + Theme.spaceXs : 0
        color: !menuItem.enabled ? Theme.textMuted
             : menuItem.highlighted ? Theme.textOnAccent : Theme.textPrimary
    }

    background: Rectangle {
        radius: Theme.radiusSm
        color: menuItem.highlighted ? Theme.accent : "transparent"
    }
}
