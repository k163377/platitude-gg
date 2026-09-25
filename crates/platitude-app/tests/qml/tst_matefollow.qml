import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The name a chip draws under itself in the card it unfolds into (`RefChip.mate`), and where a press on it goes
// (デザイン規約 §グラフ行のダブルクリック): to the place it names, never to the row the chip stands in, whose own
// handler answers at the press (`RefListPopup`) — Qt's delivery order, so measured with a real pointer.
// Also the gesture's two ways of being told about a click made elsewhere (`ReclickGesture.land` / `hush`).
Item {
    id: root
    width: 400
    height: 200

    property var followed: []
    /// How often the band the chip stands in heard a press of its own.
    property int rowPresses: 0

    /// A record the card draws, and the line under it: the branch reading this remote-tracking ref.
    readonly property var record: ({ "kind": "remote", "name": "origin/main", "isHead": false, "hasRemote": false,
                                     "hasPr": false, "here": false, "held": false, "locked": false, "remote": "",
                                     "key": "remote:origin/main" })
    function mateGoing(to) {
        return { "mark": "branch", "markTint": Theme.accent, "text": "main", "tone": Theme.textSecondary,
                 "ahead": 0, "behind": 0, "note": "", "to": to }
    }

    // A row of the card, as far as a press goes: the band answers at the press, the way the card's rows do.
    Rectangle {
        id: band
        width: 300
        height: 60
        TapHandler {
            onPressedChanged: if (pressed) root.rowPresses++
        }
        RefChip {
            id: chip
            x: 4
            y: 4
            wrapped: true
            maxWidth: 280
            records: [root.record]
            mate: root.mateGoing({ "key": "branch:main", "oid": "abc" })
            onMateFollowed: to => root.followed = root.followed.concat([to.key])
        }
    }

    ReclickGesture {
        id: gesture
    }

    /// The items under `item` that turn the pointer to the hand — the one area a chip builds for its line.
    function handsUnder(item) {
        let found = item.cursorShape === Qt.PointingHandCursor ? [item] : []
        const kids = item.children
        if (kids !== undefined)
            for (let i = 0; i < kids.length; i++)
                found = found.concat(root.handsUnder(kids[i]))
        return found
    }

    TestCase {
        name: "MateFollow"
        when: windowShown

        function init() {
            root.followed = []
            root.rowPresses = 0
            chip.mate = root.mateGoing({ "key": "branch:main", "oid": "abc" })
        }

        function handOf() {
            const hands = root.handsUnder(chip)
            compare(hands.length, 1, "one hand, over the name under the chip's own")
            return hands[0]
        }

        function test_a_press_on_the_line_goes_there_and_not_to_the_row() {
            const hand = handOf()
            tryVerify(() => hand.width > 0, undefined, "the hand is laid out")
            compare(hand.width, chip.width - 2 * Theme.borderWidth, "across the frame, measure included")
            mouseClick(hand, hand.width / 2, hand.height / 2)
            mouseClick(hand, hand.width - 2, hand.height / 2)
            compare(root.followed, ["branch:main", "branch:main"], "the middle and the far end both go there")
            compare(root.rowPresses, 0, "the row's own handler was never pressed")
        }

        function test_a_press_on_the_chip_itself_is_the_rows() {
            mouseClick(chip, 4, 4)
            compare(root.followed, [])
            compare(root.rowPresses, 1)
        }

        function test_the_hand_on_the_name_is_answered() {
            const hand = handOf()
            tryVerify(() => hand.width > 0)
            mouseMove(hand, hand.width - 2, hand.height / 2)
            tryVerify(() => chip.mateLit, undefined, "the band is laid under the line, far end included")
            mouseMove(root, root.width - 2, root.height - 2)
            tryVerify(() => !chip.mateLit)
        }

        /// The chip stands on every row of the graph, and an area built and hidden on each of them is heap.
        function test_a_name_going_nowhere_builds_no_hand() {
            chip.mate = root.mateGoing(null)
            compare(root.handsUnder(chip).length, 0)
            verify(!chip.mateGoes)
        }

        function test_a_landing_is_the_last_click_and_arms_nothing() {
            gesture.land("branch:main")
            compare(gesture.activeKey, "branch:main")
            verify(!gesture.armed, "nothing is waiting to become a name box")
            verify(gesture.click("branch:main", { "id": "main" }), "the next press on that row is a press")
            verify(gesture.armedFor("branch:main"), "and the second one on it, which arms the box")
            gesture.forget()
        }

        /// After a hush every press is the rest of a gesture made elsewhere, for one double-click window.
        function test_a_hush_swallows_the_rest_of_the_gesture_and_then_lets_go() {
            gesture.hush()
            verify(gesture.hushed)
            verify(!gesture.click("remote:origin/main", null), "a press in the window is not a press here")
            tryVerify(() => !gesture.hushed, undefined, "the window runs out")
            verify(gesture.click("remote:origin/main", null), "and the next press is one again")
            gesture.forget()
        }
    }
}
