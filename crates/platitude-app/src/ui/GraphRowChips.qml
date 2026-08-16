pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The chip column of one commit-graph row: the names this commit carries,
// or — on a row that carries none — the box that puts one there. The row
// owns the gestures; this owns what is drawn and what is under a point.
Item {
    id: chipColumn

    /// The chip's records, in the order it reads them out (see the row).
    required property var records
    /// How wide the column is. The chip and the box are laid out from it
    /// rather than from `width`, which a layout settles a frame later.
    required property real columnWidth
    /// This column is a branch-name box right now.
    required property bool naming

    /// The chip itself — what a stacked one is unstacked under.
    readonly property alias chipItem: rowChip

    signal namingSubmitted(string name)
    signal namingEdited(string text)
    signal namingCancelled()

    /// Which stacked chip a point in this column's frame is over, if it is
    /// over one that has something to unstack.
    ///
    /// A chip is `fontSmLine` tall — sixteen pixels in a row of
    /// twenty-eight — and a hand that has just arrived is still settling.
    /// Landing takes the chip itself, but once the list is out (`held`)
    /// the whole chip column holds it: drifting a dozen pixels inside the
    /// column the chips live in is not leaving them (2026-08-09 trace).
    function chipAt(px, py, held) {
        if (!rowChip.visible || rowChip.records.length < 2)
            return null
        const p = chipColumn.mapToItem(rowChip, px, py)
        if (rowChip.contains(Qt.point(p.x, p.y)))
            return rowChip
        if (!held)
            return null
        return chipColumn.contains(Qt.point(px, py)) ? rowChip : null
    }
    /// Carries the box on from whatever the last delegate to hold it was
    /// left with. `text` comes off the view, not off this column: this
    /// delegate is recycled the moment the row scrolls off.
    function takeNamingFocus(text) {
        nameField.text = text
        nameField.forceActiveFocus()
    }

    RefChip {
        id: rowChip
        // Assigning `visible` here replaces the chip's own rule,
        // so the "has anything to show" half has to be repeated:
        // without it a row with no refs draws an empty frame.
        visible: chipColumn.records.length > 0 && !chipColumn.naming
        anchors.right: parent.right
        anchors.rightMargin: Theme.spaceXs
        anchors.verticalCenter: parent.verticalCenter
        records: chipColumn.records
        // The names the chip cannot fit are read in the card, not
        // squeezed here.
        maxWidth: chipColumn.columnWidth - Theme.spaceSm
    }
    // A row with nothing to move to answers the double-click with
    // the one thing that would give it something: a name. The
    // question is asked where the chips would be, not over the
    // window (デザイン規約: 表示の切り替えで足りるならダイアログを出さない).
    SlimField {
        id: nameField
        visible: chipColumn.naming
        anchors.right: parent.right
        anchors.rightMargin: Theme.spaceXs
        anchors.verticalCenter: parent.verticalCenter
        width: chipColumn.columnWidth - 2 * Theme.spaceXs
        font.pixelSize: Theme.fontSm
        placeholderText: qsTr("Create branch here?")
        onAccepted: chipColumn.namingSubmitted(nameField.text.trim())
        // Held on the view, not here: this delegate is recycled
        // the moment the row scrolls off, and half a name is
        // still worth not losing.
        onTextEdited: chipColumn.namingEdited(nameField.text)
        Keys.onEscapePressed: chipColumn.namingCancelled()
    }
    // Nothing in this column can be hovered on its own: the row's
    // MouseArea fills the row and is declared after it, so it takes every
    // hover the chips would have seen (デザイン規約 §hover の
    // ツールチップ). What the stacked chips hold is read from the
    // RefListPopup that MouseArea opens under the pointer.
}
