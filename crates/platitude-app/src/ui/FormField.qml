import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// A text box at the height buttons and combo boxes use, for form input.
TextField {
    id: form

    /// The value is chosen: no caret and nothing a key can add, but the ground stays (規約 §選ぶ欄と打つ欄 の例外 —
    /// among grounded boxes, one without a ground reads as disabled). What the press opens is the caller's.
    property bool choosing: false
    /// A choosing box was pressed, or took Enter / Space.
    signal picking()

    readOnly: form.choosing
    implicitHeight: Theme.controlHeight
    font.pixelSize: Theme.fontMd
    // The inset every box shares (`SlimField`); only the height tells a form field from a slim one.
    leftPadding: Theme.spaceXs
    rightPadding: Theme.spaceXs
    topPadding: 0
    bottomPadding: 0
    background: Rectangle {
        color: Theme.bgBase
        radius: Theme.radiusSm
        border.color: form.activeFocus || menuSeat.holding ? Theme.borderFocus : Theme.borderDefault
        border.width: Theme.borderWidth
    }

    // The product's right-click menu, not the style's (`FieldMenuSeat`).
    ContextMenu.menu: null
    ContextMenu.onRequested: menuSeat.offer()
    FieldMenuSeat {
        id: menuSeat
        editor: form
    }

    // Only while choosing: writing `cursorVisible` for a typing box would show a caret where nobody is typing.
    Binding {
        target: form
        property: "cursorVisible"
        value: false
        when: form.choosing
    }
    TapHandler {
        enabled: form.choosing
        onTapped: form.picking()
    }
    // A connected `Keys.on<Key>Pressed` accepts the key whatever its body does, so the typing box hands it back, or
    // Space and Return go nowhere (`tst_formfield`).
    function answerKey(event) {
        if (form.choosing)
            form.picking()
        else
            event.accepted = false
    }
    Keys.onReturnPressed: event => form.answerKey(event)
    Keys.onEnterPressed: event => form.answerKey(event)
    Keys.onSpacePressed: event => form.answerKey(event)
}
