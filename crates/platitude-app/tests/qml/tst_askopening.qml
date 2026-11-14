import QtQuick
import QtTest
import platitude.ui

// A question raised with the bar built open (`AskBar`'s `Component.onCompleted`): `open` changes while the bar is
// still completing, before its words are built, and the bar takes the opening once it completes — the pill holds the
// keyboard unringed, as on any opening without a form. Alone in its window: the opening is a focus, and any other bar
// or test that moves the focus clears it.
Item {
    id: root
    width: 400
    height: 200

    AskBar {
        id: bar
        width: root.width
        label: "Drop the stash?"
        accept: "drop"
        open: true
    }

    TestCase {
        name: "AskOpening"
        when: windowShown

        function test_a_bar_built_open_hands_its_pill_the_opening() {
            tryVerify(() => bar.pill !== null && bar.pill.activeFocus)
            verify(bar.pill.tookTheOpening, "the focus is the opening's, so the pill wears no ring")
        }
    }
}
