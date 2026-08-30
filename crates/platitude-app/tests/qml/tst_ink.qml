import QtQuick
import QtTest
import platitude.ui

Item {
    id: root
    width: 80
    height: 80

    Component {
        id: fixture
        Item {
            width: 32
            height: 28
            property alias mark: mark
            InkCanvas {
                id: mark
                anchors.fill: parent
                onPaint: {
                    const ctx = getContext("2d")
                    ctx.fillStyle = "white"
                    ctx.fillRect(0, 0, width, height)
                }
            }
        }
    }

    TestCase {
        name: "VisibleInk"
        when: windowShown

        function test_hidden_parent_does_not_hold_the_screenshot() {
            const before = Ink.owed
            const item = createTemporaryObject(fixture, root, { visible: false })
            verify(item !== null)
            verify(item.mark.Window.window !== null)
            verify(!item.mark.inked)
            compare(Ink.owed, before)
        }

        function test_hiding_before_paint_releases_and_showing_reacquires() {
            const before = Ink.owed
            const item = createTemporaryObject(fixture, root)
            verify(item !== null)
            compare(Ink.owed, before + 1)
            item.visible = false
            compare(Ink.owed, before)
            item.visible = true
            compare(Ink.owed, before + 1)
            tryCompare(item.mark, "inked", true)
            compare(Ink.owed, before)
            item.visible = false
            item.visible = true
            compare(Ink.owed, before)
        }
    }
}
