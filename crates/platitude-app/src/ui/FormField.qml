import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The same frame at the height buttons and combo boxes use: for form
// input.
TextField {
    id: form

    /// The value is chosen: no caret, and nothing a key can add — but **the ground stays**
    /// (規約 §選ぶ欄と打つ欄 の例外). Taking it away is what §選ぶ欄と打つ欄 asks for, and in a form row it reads as
    /// the wrong thing entirely: with grounded boxes above and below, the one without a ground looks *disabled*
    /// rather than choosable. The disabled signal is the louder of the two, so the ground stays and what the box
    /// does is answered by pressing it.
    ///
    /// What the press opens is the caller's; this only says it happened.
    property bool choosing: false
    /// The box was pressed while it is a choosing one — the door to whatever fills it. Also what Enter and Space
    /// answer, so the box is reachable without a pointer.
    signal picking()

    readOnly: form.choosing
    implicitHeight: Theme.controlHeight
    font.pixelSize: Theme.fontMd
    // The frame-to-word inset every box in the application shares (`SlimField`). The height is what tells a form field
    // from a slim one; the words stand the same distance in either.
    leftPadding: Theme.spaceXs
    rightPadding: Theme.spaceXs
    topPadding: 0
    bottomPadding: 0
    background: Rectangle {
        color: Theme.bgBase
        radius: Theme.radiusSm
        border.color: form.activeFocus ? Theme.borderFocus : Theme.borderDefault
        border.width: Theme.borderWidth
    }

    // Only where the box is a choosing one: a typing box keeps whatever Qt does with its caret, and a `cursorVisible`
    // written for both would hold one up in a box nobody is in.
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
    // **A handler for a named key accepts the event by being connected, whatever its body does** (Qt's attached
    // `Keys`: a key it specifically handles defaults to accepted). So the typing box has to hand the key back
    // itself, or a space typed between two names and the Return that submits a box go nowhere (`tst_formfield`).
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
