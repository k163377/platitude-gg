import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// A press elsewhere takes the caret out of a box in the page, as `tst_fieldrelease` asks it of a dialog — the window's
// shape this time: the watcher over everything and the keyboard's home a focus scope (`Main` hands it `RepoPage`). A
// box that kept its caret keeps what stands while one is written in: the details pane's save row
// (`MessageActionsRow.editing`).
Item {
    id: root
    width: 520
    height: 320

    FocusScope {
        id: page
        anchors.fill: parent
        property int escapes: 0
        Keys.onEscapePressed: event => {
            page.escapes++
            event.accepted = true
        }

        // The details pane's summary: a box inside a scroll view (`MessageEditor`).
        ScrollView {
            x: 0
            y: 0
            width: 300
            height: 60
            SummaryArea {
                id: summary
                text: "Fix the thing"
            }
        }
        // A box with nothing between it and the page.
        FormField {
            id: field
            x: 0
            y: 100
            width: 200
        }
        // A box in a list's row (the graph's name box, the left menu's): the list is a focus scope of its own.
        ListView {
            id: list
            x: 0
            y: 160
            width: 300
            height: 40
            model: 1
            delegate: FormField {
                width: 200
            }
        }
        // Air: nothing is drawn there and nothing answers a press.
        Item {
            id: air
            x: 0
            y: 260
            width: 400
            height: 50
        }
    }

    FocusRelease {
        window: root.Window.window
        home: page
    }

    TestCase {
        name: "PageRelease"
        when: windowShown

        /// One line, so a failure says which half survived. `page=` is the page itself holding the keyboard: a list or
        /// a scroll view left with it would take the keys the page answers.
        function held(box) {
            return "caret=" + box.activeFocus + " page=" + (root.Window.window.activeFocusItem === page)
        }

        /// The caret into `box` by a press, out by a press on the air, and Escape after it still reaches the page.
        function walkAway(box) {
            mouseClick(box, 10, 10)
            verify(box.activeFocus)
            mouseClick(air, 10, 10)
            compare(held(box), "caret=false page=true")
            const before = page.escapes
            keyClick(Qt.Key_Escape)
            compare(page.escapes, before + 1)
        }

        function test_a_box_in_a_scroll_view() {
            walkAway(summary)
        }

        function test_a_box_under_the_page() {
            walkAway(field)
        }

        function test_a_box_in_a_list_row() {
            walkAway(list.itemAtIndex(0))
        }
    }
}
