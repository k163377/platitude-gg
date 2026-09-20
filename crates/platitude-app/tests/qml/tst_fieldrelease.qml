import QtQuick
import QtTest
import platitude.ui

// Who takes a caret out of a field, and who leaves it lit. **QML never drops a text field's focus on its own**
// (`FocusRelease`), so on a surface whose words are read every press target has to be asked which of the two it is:
// a field still wearing its blue after the reader has moved on is the screen saying it is holding something
// (規約 §右のペインの字は掴める「選択は窓に 1 つだけ」).
//
// **Only a real pointer can answer this.** The headless verbs enter the hand's own functions, so they say nothing
// about who Qt hands a press to.
//
// The shape is a dialog's: the watcher stands on the face, behind everything the screen draws, and the targets are
// laid over it (`AppDialog`). Three of them take the focus themselves and two do not — and it is the two that this
// file exists for, because they are the whole of what used to keep a selection lit through every press that
// followed it.
Item {
    id: root
    width: 520
    height: 320

    // The dialog's face. What accepts a press above this either takes the focus itself or is a reading surface's own
    // hand, so the only presses that reach the watcher are the ones that would otherwise leave a caret standing.
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
        // The band's air and the margins outside the block: nothing is drawn there and nothing answers a press.
        Item {
            id: air
            x: 0
            y: 260
            width: 400
            height: 50
        }
    }

    // The way out of a screen whose words are read. **A field is a `TextEdit`**, and a screen that put its Escape
    // behind one would be a screen a reader could shut themselves out of by dragging over a sentence
    // (`SettingsDialog`).
    property int escapes: 0
    Shortcut {
        sequences: [StandardKey.Cancel]
        onActivated: root.escapes++
    }

    TestCase {
        name: "FieldRelease"
        when: windowShown

        /// The state a reader is in after a drag over the words: the field holds the keyboard and a selection.
        /// Made the way the hand makes it — `forceActiveFocus()` on this part would put the focus on the `Item`
        /// and leave the `TextEdit` inside it holding nothing.
        function grab() {
            words.anchorFrom(root, 0, 5)
            words.extendFrom(root, 300, 5)
            verify(words.hasCaret)
            verify(words.selected !== "")
        }

        /// What the field is holding, as one line, so a failure says which half survived.
        function held() {
            return "caret=" + words.hasCaret + " selected=" + (words.selected !== "")
        }

        // The two the watcher is for. A row that answers with its own handler never asks for the focus, and the
        // plain air answers nothing at all — so before the watcher stood there, both left the selection lit.
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

        // The three that answer for themselves. Kept because "it takes the focus" is a property of the control, not
        // of this window: a tick or a button that stopped taking it would put the selection back where it was.
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

        // The words hold the caret and the way out still answers: a read-only field takes the arrows and `Ctrl+C`,
        // and lets the key that shuts the screen through.
        function test_the_words_do_not_swallow_the_way_out() {
            grab()
            const before = root.escapes
            keyClick(Qt.Key_Escape)
            compare(root.escapes, before + 1)
        }
    }
}
