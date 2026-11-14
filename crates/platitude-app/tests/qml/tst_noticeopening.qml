import QtQuick
import QtTest
import platitude.ui

// A report raised with the bar built open (`NoticeBar`'s `Component.onCompleted`): `open` changes while the bar is
// still completing, before its words are built, and the bar takes the opening once it completes — the `OK` holds the
// keyboard unringed, as on any opening (`tst_askopening.qml`, the question bar's). Alone in its window: the opening is
// a focus, and any other bar or test that moves the focus clears it.
Item {
    id: root
    width: 400
    height: 200

    NoticeBar {
        id: bar
        width: root.width
        label: "Nothing was pushed"
        detail: "The remote said no."
        open: true
    }

    TestCase {
        name: "NoticeOpening"
        when: windowShown

        function test_a_bar_built_open_hands_its_pill_the_opening() {
            tryVerify(() => bar.pill !== null && bar.pill.activeFocus)
            verify(bar.pill.tookTheOpening, "the focus is the opening's, so the pill wears no ring")
            tryVerify(() => bar.settled, undefined, "and its words stand")
        }
    }
}
