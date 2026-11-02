import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The summary box of a commit message, shared by the commit editor and the details pane. Exactly one line: a summary
// is the first line (platitude-core's `split_message`), so anything after a newline would come back in the description
// box. A `TextArea` only to wrap, so a long summary stays readable and the frame grows to fit.
TextArea {
    id: summary
    /// Being written in: the caret is here, or this box's own menu stands over it (`FieldMenuSeat.holding`).
    readonly property bool caretHeld: summary.activeFocus || menuSeat.holding
    /// The words the empty box shows, drawn by the box itself with a `PlainTextLabel`: a stopped merge's message is
    /// the repository's, and Qt's `PlaceholderText` reads its markup (`<b>` or a bare `&lt;` turns it rich —
    /// `merge-standing`). `placeholderText` stays empty, so the style draws nothing.
    property string placeholder: ""
    // The style's own size formula (Fusion's TextArea.qml) with this box's placeholder in it: a standing message
    // that wraps grows the box, as the style's placeholder did.
    implicitWidth: Math.max(contentWidth + leftPadding + rightPadding,
                            implicitBackgroundWidth + leftInset + rightInset,
                            hint.implicitWidth + leftPadding + rightPadding)
    implicitHeight: Math.max(contentHeight + topPadding + bottomPadding,
                             implicitBackgroundHeight + topInset + bottomInset,
                             hint.implicitHeight + topPadding + bottomPadding)
    wrapMode: TextArea.Wrap
    font.pixelSize: Theme.fontLg
    font.weight: Theme.fontWeightStrong
    color: Theme.textPrimary
    background: null
    padding: 0

    // Return is the one keystroke that inserts a newline.
    Keys.onReturnPressed: (event) => { event.accepted = true }
    Keys.onEnterPressed: (event) => { event.accepted = true }
    // A paste, a drop or a prefill can carry newlines too: they fold to a space, as git's own %s does. The assignment
    // re-enters this handler; the first test ends that.
    onTextChanged: {
        if (!/[\n\r]/.test(summary.text))
            return
        const kept = summary.text.substring(0, summary.cursorPosition)
        summary.text = summary.text.replace(/[\n\r]+/g, " ")
        summary.cursorPosition = kept.replace(/[\n\r]+/g, " ").length
    }

    // The product's right-click menu, not the style's, and the menu key's (`FieldMenuSeat`).
    ContextMenu.menu: null
    ContextMenu.onRequested: menuSeat.offer()
    Keys.onMenuPressed: event => event.accepted = menuSeat.offer()
    // Laid out as the style's PlaceholderText is (Fusion's TextArea.qml), so the words stand where they stood.
    PlainTextLabel {
        id: hint
        x: summary.leftPadding
        y: summary.topPadding
        width: summary.width - (summary.leftPadding + summary.rightPadding)
        height: summary.height - (summary.topPadding + summary.bottomPadding)
        text: summary.placeholder
        font: summary.font
        color: summary.placeholderTextColor
        horizontalAlignment: summary.horizontalAlignment
        verticalAlignment: summary.verticalAlignment
        visible: !summary.length && !summary.preeditText
                 && (!summary.activeFocus || summary.horizontalAlignment !== Qt.AlignHCenter)
        wrapMode: summary.wrapMode
        elide: Text.ElideRight
        renderType: summary.renderType
    }
    FieldMenuSeat {
        id: menuSeat
        editor: summary
    }
}
