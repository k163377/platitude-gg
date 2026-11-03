import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import QtTest
import platitude.ui

// Where a name too long for the box stops, and that it is short of the arrow.
//
// Only a laid-out field can answer: the overlap is the input's right inset against the chevron's seat, and a
// one-pixel overlap is not something a headless PNG can be trusted to show.
//
// A control, to hand the fields the window's font as `Main` does: the field cuts with its own font's ellipsis, and a
// default face without one cuts with three dots.
Control {
    id: root
    width: 400
    height: 260
    font.family: Theme.uiFamily
    font.pixelSize: Theme.fontMd

    readonly property string longName: "origin/feature/a-name-far-too-long-for-this-box"

    AppCombo {
        id: typed
        width: 160
        model: ["origin", "upstream"]
        wanted: root.longName
    }

    /// Nothing to open, so no arrow to keep clear of.
    AppCombo {
        id: bare
        width: 160
        y: 40
    }

    /// The picking shape: a name too long for the box is cut, not scrolled.
    AppCombo {
        id: picked
        pickOnly: true
        width: 160
        y: 80
        model: ["origin", "upstream"]
        wanted: root.longName
    }

    /// The picking shape holding a remote (the first push's and the upstream's destination): its head is the remote.
    AppCombo {
        id: pickedRemote
        pickOnly: true
        leadMark: "remote"
        width: 160
        y: 160
        model: ["origin", "upstream"]
        wanted: root.longName
    }

    /// Two narrow fields in an item their lists may spread across (`spreadWithin`): one in its left half, one in its
    /// right half, with a row far wider than either box.
    Item {
        id: spreadRoom
        x: 200
        width: 200
        height: 40
        readonly property var names: ["origin", "upstream-mirror-two"]
        AppCombo {
            id: spreadsRight
            pickOnly: true
            width: 70
            model: spreadRoom.names
            wanted: "origin"
            spreadWithin: spreadRoom
        }
        AppCombo {
            id: spreadsLeft
            pickOnly: true
            x: 130
            width: 70
            model: spreadRoom.names
            wanted: "origin"
            spreadWithin: spreadRoom
        }
    }

    /// A list whose widest row wears the push mark (`markedRow`), in an item it may spread across. Short names: the
    /// ruler's reading over the laid-out text grows with the name and would cover a lane the width leaves out.
    Item {
        id: markRoom
        y: 220
        width: 400
        height: 40
        AppCombo {
            id: markedList
            pickOnly: true
            width: 30
            model: ["main", "fork"]
            markedRow: "fork"
            wanted: "main"
            spreadWithin: markRoom
        }
    }

    /// Two boxes holding a typed name too long for them, as the question bar's forms hold one: a list's and a plain
    /// one.
    AppCombo {
        id: typedName
        leadMark: "branch"
        x: 200
        y: 60
        width: 150
        model: ["main"]
        wanted: root.longName
    }
    SlimField {
        id: plainName
        leadMark: "branch"
        x: 200
        y: 100
        width: 150
        // Set, as the form's row sets it: a field's `baselineOffset` adds the centring only for a height that was set.
        height: Theme.controlHeight
        text: root.longName
    }
    /// A picked value still empty — its list not read yet — in a row that sizes the field by what it shows, as the
    /// settings' line endings do.
    Component {
        id: emptyRow
        RowLayout {
            width: 200
            AppCombo {
                Layout.fillWidth: true
                pickOnly: true
            }
        }
    }
    /// The ruler that agrees with the laid-out text, as the field's own cut measures (`WordFace.cutToFit`).
    TextMetrics {
        id: ruler
        font: picked.font
    }

    /// A list with rows to walk, so Enter has both a row to land on and a name to finish.
    AppCombo {
        id: keys
        width: 160
        y: 120
        model: ["origin/main", "origin/taken", "origin/carried"]
        wanted: "origin/main"
    }

    TestCase {
        id: tc
        name: "AppCombo"
        when: windowShown

        property int submits: 0
        /// `wanted` at the moment Qt raised `accepted` — what a handler on `accepted` would have sent.
        property string acceptedWanted: ""

        // Named through the id: a `Connections` function sees the target's scope, not the test's.
        Connections {
            target: keys
            function onSubmitted() { tc.submits++ }
            function onAccepted() { tc.acceptedWanted = keys.wanted }
        }

        function init() {
            tc.submits = 0
            tc.acceptedWanted = ""
            keys.popup.close()
        }

        /// The x, in the field's own coordinates, past which the input draws nothing.
        function textRight(combo) {
            const input = combo.contentItem
            return input.x + input.width - input.rightPadding
        }

        /// The premise the inset is worked out from: the control keeps back the indicator's width and no more.
        /// The premise is the style's, so a test goes red when it changes.
        function test_the_control_reserves_the_indicators_width_and_no_more() {
            const input = typed.contentItem
            compare(typed.width - (input.x + input.width), typed.indicator.width)
            verify(typed.indicator.x < input.x + input.width,
                   "and the seat starts inside that, which is why the inset is not zero")
        }

        function test_a_long_name_stops_short_of_the_arrow() {
            const seat = typed.indicator
            verify(seat.visible, "the field has a list, so the arrow is up")
            verify(typed.contentItem.contentWidth > typed.width,
                   "the name is longer than the box, which is what puts its end at the inset")
            verify(textRight(typed) <= seat.x,
                   "the text stops at " + textRight(typed) + ", the arrow's seat starts at " + seat.x)
        }

        /// The word ends `spaceXs` short of the arrow's ink, the step the ink keeps from the frame — the seat's air
        /// beside the ink not added on (`AppCombo`'s `rightPadding`).
        function test_the_word_ends_a_step_short_of_the_arrows_ink() {
            fuzzyCompare(typed.indicator.x + typed.indicator.inkLeft - textRight(typed), Theme.spaceXs, 0.01,
                         "the gap between the word and the arrow's ink")
        }

        /// A cut value fills its room: one letter more on either side it is cut from would not fit
        /// (`WordFace.cutToFit`) — not Qt's middle elide, which stops up to two letters short and leaves a gap before
        /// the arrow.
        function widthOf(word) {
            ruler.text = word
            return ruler.advanceWidth
        }

        function test_a_cut_value_fills_its_room() {
            const room = picked.contentItem.width - picked.contentItem.leftPadding - picked.contentItem.rightPadding
            const shown = picked.contentItem.text
            const cut = shown.indexOf("…")
            verify(cut > 0, "cut in the middle: " + shown)
            const head = shown.slice(0, cut)
            const tail = shown.slice(cut + 1)
            verify(widthOf(shown) <= room, "inside its room")
            fuzzyCompare(picked.contentItem.contentWidth, widthOf(shown), 0.5, "measured as it is laid out")
            verify(widthOf(root.longName.slice(0, head.length + 1) + "…" + tail) > room,
                   "one more letter at the head would not fit: " + shown)
            verify(widthOf(head + "…" + root.longName.slice(root.longName.length - tail.length - 1)) > room,
                   "nor one more at the tail: " + shown)

            const remoteRoom = pickedRemote.contentItem.width - pickedRemote.contentItem.leftPadding
                - pickedRemote.contentItem.rightPadding
            const remote = pickedRemote.contentItem.text
            verify(widthOf(root.longName.slice(0, remote.length) + "…") > remoteRoom,
                   "a remote's cut at its end fills its room too: " + remote)
        }

        /// An empty value has nothing to cut, even in a field not laid out yet, with no room at all: what the field
        /// shows stays empty, so its width does not swing between the two and loop the row's binding.
        function test_an_empty_value_is_never_cut() {
            failOnWarning(/Binding loop/)
            compare(WordFace.cutToFit(widthOf, "", -8, false), "")
            const row = createTemporaryObject(emptyRow, root)
            tryVerify(() => row.children[0].width > 0)
            compare(row.children[0].contentItem.text, "")
        }

        /// A typed name with nobody typing in it is shown cut, on the input's own line; typing, it is whole again.
        function test_a_typed_name_is_cut_only_while_nobody_types_in_it() {
            keys.contentItem.forceActiveFocus()
            for (const box of [typedName, plainName]) {
                verify(box.idleCut, "nobody is typing in it, so it is cut")
                verify(box.idleLabel.text.indexOf("…") > 0, "in its middle: " + box.idleLabel.text)
                const line = box.idleLabel.mapToItem(box, 0, box.idleLabel.baselineOffset).y
                const field = box.contentItem === undefined || box.contentItem === null
                    ? box.baselineOffset : box.contentItem.mapToItem(box, 0, box.contentItem.baselineOffset).y
                fuzzyCompare(line, field, 0.5, "the cut name stands on the field's own line: "
                             + (box === typedName ? "the list's box" : "the plain box"))
            }
            typedName.contentItem.forceActiveFocus()
            verify(!typedName.idleCut, "typing in the list's box, it scrolls whole")
            plainName.forceActiveFocus()
            verify(!plainName.idleCut, "and in the plain one")
            keys.contentItem.forceActiveFocus()
        }

        /// A hand on a cut name asks for the whole of it.
        function test_a_hand_on_a_cut_name_asks_for_the_whole() {
            keys.contentItem.forceActiveFocus()
            mouseMove(root, typedName.x + typedName.width / 2, typedName.y + typedName.height / 2)
            tryVerify(() => typedName.cutTip.asked, undefined, "the list's box asks")
            compare(typedName.cutTip.words, root.longName)
            mouseMove(root, plainName.x + plainName.width / 2, plainName.y + plainName.height / 2)
            tryVerify(() => plainName.cutTip.asked, undefined, "the plain box asks")
            mouseMove(root, root.width - 1, root.height - 1)
        }

        /// The box's right-click menu takes the focus to answer the keys, but the name is still being typed in it: it
        /// scrolls whole under the menu (`FieldMenuSeat.holding`).
        function test_a_typed_name_stays_whole_under_its_menu() {
            plainName.forceActiveFocus()
            let seat = null
            for (const child of plainName.children)
                if (child.editor === plainName)
                    seat = child
            verify(seat !== null, "the box's menu seat")
            mouseClick(plainName, plainName.width / 2, plainName.height / 2, Qt.RightButton)
            tryVerify(() => seat.item !== null && seat.item.opened)
            verify(!plainName.activeFocus, "the card has the focus")
            verify(!plainName.idleCut, "and the name is still being typed")
            seat.item.close()
            tryVerify(() => plainName.activeFocus)
            mouseMove(root, root.width - 1, root.height - 1)
            keys.contentItem.forceActiveFocus()
        }

        /// A list wider than its field spreads across `spreadWithin`: right from a field in its left half, left from
        /// one in its right half, and never past the item.
        function test_a_list_spreads_away_from_the_side_its_field_stands_on() {
            spreadsRight.popup.open()
            tryCompare(spreadsRight.popup, "visible", true)
            verify(spreadsRight.popup.width > spreadsRight.width,
                   "the list is " + spreadsRight.popup.width + " for a field of " + spreadsRight.width)
            compare(spreadsRight.popup.x, 0, "it starts under the field's left edge")
            verify(spreadsRight.x + spreadsRight.popup.width <= spreadRoom.width, "and ends inside the item")
            spreadsRight.popup.close()

            spreadsLeft.popup.open()
            tryCompare(spreadsLeft.popup, "visible", true)
            compare(spreadsLeft.popup.width, spreadsRight.popup.width, "the same rows, the same width")
            compare(spreadsLeft.popup.x + spreadsLeft.popup.width, spreadsLeft.width,
                    "it ends under the field's right edge")
            verify(spreadsLeft.x + spreadsLeft.popup.x >= 0, "and starts inside the item")
            spreadsLeft.popup.close()
        }

        /// Rows wider than the item itself: the list takes the item's width and no more.
        function test_a_list_wider_than_its_item_stops_at_the_item() {
            spreadRoom.width = 100
            spreadsLeft.x = 30
            spreadsLeft.popup.open()
            tryCompare(spreadsLeft.popup, "visible", true)
            compare(spreadsLeft.popup.width, spreadRoom.width)
            compare(spreadsLeft.x + spreadsLeft.popup.x, 0, "held at the item's left edge")
            spreadsLeft.popup.close()
            spreadRoom.width = 200
            spreadsLeft.x = 130
        }

        /// The widest row wearing the push mark is whole too: the list's width counts the mark's step off the thumb's
        /// lane, where the row seats it.
        function test_a_marked_widest_row_is_whole() {
            markedList.popup.open()
            tryCompare(markedList.popup, "visible", true)
            let rows = null
            for (const child of markedList.popup.contentItem.children)
                if (child.itemAtIndex !== undefined)
                    rows = child
            tryVerify(() => rows.itemAtIndex(1) !== null)
            const word = rows.itemAtIndex(1).contentItem.children[0]
            const mark = rows.itemAtIndex(1).contentItem.children[1]
            compare(word.text, "fork")
            verify(mark.visible, "the row wears the mark")
            verify(!word.truncated, "the word is whole in " + word.width + " for " + word.implicitWidth)
            markedList.popup.close()
        }

        function test_the_caret_at_the_end_stays_off_the_arrow() {
            typed.contentItem.forceActiveFocus()
            typed.contentItem.cursorPosition = root.longName.length
            const caret = typed.contentItem.cursorRectangle
            compare(typed.editText, root.longName, "and the value is all of it, cut nowhere")
            verify(caret.x + typed.contentItem.x <= typed.indicator.x,
                   "the caret sits at " + (caret.x + typed.contentItem.x)
                     + ", the arrow's seat starts at " + typed.indicator.x)
        }

        /// Cut in the middle: with no caret, an uncut text sits scrolled to its end, the head off the left edge and
        /// no mark saying so. The cut keeps both ends a remote is told apart by.
        function test_a_picked_name_too_long_for_the_box_is_cut_rather_than_scrolled() {
            const shown = picked.contentItem.text
            verify(shown.length < root.longName.length, "something was taken out of it: " + shown)
            verify(shown.indexOf("…") > 0, "and the mark says so: " + shown)
            verify(shown.startsWith(root.longName.slice(0, 4)),
                   "the head the reader looks for is still there: " + shown)
            verify(shown.endsWith(root.longName.slice(-4)), "and so is the tail: " + shown)
            compare(picked.wanted, root.longName, "while the value itself is untouched")
        }

        /// A remote's name gives way at its end, its head — the remote — kept (デザイン規約 §寸法「upstream 名だけは
        /// 末尾で切る」).
        function test_a_picked_remote_is_cut_at_its_end() {
            const shown = pickedRemote.contentItem.text
            verify(shown.endsWith("…"), "cut at the end: " + shown)
            verify(root.longName.startsWith(shown.slice(0, -1)), "the head kept whole: " + shown)
            compare(pickedRemote.wanted, root.longName, "while the value itself is untouched")
        }

        function test_a_picked_name_stops_inside_the_box() {
            verify(picked.contentItem.contentWidth
                       <= picked.width - picked.contentItem.leftPadding - picked.contentItem.rightPadding + 1,
                   "drawn " + picked.contentItem.contentWidth + " in a box of " + picked.width)
            verify(textRight(picked) <= picked.indicator.x,
                   "the text stops at " + textRight(picked) + ", the arrow's seat starts at " + picked.indicator.x)
        }

        function test_a_field_with_no_list_keeps_the_plain_inset() {
            verify(!bare.indicator.visible, "no list, no arrow")
            compare(bare.contentItem.rightPadding, bare.contentItem.leftPadding,
                    "so both insets are the one every boxed field shares")
        }

        /// Enter with the list shut finishes the name (デザイン規約 §立っている質問は 1 か所で聞く). Only a real
        /// keystroke can say that Qt raises `accepted` on it.
        function test_return_with_the_list_shut_is_a_finished_name() {
            keys.wanted = "origin/main"
            keys.popup.close()
            keys.contentItem.forceActiveFocus()
            keyClick(Qt.Key_Return)
            compare(tc.submits, 1)
            keyClick(Qt.Key_Enter)
            compare(tc.submits, 2, "and the keypad's own, which is a second key for the same word")
        }

        /// Under the open list the key belongs to the list and nothing is finished: Qt raises `accepted` on the
        /// press and picks the row on the release, so an answer hung off `accepted` carries the name the pick replaces.
        function test_return_under_the_open_list_picks_and_finishes_nothing() {
            keys.wanted = "origin/main"
            keys.contentItem.forceActiveFocus()
            keys.popup.open()
            tryCompare(keys.popup, "visible", true)
            keyClick(Qt.Key_Down)
            keyClick(Qt.Key_Return)
            compare(tc.submits, 0, "nothing was finished")
            compare(tc.acceptedWanted, "origin/main",
                    "and this is what a handler on `accepted` would have sent: the name before the pick")
            compare(keys.wanted, "origin/taken", "while the press itself picked the row the keyboard was on")
        }
    }
}
