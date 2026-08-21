pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The chip column of one commit-graph row: the names this commit carries, or — on a row that carries none — the box
// that puts one there. The row owns the gestures; this owns what is drawn and what is under a point.
Item {
    id: chipColumn

    /// The chip's records, in the order it reads them out (see the row).
    required property var records
    /// How wide the column is. The chip and the box are laid out from it rather than from `width`, which a layout
    /// settles a frame later.
    required property real columnWidth
    /// This column is a branch-name box right now.
    required property bool naming

    /// The chip itself — what a stacked one is unstacked under.
    readonly property alias chipItem: rowChip

    signal namingSubmitted(string name)
    signal namingEdited(string text)
    signal namingCancelled()

    /// Whether there is a name in this column at all — the whole of what the row needs to divide itself
    /// (`GraphRowDelegate.partAt`).
    ///
    /// **Not "which chip is under this point".** The row divides on this column's own edge rather than on the chip's
    /// frame, so the geometry belongs to the row, which is where the column's width already lives. A frame eighteen
    /// pixels tall in a row of twenty-eight is a boundary nobody can see (2026-08-21 ユーザー報告).
    ///
    /// **Every chip answers, stacked or not** (2026-08-21 ユーザー判断): a chip with one name on it is cut to the column
    /// just the same, and a name that cannot be read is a name that cannot be read — the reason a stack unfolds is the
    /// reason a single one does. What comes out is the same card either way.
    readonly property bool hasChip: rowChip.visible
    /// Carries the box on from whatever the last delegate to hold it was left with. `text` comes off the view, not off
    /// this column: this delegate is recycled the moment the row scrolls off.
    function takeNamingFocus(text) {
        nameField.text = text
        nameField.forceActiveFocus()
    }

    RefChip {
        id: rowChip
        // Assigning `visible` here replaces the chip's own rule, so the "has anything to show" half has to be repeated:
        // without it a row with no refs draws an empty frame.
        visible: chipColumn.records.length > 0 && !chipColumn.naming
        anchors.right: parent.right
        anchors.rightMargin: Theme.spaceXs
        anchors.verticalCenter: parent.verticalCenter
        records: chipColumn.records
        // The names the chip cannot fit are read in the card, not squeezed here.
        maxWidth: chipColumn.columnWidth - Theme.spaceSm
    }
    // A row with nothing to move to answers the double-click with the one thing that would give it something: a name.
    // The question is asked where the chips would be, not over the window (デザイン規約: 表示の切り替えで足りるならダイアログを出さない).
    SlimField {
        id: nameField
        visible: chipColumn.naming
        anchors.right: parent.right
        anchors.rightMargin: Theme.spaceXs
        anchors.verticalCenter: parent.verticalCenter
        width: chipColumn.columnWidth - 2 * Theme.spaceXs
        // Left at the slim field's own size, which is the chip's: what is typed here becomes the chip that stands in
        // this column, so it is read at the size it will be read at. **The placeholder does not fit a column at
        // `labelColW`** — it elides to `Create branch he…`, which asks nothing — but the column is a width a hand can
        // drag, so that is a question about the wording and the floor rather than about this size (2026-08-20 ユーザー判断:
        // 別セッションで直す).
        placeholderText: qsTr("Create branch here?")
        onAccepted: chipColumn.namingSubmitted(nameField.text.trim())
        // Held on the view, not here: this delegate is recycled the moment the row scrolls off, and half a name is
        // still worth not losing.
        onTextEdited: chipColumn.namingEdited(nameField.text)
        Keys.onEscapePressed: chipColumn.namingCancelled()
    }
    // Nothing in this column can be hovered on its own: the row's MouseArea fills the row and is declared after it, so
    // it takes every hover the chips would have seen (デザイン規約 §hover の ツールチップ). What the stacked chips hold is read from
    // the RefListPopup that MouseArea opens under the pointer.
}
