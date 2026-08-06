import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// One AppMenu row: the list row height and body size used everywhere
// else, and its own words as its width. A row elides only where the
// menu has run out of window to grow into; hovering an elided row says
// the whole line.
MenuItem {
    id: menuItem

    /// A short warning said after the row's words ("already pushed").
    /// The row still runs on click — this is the tag that says what it
    /// costs, the same shape the amend editor uses.
    property string note: ""

    padding: Theme.spaceSm
    topPadding: 0
    bottomPadding: 0
    // A row this menu is not offering takes no room. The list lays its
    // rows out by height, so an invisible one that keeps a height leaves
    // an empty row behind — a hole where the reader looks for the row
    // that is missing (measured on the file menu, whose two destructive
    // rows are one per bucket).
    implicitHeight: menuItem.visible ? Theme.rowHeight : 0
    implicitWidth: itemLabel.implicitWidth
                   + (menuItem.note !== ""
                      ? noteLabel.implicitWidth + Theme.spaceSm : 0)
                   + menuItem.leftPadding + menuItem.rightPadding
    font.pixelSize: Theme.fontMd

    ToolTip.visible: menuItem.hovered && itemLabel.truncated
    ToolTip.delay: 600
    ToolTip.text: menuItem.text

    contentItem: RowLayout {
        spacing: Theme.spaceSm
        Label {
            id: itemLabel
            Layout.fillWidth: true
            text: menuItem.text
            font: menuItem.font
            elide: Text.ElideRight
            verticalAlignment: Text.AlignVCenter
            // Clear of the arrow the style paints over the row's right
            // edge on a row that opens a submenu (the note, when there
            // is one, is what sits last instead).
            rightPadding: !noteLabel.visible && menuItem.subMenu && menuItem.arrow
                          ? menuItem.arrow.width + Theme.spaceXs : 0
            color: !menuItem.enabled ? Theme.textMuted
                 : menuItem.highlighted ? Theme.textOnAccent : Theme.textPrimary
        }
        Label {
            id: noteLabel
            visible: menuItem.note !== ""
            text: menuItem.note
            verticalAlignment: Text.AlignVCenter
            rightPadding: menuItem.subMenu && menuItem.arrow
                          ? menuItem.arrow.width + Theme.spaceXs : 0
            color: Theme.warning
            font.pixelSize: Theme.fontSm
        }
    }

    background: Rectangle {
        radius: Theme.radiusSm
        color: menuItem.highlighted ? Theme.accent : "transparent"
    }
}
