import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The right-click menu of the boxes words are typed or read in (`FieldMenu`, built by `FieldMenuSeat`).
// - Every box the product draws hands its right-click to the product's menu, and the style's own
//   (`TextEditingContextMenu`) is never built: a box that left `ContextMenu.menu` unwritten would open both.
// - The rows are a fixed table (デザイン規約 §メニュー の文字の欄): the ones that change the words go on a read-only
//   box, the ones the box cannot do right now stay greyed and take no click, and each lit row does what it says.
// - Qt names a text box's caret as the place of every request it answers, a right-click's too, so the card reads the
//   hand instead: under the hand where it is on the box (a right-click), under the caret where it is not (the menu
//   key), and under the caret in a headless window whatever the hand (`Hand.away`).
Item {
    id: root
    width: 640
    height: 560

    Column {
        y: 40
        spacing: 8
        // The control: a box of the style's, which carries the style's menu.
        TextField {
            id: styled
            width: 300
            text: "styled"
        }
        FormField {
            id: formBox
            width: 300
            text: "form field"
        }
        SlimField {
            id: slimBox
            width: 300
            text: "slim field"
        }
        SummaryArea {
            id: summaryBox
            width: 300
            text: "a summary"
        }
        SidebarFilterRow {
            id: filterBox
            width: 300
            text: "filter"
        }
        DescriptionBox {
            id: descBox
            width: 300
            height: 80
            text: "a description"
        }
        // The two kinds of box a menu is handed, each with a seat of its own, for the rows. Hover written in: the default
        // follows the platform's hover effects, which the offscreen platform has none of.
        TextField {
            id: line
            width: 300
            hoverEnabled: true
            FieldMenuSeat {
                id: lineSeat
                editor: line
            }
        }
        TextArea {
            id: page
            width: 300
            height: 60
            hoverEnabled: true
            FieldMenuSeat {
                id: pageSeat
                editor: page
            }
        }
    }

    // The pair the commit editor and the details pane write a message in: `anyFocused` is what keeps amend's row up.
    MessageEditor {
        id: message
        x: 330
        y: 40
        width: 300
        height: 220
    }

    ClipboardHelper {
        id: board
    }
    SignalSpy {
        id: requests
        signalName: "requested"
    }

    TestCase {
        name: "FieldMenu"
        when: windowShown

        function init() {
            for (const seat of [lineSeat, pageSeat])
                if (seat.item)
                    seat.item.close()
            Hand.away = false
            mouseMove(root, root.width - 2, root.height - 2)
            line.readOnly = false
            page.readOnly = false
            line.text = ""
            page.text = ""
            root.forceActiveFocus()
            board.copy("pasted")
        }
        // `Hand` is one for the whole run: a file that leaves it away hands the files after it a window with no hand.
        function cleanup() {
            Hand.away = false
        }

        /// The box the right-click lands on inside `item`: the item itself, or the one text box in it.
        function editorIn(item) {
            if (item.canPaste !== undefined)
                return item
            for (let i = 0; i < item.children.length; i++) {
                const found = editorIn(item.children[i])
                if (found)
                    return found
            }
            return null
        }
        /// Every text box inside `item`, in the order they are declared.
        function editorsIn(item, found) {
            if (item.canPaste !== undefined)
                found.push(item)
            for (let i = 0; i < item.children.length; i++)
                editorsIn(item.children[i], found)
            return found
        }
        function seatOf(editor) {
            for (let i = 0; i < editor.children.length; i++)
                if (editor.children[i].editor === editor)
                    return editor.children[i]
            return null
        }

        function typeInto(box, words) {
            box.forceActiveFocus()
            for (const letter of words)
                keyClick(letter)
        }
        /// Opens the menu and answers the rows that can be pressed.
        function rowsOn(seat) {
            verify(seat.offer(), "the menu opened")
            tryCompare(seat.item, "opened", true)
            return seat.item.rowWords(true)
        }
        function rowOn(seat, words) {
            for (let i = 0; i < seat.item.count; i++) {
                const row = seat.item.itemAt(i)
                if (row && row.text === words)
                    return row
            }
            return null
        }
        function press(seat, words) {
            for (let i = 0; i < seat.item.count; i++) {
                const row = seat.item.itemAt(i)
                if (row && row.text === words) {
                    row.triggered()
                    return
                }
            }
            fail("no row says " + words)
        }

        function test_the_style_carries_a_menu_of_its_own() {
            verify(styled.ContextMenu.menu !== null, "the style's box carries the style's menu")
        }

        function test_a_right_click_on_any_box_opens_the_products_menu_data() {
            return [
                { tag: "FormField", box: formBox },
                { tag: "SlimField", box: slimBox },
                { tag: "SummaryArea", box: summaryBox },
                { tag: "SidebarFilterRow", box: filterBox },
                { tag: "DescriptionBox", box: descBox },
            ]
        }
        function test_a_right_click_on_any_box_opens_the_products_menu(data) {
            const editor = editorIn(data.box)
            verify(editor !== null, "the box holds a text box")
            const seat = seatOf(editor)
            verify(seat !== null, "the text box has a menu seat")
            mouseClick(editor, 20, 8, Qt.RightButton)
            // The product's menu opened.
            tryVerify(() => seat.item !== null && seat.item.opened)
            compare(editor.ContextMenu.menu, null, "the style's menu was never built")
            seat.item.close()
            tryCompare(seat.item, "visible", false)
        }

        function test_the_rows_are_what_the_box_can_do_now_data() {
            return [
                { tag: "line", box: line, seat: lineSeat },
                { tag: "page", box: page, seat: pageSeat },
            ]
        }
        function test_the_rows_are_what_the_box_can_do_now(data) {
            const box = data.box
            const seat = data.seat
            const table = ["Undo", "Redo", "Cut", "Copy", "Paste", "Delete", "Select all"]
            compare(rowsOn(seat), ["Paste"], "empty: only what the clipboard holds is lit")
            compare(seat.item.rowWords(false), table, "a box that takes typing lays the whole table out")
            seat.item.close()
            typeInto(box, "abc")
            compare(rowsOn(seat), ["Undo", "Paste", "Select all"], "typed into, nothing selected")
            seat.item.close()
            box.select(1, 3)
            compare(rowsOn(seat), ["Undo", "Cut", "Copy", "Paste", "Delete", "Select all"], "part selected")
            seat.item.close()
            box.selectAll()
            compare(rowsOn(seat), ["Undo", "Cut", "Copy", "Paste", "Delete"], "all selected")
            compare(seat.item.rowWords(false), table, "the rows that cannot be done stay on the table")
            seat.item.close()
            box.undo()
            box.deselect()
            compare(rowsOn(seat), ["Redo", "Paste"], "undone back to empty")
            seat.item.close()
            box.text = "read only"
            box.readOnly = true
            box.select(0, 4)
            compare(rowsOn(seat), ["Copy", "Select all"], "read only, part selected")
            compare(seat.item.rowWords(false), ["Copy", "Select all"], "a read-only box has no rows that change words")
            seat.item.close()
            box.deselect()
            compare(rowsOn(seat), ["Select all"], "read only, nothing selected")
            seat.item.close()
            box.text = ""
            compare(rowsOn(seat), [], "read only and empty: the table opens, every row greyed")
            compare(seat.item.rowWords(false), ["Copy", "Select all"])
        }

        /// A box being written in stays written in under its own menu, which takes the focus to answer the keys: amend's
        /// row stays up under it, Escape closes only the card, and the caret is back in the box as the card goes.
        function test_a_box_being_written_in_stays_so_under_its_menu() {
            message.setMessage("a summary", "a description")
            const desc = editorsIn(message, [])[1]
            message.focusDescription()
            tryVerify(() => desc.activeFocus)
            verify(message.anyFocused)
            const seat = seatOf(desc)
            mouseClick(desc, 20, 8, Qt.RightButton)
            tryVerify(() => seat.item !== null && seat.item.opened)
            verify(!desc.activeFocus, "the card has the focus")
            verify(message.anyFocused, "the message is still being written")
            keyClick(Qt.Key_Escape)
            tryVerify(() => !seat.item.visible)
            tryVerify(() => desc.activeFocus)
            verify(message.anyFocused, "Escape closed the card, not the writing")
        }

        function test_a_written_in_frame_keeps_its_focus_ring_under_its_menu() {
            formBox.forceActiveFocus()
            const seat = seatOf(formBox)
            mouseClick(formBox, 20, 8, Qt.RightButton)
            tryVerify(() => seat.item !== null && seat.item.opened)
            verify(!formBox.activeFocus)
            compare(formBox.background.border.color, Theme.borderFocus)
            seat.item.close()
            tryVerify(() => formBox.activeFocus)
        }

        function test_a_greyed_row_takes_no_click_data() {
            return test_the_rows_are_what_the_box_can_do_now_data()
        }
        function test_a_greyed_row_takes_no_click(data) {
            const box = data.box
            const seat = data.seat
            typeInto(box, "abc")
            rowsOn(seat)
            const cut = rowOn(seat, "Cut")
            verify(cut !== null && !cut.enabled, "Cut is greyed with nothing selected")
            // The card's list lays its rows out after it opens; until then every row stands at the top, on Undo.
            const undo = rowOn(seat, "Undo")
            tryVerify(() => cut.y > undo.y)
            mouseClick(cut)
            compare(box.text, "abc", "the greyed row did nothing")
            verify(seat.item.visible, "and the card stands, as it does under a press on its padding")
        }

        function test_each_row_does_what_it_says_data() {
            return test_the_rows_are_what_the_box_can_do_now_data()
        }
        function test_each_row_does_what_it_says(data) {
            const box = data.box
            const seat = data.seat
            typeInto(box, "abcd")
            box.select(1, 3)
            rowsOn(seat)
            press(seat, "Cut")
            compare(box.text, "ad")
            box.cursorPosition = 2
            rowsOn(seat)
            press(seat, "Paste")
            compare(box.text, "adbc", "Cut put what it took on the clipboard")
            box.select(0, 2)
            rowsOn(seat)
            press(seat, "Copy")
            box.cursorPosition = 4
            rowsOn(seat)
            press(seat, "Paste")
            compare(box.text, "adbcad")
            box.select(0, 2)
            rowsOn(seat)
            press(seat, "Delete")
            compare(box.text, "bcad")
            rowsOn(seat)
            press(seat, "Undo")
            compare(box.text, "adbcad")
            rowsOn(seat)
            press(seat, "Redo")
            compare(box.text, "bcad")
            box.deselect()
            rowsOn(seat)
            press(seat, "Select all")
            compare(box.selectedText, "bcad")
        }

        /// The premise the placement stands on: Qt moves a text box's request to its caret
        /// (`QQuickTextInput::contextMenuEvent` / `QQuickTextEdit::contextMenuEvent`). A Qt that hands over the click
        /// instead goes red here, and the hand need not be read any more.
        function test_a_text_boxs_request_names_its_caret_wherever_the_click_data() {
            return [
                { tag: "TextField", box: formBox },
                { tag: "TextArea", box: summaryBox },
            ]
        }
        function test_a_text_boxs_request_names_its_caret_wherever_the_click(data) {
            const box = data.box
            const seat = seatOf(box)
            requests.clear()
            requests.target = box.ContextMenu
            const caret = box.cursorRectangle
            const at = Qt.point(Math.round(caret.x + caret.width / 2), Math.round(caret.y + caret.height / 2))
            for (const x of [box.width - 20, box.width / 2]) {
                const before = requests.count
                mouseClick(box, x, box.height / 2, Qt.RightButton)
                // The right-click made a request.
                tryVerify(() => requests.count === before + 1)
                compare(requests.signalArguments[before][0], at, "named the caret, not x=" + x)
                seat.item.close()
                tryCompare(seat.item, "visible", false)
            }
            requests.target = null
        }

        function test_the_card_stands_under_the_hand_or_under_the_caret_data() {
            return test_the_rows_are_what_the_box_can_do_now_data()
        }
        function test_the_card_stands_under_the_hand_or_under_the_caret(data) {
            const box = data.box
            const seat = data.seat
            typeInto(box, "abc")
            const caret = box.cursorRectangle
            const underCaret = Qt.point(caret.x, caret.y + caret.height)
            verify(!box.hovered)
            verify(seat.offer())
            tryCompare(seat.item, "opened", true)
            compare(Qt.point(seat.item.x, seat.item.y), underCaret, "no hand on the box: the menu key's, under the caret")
            seat.item.close()
            tryCompare(seat.item, "visible", false)
            mouseMove(box, 40, 6)
            tryCompare(box, "hovered", true)
            verify(seat.offer())
            tryCompare(seat.item, "opened", true)
            // Where the platform's cursor is — the offscreen one never moves off the screen's corner — not the caret.
            verify(seat.item.x !== underCaret.x || seat.item.y !== underCaret.y, "a hand on the box: a right-click's")
            seat.item.close()
            tryCompare(seat.item, "visible", false)
            Hand.away = true
            verify(seat.offer())
            tryCompare(seat.item, "opened", true)
            compare(Qt.point(seat.item.x, seat.item.y), underCaret, "a headless window's hand answers nothing")
        }
    }
}
