import QtQuick
import platitude.ui

// The right-click menu of a box words are typed or read in: the rows of the style's own (`TextEditingContextMenu`), on
// the product's card (デザイン規約 §メニュー の文字の欄). A fixed table: a row the box has no use for goes (a read-only
// box takes no typing), and a row that cannot be done right now stays, greyed. Settled as the menu opens. Built by
// `FieldMenuSeat`.
AppMenu {
    id: fieldMenu

    /// The box the rows act on: a `TextField` or a `TextArea`, whose editing calls the rows make.
    required property Item editor

    // Decided as the menu opens and held while it stands (規約 §メニュー).
    /// The box takes typing: the rows that change its words are on the table.
    property bool typing: false
    property bool canUndo: false
    property bool canRedo: false
    /// Cut and Delete alike: there is a selection.
    property bool canCut: false
    property bool canCopy: false
    property bool canPaste: false
    /// Something left out of the selection: selecting all of an empty box, or of one already all selected, does
    /// nothing.
    property bool canSelectAll: false

    /// The box had the caret when the card opened. The card takes the window's focus while it stands — it answers the
    /// keys, Escape included, only with it — so the box's `activeFocus` drops under it; what the box dresses as "being
    /// written in" reads `holding` beside it, and the box gets the caret back as the card goes.
    property bool hadCaret: false
    readonly property bool holding: fieldMenu.visible && fieldMenu.hadCaret
    onClosed: {
        if (fieldMenu.hadCaret && !fieldMenu.editor.activeFocus)
            fieldMenu.editor.forceActiveFocus()
        fieldMenu.hadCaret = false
    }

    /// Opens on the box as it stands: under the hand where the hand is on the box — a right-click, opened where every
    /// right-click menu opens — and under the caret otherwise, the menu key's. Not from the request's position: Qt moves
    /// every request a text box answers to its caret, a right-click's too (`QQuickTextInput::contextMenuEvent` /
    /// `QQuickTextEdit::contextMenuEvent`). Says whether it opened.
    function offerNow() {
        const box = fieldMenu.editor
        const chosen = box.selectedText !== ""
        fieldMenu.hadCaret = box.activeFocus
        fieldMenu.typing = !box.readOnly
        fieldMenu.canUndo = box.canUndo
        fieldMenu.canRedo = box.canRedo
        fieldMenu.canCut = chosen
        fieldMenu.canCopy = chosen
        fieldMenu.canPaste = box.canPaste
        fieldMenu.canSelectAll = box.selectionStart > 0 || box.selectionEnd < box.length
        // A headless window's pointer rests on its corner and answers nothing (`Hand.away`).
        if (box.hovered && !Hand.away)
            return fieldMenu.offer()
        const caret = box.cursorRectangle
        return fieldMenu.offerAt(Qt.point(caret.x, caret.y + caret.height))
    }

    /// Automation: the rows on the table, in order, as the words they show — the ones that can be pressed only, when
    /// `live` is set.
    function rowWords(live) {
        const words = []
        for (let i = 0; i < fieldMenu.count; i++) {
            const row = fieldMenu.itemAt(i)
            if (row && row.offered && row.codeColSeat !== undefined && (!live || row.enabled))
                words.push(row.text)
        }
        return words
    }

    AppMenuItem {
        text: qsTr("Undo")
        offered: fieldMenu.typing
        enabled: fieldMenu.canUndo
        onTriggered: fieldMenu.editor.undo()
    }
    AppMenuItem {
        text: qsTr("Redo")
        offered: fieldMenu.typing
        enabled: fieldMenu.canRedo
        onTriggered: fieldMenu.editor.redo()
    }
    AppMenuSeparator {}
    AppMenuItem {
        text: qsTr("Cut")
        offered: fieldMenu.typing
        enabled: fieldMenu.canCut
        onTriggered: fieldMenu.editor.cut()
    }
    AppMenuItem {
        text: qsTr("Copy")
        enabled: fieldMenu.canCopy
        onTriggered: fieldMenu.editor.copy()
    }
    AppMenuItem {
        text: qsTr("Paste")
        offered: fieldMenu.typing
        enabled: fieldMenu.canPaste
        onTriggered: fieldMenu.editor.paste()
    }
    AppMenuItem {
        text: qsTr("Delete")
        offered: fieldMenu.typing
        enabled: fieldMenu.canCut
        onTriggered: fieldMenu.editor.remove(fieldMenu.editor.selectionStart, fieldMenu.editor.selectionEnd)
    }
    AppMenuSeparator {}
    AppMenuItem {
        text: qsTr("Select all")
        enabled: fieldMenu.canSelectAll
        onTriggered: fieldMenu.editor.selectAll()
    }
}
