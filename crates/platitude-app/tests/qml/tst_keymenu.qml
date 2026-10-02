import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The keyboard's way into the right-click menus (`KeyMenu`, デザイン規約 §メニュー のキーボード).
// - A menu asked for through `KeyMenu.ask` stands at the place asked, not under the pointer, and the place is spent on
//   that one `offer()`.
// - The menu key reaches the list holding the keyboard as a key, and the list's menu stands under its row's left end.
// - Qt's request is taken where nothing above took it, as the window takes it (`Main.keyMenuAsked`): only one with no
//   place is the keyboard's (`KeyMenu.placeless`), and it is answered at the focus (`KeyMenu.answer`) — a box through
//   its own seat, a list through its door. A right-click's names the point it landed on, pressed or released (the
//   premise the window stands on). Windows' placeless request cannot be raised from here; the window's origin, which
//   Qt names for it, stands in.
Item {
    id: root
    width: 480
    height: 360

    // Where the window takes what nothing above took: under everything here.
    Item {
        id: ground
        anchors.fill: parent
        ContextMenu.onRequested: position => root.asked(ground.mapToItem(null, position))
    }
    property var requests: []
    function asked(scenePoint) {
        root.requests.push(scenePoint)
        return KeyMenu.placeless(scenePoint) && KeyMenu.answer(root.Window.activeFocusItem)
    }

    // A list wired as the product's are (`GraphPane`'s list): the key, and a door the window's answer finds.
    ListView {
        id: list
        x: 40
        y: 30
        width: 300
        height: 200
        model: 5
        delegate: Rectangle {
            required property int index
            width: list.width
            height: 24
            color: "transparent"
        }
        property int asked: 0
        function menuFromKeys() {
            if (!list.activeFocus)
                return false
            list.asked++
            const item = list.itemAtIndex(list.currentIndex)
            KeyMenu.ask(item.mapToItem(null, 0, item.height), () => menu.offer())
            return true
        }
        Keys.onMenuPressed: event => event.accepted = list.menuFromKeys()
    }
    FormField {
        id: box
        x: 40
        y: 260
        width: 200
        text: "a box"
    }
    // Off the window's origin, so a place handed over in the wrong terms lands visibly elsewhere.
    Item {
        x: 7
        y: 11
        width: root.width
        height: root.height
        AppMenu {
            id: menu
            AppMenuItem {
                text: "One"
            }
        }
    }

    TestCase {
        name: "KeyMenu"
        when: windowShown

        property int trigger: Application.styleHints.contextMenuTrigger
        function init() {
            menu.close()
            tryCompare(menu, "visible", false)
            list.asked = 0
            list.currentIndex = 2
            root.requests = []
            root.forceActiveFocus()
        }
        // The style hint is the application's: a file that leaves it moved hands the files after it another platform.
        function cleanup() {
            Application.styleHints.contextMenuTrigger = trigger
        }
        function standsAt() {
            return menu.parent.mapToItem(null, menu.x, menu.y)
        }
        function seatOf(editor) {
            for (let i = 0; i < editor.children.length; i++)
                if (editor.children[i].editor === editor)
                    return editor.children[i]
            return null
        }

        function test_a_menu_asked_from_the_keyboard_stands_where_asked() {
            verify(KeyMenu.ask(Qt.point(100, 120), () => menu.offer()), "the door's answer comes back")
            tryCompare(menu, "opened", true)
            compare(standsAt(), Qt.point(100, 120))
            compare(KeyMenu.at, null, "the place is spent on the one offer")
        }

        function test_the_menu_key_opens_the_lists_menu_under_its_row() {
            list.forceActiveFocus()
            keyClick(Qt.Key_Menu)
            tryCompare(menu, "opened", true)
            compare(list.asked, 1)
            const row = list.itemAtIndex(2)
            compare(standsAt(), row.mapToItem(null, 0, row.height), "under the current row's left end")
        }

        // Linux raises a right-click's request at the press, Windows at the release.
        function test_a_right_click_names_its_point_and_the_window_turns_it_away_data() {
            return [
                { tag: "press", trigger: Qt.ContextMenuTrigger.Press },
                { tag: "release", trigger: Qt.ContextMenuTrigger.Release },
            ]
        }
        function test_a_right_click_names_its_point_and_the_window_turns_it_away(data) {
            Application.styleHints.contextMenuTrigger = data.trigger
            list.forceActiveFocus()
            mouseClick(list, 50, 60, Qt.RightButton)
            // The right-click made a request, which nothing above the ground took, naming where it landed.
            tryVerify(() => root.requests.length === 1)
            compare(root.requests[0], list.mapToItem(null, 50, 60))
            compare(list.asked, 0, "a placed request is not the keyboard's")
            verify(!menu.visible)
        }

        function test_only_the_window_origin_is_placeless() {
            verify(KeyMenu.placeless(Qt.point(0, 0)))
            verify(KeyMenu.placeless(Qt.point(1, -1)), "a pixel's rounding either way")
            verify(!KeyMenu.placeless(Qt.point(5, 0)), "a point of its own")
        }

        function test_the_placeless_request_is_answered_at_the_focus() {
            verify(!root.asked(Qt.point(0, 0)), "the keyboard nowhere that answers: nothing")
            list.forceActiveFocus()
            verify(root.asked(Qt.point(0, 0)), "the list holding the keyboard, through its door")
            tryCompare(menu, "opened", true)
            compare(list.asked, 1)
            menu.close()
            tryCompare(menu, "visible", false)
            box.forceActiveFocus()
            verify(root.asked(Qt.point(0, 0)), "the box holding the caret, through its own seat")
            const seat = seatOf(box)
            tryVerify(() => seat.item !== null && seat.item.opened)
            compare(list.asked, 1, "not the list's")
            const caret = box.cursorRectangle
            compare(Qt.point(seat.item.x, seat.item.y), Qt.point(caret.x, caret.y + caret.height), "under the caret")
        }
    }
}
