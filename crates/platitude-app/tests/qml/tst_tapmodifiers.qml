import QtQuick
import QtTest

// A `TapHandler`'s `singleTapped` carries no modifiers, so `RefListPopup`'s rows read the handler's own
// `point.modifiers` (デザイン規約 §複数のコミットを選ぶ); this holds that it is the modifier of the press the tap
// reports. Only the single tap can be driven: `mouseDoubleClickSequence` reaches a `MouseArea` (`tst_moddblclick`)
// and not a `TapHandler`.
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
            tryVerify(() => root.singleModifiers !== -1, undefined,
                      "the tap is reported once the double-click window closes")
            verify((root.singleModifiers & Qt.ControlModifier) !== 0,
                   "the handler's point carries the modifier the press was made with")
        }

        function test_a_plain_tap_reports_no_modifier() {
            root.singleModifiers = -1
            mouseClick(root, 20, 20, Qt.LeftButton, Qt.NoModifier)
            tryVerify(() => root.singleModifiers !== -1, undefined,
                      "the tap is reported once the double-click window closes")
            compare(root.singleModifiers, Qt.NoModifier, "and a plain press reads as plain")
        }
    }
}
