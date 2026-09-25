import QtQuick
import QtTest
import platitude.ui

// Who takes a caret out of a field, and who leaves it lit (規約 §右のペインの字は掴める「選択は窓に 1 つだけ」). QML never
// drops a text field's focus on its own (`FocusRelease`), so every press target on a reading surface has to be asked.
// Only a real pointer can answer: the headless verbs enter the hand's own functions, not Qt's delivery.
// The shape is a dialog's (`AppDialog`): the watcher stands on the face behind everything, the targets over it.
Item {
    id: root
    width: 520
    height: 320

    Rectangle {
        id: face
        anchors.fill: parent
        color: "transparent"
        FocusRelease {
            window: root.Window.window
            home: body
        }
    }

    Item {
        id: body
        anchors.fill: parent

        CardText {
            id: words
            x: 0
            y: 0
            width: 400
            text: "Saved to your global git configuration, replacing what is set there."
        }

        // The rail's shape: a row that answers a press with a handler of its own and asks for no focus.
        Rectangle {
            id: railRow
            x: 0
            y: 120
            width: 160
            height: 32
            color: "transparent"
            property int taps: 0
            TapHandler {
                onTapped: railRow.taps++
            }
        }
        AppCheckBox {
            id: tick
            x: 200
            y: 120
            text: "Load the whole history"
        }
        ActionButton {
            id: button
            x: 0
            y: 200
            text: "Save"
        }
        FormField {
            id: box
            x: 200
            y: 200
            width: 200
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

    // A screen whose Escape a field could swallow would shut in a reader who dragged over a sentence
    // (`SettingsDialog`).
    property int escapes: 0
    Shortcut {
        sequences: [StandardKey.Cancel]
        onActivated: root.escapes++
    }

    TestCase {
        name: "FieldRelease"
        when: windowShown

        /// A drag over the words, made the way the hand makes it — `forceActiveFocus()` on this part would focus the
        /// `Item` and leave the `TextEdit` inside holding nothing.
        function grab() {
            words.anchorFrom(root, 0, 5)
            words.extendFrom(root, 300, 5)
            verify(words.hasCaret)
            verify(words.selected !== "")
        }

        /// One line, so a failure says which half survived.
        function held() {
            return "caret=" + words.hasCaret + " selected=" + (words.selected !== "")
        }

        // The two the watcher is for: neither a row's own handler nor the air asks for the focus.
        function test_a_press_on_a_rail_row_drops_it() {
            grab()
            mouseClick(railRow, 10, 10)
            // The row still gets its press: the watcher accepts nothing and takes only a passive grab.
            compare(railRow.taps, 1)
            compare(held(), "caret=false selected=false")
        }

        function test_a_press_on_nothing_drops_it() {
            grab()
            mouseClick(air, 10, 10)
            compare(held(), "caret=false selected=false")
        }

        // The three that take the focus themselves — a property of the control, so one that stopped would leave the
        // selection lit.
        function test_a_press_on_a_tick_drops_it() {
            grab()
            mouseClick(tick, 10, 10)
            compare(held(), "caret=false selected=false")
        }

        function test_a_press_on_a_button_drops_it() {
            grab()
            mouseClick(button, 10, 10)
            compare(held(), "caret=false selected=false")
        }

        function test_a_press_on_a_box_drops_it() {
            grab()
            mouseClick(box, 10, 10)
            compare(held(), "caret=false selected=false")
        }

        // A read-only field takes the arrows and `Ctrl+C` but lets Escape through.
        function test_the_words_do_not_swallow_the_way_out() {
            grab()
            const before = root.escapes
            keyClick(Qt.Key_Escape)
            compare(root.escapes, before + 1)
        }
    }
}
