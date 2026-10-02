pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// Where a box's right-click menu (`FieldMenu`) is built, on the box's first right-click: a card of rows standing in
// every box from the start would cost each tab a menu per box it never opens.
//
// The box hands Qt's request here and declares the style's menu away beside it:
//     ContextMenu.menu: null
//     ContextMenu.onRequested: <seat>.offer()
// Qt opens a menu it is handed by itself, with no say from `offer()`, and a box that leaves the binding unwritten keeps
// the style's, built on the first right-click (`tst_fieldmenu`). The request's position is left alone: a text box's is
// always its caret (`FieldMenu.offerNow`).
Loader {
    id: seat

    /// The box the menu acts on, and the item this seat is declared in.
    required property Item editor
    /// The menu stands over a box that had the caret (`FieldMenu.holding`): the box still counts as being written in,
    /// though the card has the focus. Read beside `activeFocus` wherever the box dresses for it.
    readonly property bool holding: seat.item !== null && seat.item.holding

    active: false
    sourceComponent: FieldMenu {
        editor: seat.editor
    }

    /// Opens the menu on the box as it stands (`FieldMenu.offerNow`). Says whether it opened.
    function offer() {
        seat.active = true
        return seat.item.offerNow()
    }
}
