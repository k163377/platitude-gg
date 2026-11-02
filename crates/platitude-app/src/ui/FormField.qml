import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// A text box at the height buttons and combo boxes use, for form input.
//
// Never set words beside it on its baseline (`Qt.AlignBaseline`): while its height is not given explicitly, a
// `TextField`'s `baselineOffset` answers where the text would stand packed at the top, not centred (`heightValid()`
// false adds no centring — 4px high in a `controlHeight` box).
TextField {
    id: form

    /// The value is chosen: no caret and nothing a key can add, but the ground stays (規約 §選ぶ欄と打つ欄 の例外 —
    /// among grounded boxes, one without a ground reads as disabled). What the press opens is the caller's.
    property bool choosing: false
    /// The words the empty box shows, drawn by the box itself with a `PlainTextLabel`: a form field's can be the
    /// repository's or the machine's (the path of the git in use, a default read off the backend), and Qt's
    /// `PlaceholderText` reads markup in them (`SummaryArea.placeholder`). `placeholderText` stays empty, so the
    /// style draws nothing.
    property string placeholder: ""
    /// A choosing box was pressed, or took Enter / Space.
    signal picking()

    readOnly: form.choosing
    // The style's own width formula (Fusion's TextField.qml) with this box's placeholder in it.
    implicitWidth: implicitBackgroundWidth + leftInset + rightInset
                   || Math.max(contentWidth, hint.implicitWidth) + leftPadding + rightPadding
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

    // The product's right-click menu, not the style's, and the menu key's (`FieldMenuSeat`).
    ContextMenu.menu: null
    ContextMenu.onRequested: menuSeat.offer()
    Keys.onMenuPressed: event => event.accepted = menuSeat.offer()
    // Laid out as the style's PlaceholderText is (Fusion's TextField.qml), so the words stand where they stood.
    PlainTextLabel {
        id: hint
        x: form.leftPadding
        y: form.topPadding
        width: form.width - (form.leftPadding + form.rightPadding)
        height: form.height - (form.topPadding + form.bottomPadding)
        text: form.placeholder
        font: form.font
        color: form.placeholderTextColor
        horizontalAlignment: form.horizontalAlignment
        verticalAlignment: form.verticalAlignment
        visible: !form.length && !form.preeditText
                 && (!form.activeFocus || form.horizontalAlignment !== Qt.AlignHCenter)
        elide: Text.ElideRight
        renderType: form.renderType
    }
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
