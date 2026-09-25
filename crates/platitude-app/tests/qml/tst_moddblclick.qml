import QtQuick
import QtTest

// What Qt does with a double-click that had a modifier held down (デザイン規約 §複数のコミットを選ぶ): the area is
// handed the double whatever was held, modifiers riding along, so a surface whose double-click leads to a write has
// to read the modifiers itself (`GraphRowDelegate.doubleClick`).
Item {
    id: root
    width: 200
    height: 200

    property int plainDoubles: 0
    property int heldDoubles: 0
    property int lastModifiers: 0

    MouseArea {
        id: area
        anchors.fill: parent
        onDoubleClicked: mouse => {
            root.lastModifiers = mouse.modifiers
            if (mouse.modifiers & Qt.ControlModifier)
                root.heldDoubles++
            else
                root.plainDoubles++
        }
    }

    TestCase {
        name: "ModifiedDoubleClick"
        when: windowShown

        function test_an_area_is_handed_the_double_whatever_was_held() {
            // A quick hand toggling a row out of a choice and back in with Ctrl.
            mouseDoubleClickSequence(area, 20, 20, Qt.LeftButton, Qt.ControlModifier)
            compare(root.heldDoubles, 1, "Qt delivers the double even with the modifier held")
            verify((root.lastModifiers & Qt.ControlModifier) !== 0,
                   "and the event carries the modifier, so the surface can tell for itself")
            compare(root.plainDoubles, 0, "and it is not reported as a plain one")
        }
    }
}
