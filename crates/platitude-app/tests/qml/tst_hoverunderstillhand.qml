import QtQuick
import QtTest

// What Qt does with the pointer when a list moves under a hand that never moved (デザイン規約 §左メニューの所作):
//  - hover follows the item, not the hand: a scroll gives it to whatever arrives under the still pointer, and the app
//    hears an ordinary arrival;
//  - a handler on the ancestor has its `point` handed to it again when the layout moves, so being told is no test of
//    whether the hand moved — the position is (why `SidebarPane` weighs the new place against the last). It comes on
//    a later frame than the one the move is drawn on (Qt 6.12), so it is waited for.
Item {
    id: root
    width: 200
    height: 200

    readonly property int rowHeight: 24
    /// The row that has opened and grown where it stands (`NavItemDelegate.factsOpen`).
    property int grownRow: -1

    /// Which row the pointer is on, as the rows answer (the reading `NavItemDelegate.pointed` makes).
    property int pointedRow: -1

    /// How often the ancestor was told about the pointer, and how often the place it was told about had changed —
    /// the two the panel has to tell apart (`SidebarRowGestures.handMoves`).
    property int stirs: 0
    property int moves: 0
    property point handAt: Qt.point(-1, -1)

    HoverHandler {
        id: watch
        onPointChanged: {
            ++root.stirs
            if (watch.point.position.x !== root.handAt.x || watch.point.position.y !== root.handAt.y) {
                root.handAt = watch.point.position
                ++root.moves
            }
        }
    }

    ListView {
        id: list
        width: 200
        height: root.rowHeight * 5
        clip: true
        // The sidebar's lists recycle their rows (`AppListView`), so the probe does too.
        reuseItems: true
        model: 12
        delegate: Item {
            id: row
            required property int index
            width: list.width
            height: root.rowHeight + (root.grownRow === row.index ? root.rowHeight * 3 : 0)
            HoverHandler {
                id: hover
                onHoveredChanged: {
                    if (hover.hovered)
                        root.pointedRow = row.index
                    else if (root.pointedRow === row.index)
                        root.pointedRow = -1
                }
            }
        }
    }

    TestCase {
        name: "HoverUnderAStillHand"
        when: windowShown

        function init() {
            list.contentY = 0
            root.grownRow = -1
        }

        function test_a_list_that_scrolls_hands_the_hover_to_another_row() {
            mouseMove(list, 100, root.rowHeight + root.rowHeight / 2)
            tryCompare(root, "pointedRow", 1, undefined, "the row under the hand is the one it was moved to")

            // What `NavList.revealOpenRow` does when the section cannot grow; nothing touches the pointer.
            list.contentY = root.rowHeight * 2
            verify(waitForRendering(list), "the move is on screen")

            tryCompare(root, "pointedRow", 3, undefined,
                       "the row that arrived under the hand takes the hover from the one the hand was on")
        }

        function test_the_place_the_ancestor_was_told_about_is_the_test() {
            mouseMove(list, 100, root.rowHeight / 2)
            const walked = root.moves
            verify(walked > 0, "the pointer moving is heard on the item the rows stand in")

            mouseMove(list, 100, root.rowHeight * 3 + root.rowHeight / 2)
            verify(root.moves > walked, "and every further move of the hand is heard as well")

            const rested = root.moves
            const told = root.stirs
            list.contentY = root.rowHeight * 2
            verify(waitForRendering(list), "the move is on screen")
            tryVerify(() => root.stirs > told, undefined,
                      "the ancestor is told about the pointer again — the layout moved under it")
            compare(root.moves, rested, "but the place it was told about is the one it already had")
        }

        /// The other way a list moves under a hand: a row opening grows where it stands.
        function test_a_row_growing_under_the_hand_is_not_the_hand_moving() {
            mouseMove(list, 100, root.rowHeight + root.rowHeight / 2)
            tryCompare(root, "pointedRow", 1, undefined, "the hand is on the row that is about to grow")

            const rested = root.moves
            root.grownRow = 1
            verify(waitForRendering(list), "the row is taller on screen")
            compare(root.pointedRow, 1, "the pointer is still inside the row it was on")
            compare(root.moves, rested, "and a row growing under a still hand is not the hand moving either")
        }
    }
}
