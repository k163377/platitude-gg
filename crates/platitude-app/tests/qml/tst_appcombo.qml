import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// Where a name too long for the box stops, and that it is short of the arrow.
//
// Only a laid-out field can answer: the overlap is the input's right inset against the chevron's seat, and a
// one-pixel overlap is not something a headless PNG can be trusted to show.
Item {
    id: root
    width: 400
    height: 200

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
