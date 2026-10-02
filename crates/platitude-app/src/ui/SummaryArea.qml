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
    wrapMode: TextArea.Wrap
    font.pixelSize: Theme.fontLg
    font.weight: Font.DemiBold
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

    // The product's right-click menu, not the style's (`FieldMenuSeat`).
    ContextMenu.menu: null
    ContextMenu.onRequested: menuSeat.offer()
    FieldMenuSeat {
        id: menuSeat
        editor: summary
    }
}
