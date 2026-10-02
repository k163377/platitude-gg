pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// Where a box's right-click menu (`FieldMenu`) is built, on the box's first request: a card of rows standing in every
// box from the start would cost each tab a menu per box it never opens. Laid over the box, which it hears the right
// button on.
//
// The box hands Qt's request and the menu key here, and declares the style's menu away beside them:
//     ContextMenu.menu: null
//     ContextMenu.onRequested: <seat>.offer()
//     Keys.onMenuPressed: event => event.accepted = <seat>.offer()
// Qt opens a menu it is handed by itself, with no say from `offer()`, and a box that leaves the binding unwritten keeps
// the style's, built on the first right-click (`tst_fieldmenu`). The request's position is left alone: a text box's is
// always its caret (`FieldMenu.offerNow`).
Item {
    id: seat

    /// The box the menu acts on, and the item this seat is declared in.
    required property Item editor
    /// The menu stands over a box that had the caret (`FieldMenu.holding`): the box still counts as being written in,
    /// though the card has the focus. Read beside `activeFocus` wherever the box dresses for it.
    readonly property bool holding: menuSeat.item !== null && menuSeat.item.holding
    /// The menu, once built.
    readonly property alias item: menuSeat.item

    /// A right button is going down or coming up on the box: a request Qt raises meanwhile is that right-click's,
    /// synthesized inside the press (Linux) or the release (Windows)
    /// (`QQuickWindowPrivate::maybeSynthesizeContextMenuEvent`). Every other request is the keyboard's — xcb's menu key
    /// raises one at the pointer, which may well be resting on the box, so the hand over the box tells nothing.
    readonly property bool rightPressing: rightHand.active || seat.rightReleasing
    property bool rightReleasing: false

    anchors.fill: parent

    /// Opens the menu on the box as it stands (`FieldMenu.offerNow`). Says whether it opened.
    function offer() {
        menuSeat.active = true
        return menuSeat.item.offerNow(seat.rightPressing)
    }

    // A passive grab, which leaves the press to the box under it. Not a `TapHandler`: one on the right button stops Qt
    // synthesizing the request from a press at all.
    PointHandler {
        id: rightHand
        acceptedButtons: Qt.RightButton
        onActiveChanged: {
            if (rightHand.active)
                return
            // The release's own request comes after this, inside the same delivery; the turn after it is past.
            seat.rightReleasing = true
            Qt.callLater(seat.letGo)
        }
    }
    function letGo() {
        seat.rightReleasing = false
    }
    Loader {
        id: menuSeat
        active: false
        sourceComponent: FieldMenu {
            editor: seat.editor
        }
    }
}
