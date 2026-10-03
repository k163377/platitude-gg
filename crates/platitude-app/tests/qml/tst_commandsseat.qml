import QtQuick
import QtTest
import platitude.ui

// The command log's seat in the panel's band carries its `(N)`: a press on the count closes the log as a press on the
// name does, and the hover's ground runs as far past the count as it starts before the mark.
Item {
    id: root
    width: 400
    height: 60

    QtObject {
        id: page
        property var pageCommands: null
        property bool commandsWrong: false
        property bool commandsOpen: true
        property int toggled: 0
        function toggleCommands() {
            page.toggled++
        }
    }

    CommandsToggle {
        id: seat
        captioned: true
        ruled: false
        counted: true
        count: 12
        curPage: page
    }

    TestCase {
        name: "CommandsSeat"
        when: windowShown

        function countLabel() {
            for (const child of seat.children) {
                if (child.text === "(12)")
                    return child
            }
            fail("no `(12)` in the seat")
        }
        function hand() {
            for (const child of seat.children) {
                if (child instanceof MouseArea)
                    return child
            }
            fail("no MouseArea in the seat")
        }

        function init() {
            page.toggled = 0
        }

        function test_a_press_on_the_count_closes_the_log() {
            const label = countLabel()
            mouseClick(seat, label.x + label.width / 2, seat.height / 2)
            compare(page.toggled, 1)
        }

        /// The control: a press just past the seat's end reaches nothing.
        function test_a_press_past_the_seat_closes_nothing() {
            mouseClick(root, seat.width + 2, seat.height / 2)
            compare(page.toggled, 0)
        }

        function test_a_hand_on_the_count_lights_the_seat() {
            const label = countLabel()
            mouseMove(root, seat.width + 2, seat.height / 2)
            tryVerify(() => !hand().containsMouse)
            mouseMove(seat, label.x + label.width / 2, seat.height / 2)
            tryVerify(() => hand().containsMouse)
        }

        function test_the_ground_ends_as_far_past_the_count_as_it_starts_before_the_mark() {
            const label = countLabel()
            verify(seat.markX > 0)
            compare(seat.width - (label.x + label.implicitWidth), seat.markX)
        }
    }
}
