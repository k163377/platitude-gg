import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// Where a name too long for the box stops, and that it is short of the arrow rather than under it.
//
// **Only a built field can answer it.** The box scrolls its text rather than eliding it, so what decides the overlap
// is the inset the input keeps at its right edge against the seat the chevron actually stands in — two geometries
// that only exist once the control has been laid out. Nothing in Rust reaches either, and a picture of a field whose
// last letter is half under a glyph is exactly the kind of one-pixel judgement a headless PNG cannot be trusted with
// (verify-ui §目視).
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

    /// The same field with nothing to open, which is the one shape that has no arrow to keep clear of.
    AppCombo {
        id: bare
        width: 160
        y: 40
    }

    /// The picking shape: the value is the owner's and no caret can reach it, so a name too long for the box has
    /// nowhere to scroll to and is cut instead.
    AppCombo {
        id: picked
        pickOnly: true
        width: 160
        y: 80
        model: ["origin", "upstream"]
        wanted: root.longName
    }

    TestCase {
        name: "AppCombo"
        when: windowShown

        /// The x, in the field's own coordinates, past which the input draws nothing.
        function textRight(combo) {
            const input = combo.contentItem
            return input.x + input.width - input.rightPadding
        }

        /// The premise the field's own inset is worked out from: the control keeps the indicator's width back and
        /// nothing else, so the seat being pushed further in than that is the whole of what the word has to clear.
        /// Here rather than in a comment, because it is a fact about the style underneath and not about this code.
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

        /// The caret is what the reader is following while they type, so the end of the name has to be readable at
        /// the inset — not under the glyph, and not off the end of a box that never scrolled.
        function test_the_caret_at_the_end_stays_off_the_arrow() {
            typed.contentItem.forceActiveFocus()
            typed.contentItem.cursorPosition = root.longName.length
            const caret = typed.contentItem.cursorRectangle
            compare(typed.editText, root.longName, "and the value is all of it, cut nowhere")
            verify(caret.x + typed.contentItem.x <= typed.indicator.x,
                   "the caret sits at " + (caret.x + typed.contentItem.x)
                     + ", the arrow's seat starts at " + typed.indicator.x)
        }

        /// **A picked name is cut, not scrolled.** Nothing can put a caret in this field, so a text too wide for it
        /// would sit at whatever offset it was left at — which on a fresh field is the end, with the head of the name
        /// off the left edge and no mark saying any of it was taken. It is cut in the middle instead, keeping both of
        /// the ends a remote is told apart by.
        function test_a_picked_name_too_long_for_the_box_is_cut_rather_than_scrolled() {
            const shown = picked.contentItem.text
            verify(shown.length < root.longName.length, "something was taken out of it: " + shown)
            verify(shown.indexOf("…") > 0, "and the mark says so: " + shown)
            verify(shown.startsWith(root.longName.slice(0, 4)),
                   "the head the reader looks for is still there: " + shown)
            verify(shown.endsWith(root.longName.slice(-4)), "and so is the tail: " + shown)
            compare(picked.wanted, root.longName, "while the value itself is untouched")
        }

        /// And what is drawn fits: the cut is made against the room the word has, so the ink stops inside the box and
        /// short of the arrow rather than being clipped by the frame.
        function test_a_picked_name_stops_inside_the_box() {
            verify(picked.contentItem.contentWidth
                       <= picked.width - picked.contentItem.leftPadding - picked.contentItem.rightPadding + 1,
                   "drawn " + picked.contentItem.contentWidth + " in a box of " + picked.width)
            verify(textRight(picked) <= picked.indicator.x,
                   "the text stops at " + textRight(picked) + ", the arrow's seat starts at " + picked.indicator.x)
        }

        /// Nothing to open, nothing to keep clear of: the field falls back to the plain frame-to-word inset every
        /// other box keeps, so a chooser-less field does not lose a quarter of its width to an arrow it never draws.
        function test_a_field_with_no_list_keeps_the_plain_inset() {
            verify(!bare.indicator.visible, "no list, no arrow")
            compare(bare.contentItem.rightPadding, bare.contentItem.leftPadding,
                    "so both insets are the one every boxed field shares")
        }
    }
}
