import QtQuick
import QtTest

// What a TapHandler knows about the keyboard when it reports a tap — the question behind "may the chip's card read a
// held Ctrl or Shift off its own rows" (デザイン規約 §複数のコミットを選ぶ). A `MouseArea` hands its handler a `mouse`
// with `modifiers` on it; a `TapHandler`'s `singleTapped(eventPoint, button)` carries none, so the card reads the
// handler's own `point.modifiers` instead, and this holds that reading down: **it is the modifier of the press the tap
// reports** (`RefListPopup`'s rows read the double off the same point). Only the single tap can be driven here —
// `mouseDoubleClickSequence` reaches a `MouseArea` (`tst_moddblclick`) and not a `TapHandler`, which reports no double
// for it (measured).
Item {
    id: root
    width: 200
    height: 200

    property int singleModifiers: -1

    TapHandler {
        id: tap
        onSingleTapped: root.singleModifiers = tap.point.modifiers
    }

    TestCase {
        name: "TapHandlerModifiers"
        when: windowShown

        function test_a_single_tap_reports_the_modifier_held_for_it() {
            root.singleModifiers = -1
            mouseClick(root, 20, 20, Qt.LeftButton, Qt.ControlModifier)
            tryVerify(() => root.singleModifiers !== -1, 1000, "the tap is reported once the double-click window closes")
            verify((root.singleModifiers & Qt.ControlModifier) !== 0,
                   "the handler's point carries the modifier the press was made with")
        }

        function test_a_plain_tap_reports_no_modifier() {
            root.singleModifiers = -1
            mouseClick(root, 20, 20, Qt.LeftButton, Qt.NoModifier)
            tryVerify(() => root.singleModifiers !== -1, 1000, "the tap is reported once the double-click window closes")
            compare(root.singleModifiers, Qt.NoModifier, "and a plain press reads as plain")
        }
    }
}
