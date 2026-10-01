import QtQuick
import QtTest
import platitude.ui

// A view toggle's half whose view is showing wears its mark in the accent, and nothing a hand brings: the wash, the tip
// and `lit` answer the hand alone (`HoverToolButton.lit`), on the showing half as on the other.
Item {
    id: root
    width: 200
    height: 60

    DiffViewToggle {
        id: toggle
        height: Theme.iconXl
        split: false
    }

    TestCase {
        name: "ViewToggle"
        when: windowShown

        /// The half showing its view: unified, while `split` is false.
        function shown() {
            const half = toggle.children[0]
            compare(half.tip, "Unified view")
            return half
        }
        function other() {
            const half = toggle.children[1]
            compare(half.tip, "Split view")
            return half
        }

        function init() {
            shown().pointedAt = false
            other().pointedAt = false
            tryVerify(() => Qt.colorEqual(shown().washColor, "transparent") && !shown().tipShown)
        }

        function test_the_showing_half_wears_no_wash_and_no_tip_without_a_hand() {
            // waits(paced): the subject is a tip that must **not** come up, so the wait is the delay it would come after.
            wait(Metrics.tipDelayMs * 2)
            verify(Qt.colorEqual(shown().washColor, "transparent"), "washed with no hand on it")
            verify(!shown().tipShown, "its tip came up with no hand on it")
        }

        /// The control: the test above would pass on a half that never washes.
        function test_a_hand_washes_either_half_and_brings_its_tip() {
            for (const half of [shown(), other()]) {
                half.pointedAt = true
                tryVerify(() => Qt.colorEqual(half.washColor, Theme.bgHover))
                tryVerify(() => half.tipShown)
                half.pointedAt = false
                tryVerify(() => Qt.colorEqual(half.washColor, "transparent") && !half.tipShown)
            }
        }

        /// `lit` is the button's own word for a hand on it, read from outside as from inside.
        function test_the_showing_half_is_not_lit_without_a_hand() {
            verify(!shown().lit, "lit, read from outside, answered the view instead of the hand")
        }

        function test_the_marks_say_which_view_shows() {
            verify(Qt.colorEqual(shown().contentItem.tint, Theme.accent))
            verify(Qt.colorEqual(other().contentItem.tint, Theme.accentDim))
        }
    }
}
