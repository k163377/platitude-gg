import QtQuick
import QtTest

// What Qt hands a text hand laid over the rows on a double-click (`DiffTextSelect.wordAt`, `CommandsTextSelect.wordAt`):
// the order of its signals, what reaches it where it turned the second press down, and why the diff's right button has
// an area of its own. The top half is stacked as `DiffPane` stacks it — a passive sheet on top (`pressDoor`), the
// middle button's area (`MiddleAutoScroll`), the right button's area over the left's, and a button under them standing
// for what the hand leaves its presses to (a gutter's `+`, a heading's words). The bottom half is one area taking both
// buttons with a double-click connected.
Item {
    id: root
    width: 300
    height: 200

    /// The hands' signals in the order they came, as `<signal>:<x>`.
    property var said: []
    /// Presses the hand turns down: left of this x, as the gutter is.
    readonly property real refusedLeftOf: 100
    property int under: 0

    function heard(what, mouse) {
        root.said = root.said.concat([what + ":" + Math.round(mouse.x)])
    }

    Item {
        id: rows
        width: root.width
        height: 100

        MouseArea {
            id: below
            anchors.fill: parent
            onPressed: root.under++
        }
        MouseArea {
            id: hand
            anchors.fill: parent
            onPressed: mouse => {
                mouse.accepted = mouse.x >= root.refusedLeftOf
                root.heard(mouse.accepted ? "pressed" : "refused", mouse)
            }
            onReleased: mouse => root.heard("released", mouse)
            onClicked: mouse => root.heard("clicked", mouse)
            onDoubleClicked: mouse => root.heard("doubled", mouse)
        }
        MouseArea {
            anchors.fill: parent
            acceptedButtons: Qt.RightButton
            onClicked: mouse => root.heard("right", mouse)
        }
        MouseArea {
            anchors.fill: parent
            acceptedButtons: Qt.MiddleButton
        }
        Item {
            anchors.fill: parent
            PointHandler {
                acceptedButtons: Qt.AllButtons
            }
        }
    }
    MouseArea {
        id: both
        y: 100
        width: root.width
        height: 100
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onClicked: mouse => root.heard(mouse.button === Qt.RightButton ? "right" : "clicked", mouse)
        onDoubleClicked: mouse => root.heard("doubled", mouse)
    }

    TestCase {
        name: "TextDoubleClick"
        when: windowShown

        function init() {
            root.said = []
            root.under = 0
            // Spaced from the test before: inside the window the first press would be half a double-click.
            wait(Qt.styleHints.mouseDoubleClickInterval + 50) // waits(paced): the platform's double-click window
        }

        function test_the_double_comes_after_the_second_press_and_no_click_follows_it() {
            mouseDoubleClickSequence(rows, 150, 50, Qt.LeftButton)
            compare(root.said, ["pressed:150", "released:150", "clicked:150",
                                "pressed:150", "doubled:150", "released:150"],
                    "the second press is the hand's before the double, so the word is laid over its empty start")
        }

        function test_a_press_turned_down_is_not_followed_by_a_double() {
            mouseDoubleClickSequence(rows, 50, 50, Qt.LeftButton)
            compare(root.said, ["refused:50", "refused:50"],
                    "only presses reach the hand where it turned them down")
            compare(root.under, 2, "both presses went on to what lies under it")
        }

        function test_a_press_nobody_takes_is_not_followed_by_a_double_either() {
            below.enabled = false
            mouseDoubleClickSequence(rows, 50, 50, Qt.LeftButton)
            below.enabled = true
            compare(root.said, ["refused:50", "refused:50"])
        }

        function test_an_area_with_a_double_connected_swallows_the_second_click_of_any_button() {
            mouseDoubleClickSequence(both, 150, 50, Qt.RightButton)
            compare(root.said, ["right:150", "doubled:150"],
                    "a quick second right-click raises no menu on an area that also takes the left's double")
        }

        function test_the_right_button_on_its_own_area_clicks_twice() {
            mouseDoubleClickSequence(rows, 150, 50, Qt.RightButton)
            compare(root.said, ["right:150", "right:150"], "and the left's area hears nothing of it")
        }
    }
}
