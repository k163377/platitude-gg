import QtQuick
import QtTest

// What Qt does with the pointer when a list moves under a hand that never moved — the question behind
// "may a row of the sidebar grow where it stands, and may the list send itself to show what it opened"
// (デザイン規約 §左メニューの所作).
//
// **Hover follows the item, not the hand**: a scroll with the pointer standing still takes the hover off the row it
// was on and gives it to whatever arrives in that place, and the app hears it as an ordinary arrival — the wash, the
// rest and everything else a row does on hover.
//
// **And the ancestor is told as well.** A handler on the item the rows stand in has its `point` handed to it again
// when the layout moves under a still pointer, so *being told* is no test of whether the hand moved — **the position
// is**. That is the whole of why the panel keeps the last place it saw the hand and weighs the new one against it
// (`SidebarPane`), rather than counting what it is told.
Item {
    id: root
    width: 200
    height: 200

    readonly property int rowHeight: 24
    /// The row that has opened, and grown where it stands (`NavItemDelegate` の `factsOpen`).
    property int grownRow: -1

    /// Which row the pointer is on, as the rows themselves answer — the reading `NavItemDelegate.pointed` makes.
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
        // The sidebar's lists recycle their rows (`AppListView`), so the probe does too: a delegate handed to
        // another row is part of what is being asked about.
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
            // The hand comes to rest in the middle of row 1 and stays there for the whole test.
            mouseMove(list, 100, root.rowHeight + root.rowHeight / 2)
            tryCompare(root, "pointedRow", 1, undefined, "the row under the hand is the one it was moved to")

            // The list sends itself by two rows — what showing an open row's lines costs when the section it is in
            // cannot grow (`NavList.revealOpenRow`). Nothing touches the pointer.
            list.contentY = root.rowHeight * 2
            verify(waitForRendering(list), "the move is on screen")

            // Qt re-delivers hover for the item that arrived under the still pointer, so the row the reader was on
            // is no longer the row the app thinks they are on.
            tryCompare(root, "pointedRow", 3, undefined,
                       "the row that arrived under the hand takes the hover from the one the hand was on")
        }

        /// The ancestor's side of it: **it hears every move of the hand, and it is told again when the layout moves
        /// under a hand that did not** — so what the panel counts is the place, not the telling.
        function test_the_place_the_ancestor_was_told_about_is_the_test() {
            mouseMove(list, 100, root.rowHeight / 2)
            const walked = root.moves
            verify(walked > 0, "the pointer moving is heard on the item the rows stand in")

            mouseMove(list, 100, root.rowHeight * 3 + root.rowHeight / 2)
            verify(root.moves > walked, "and every further move of the hand is heard as well")

            // The list sends itself with the pointer standing still: the rows under it change, the hand does not.
            const rested = root.moves
            const told = root.stirs
            list.contentY = root.rowHeight * 2
            verify(waitForRendering(list), "the move is on screen")
            verify(root.stirs > told, "the ancestor is told about the pointer again — the layout moved under it")
            compare(root.moves, rested, "but the place it was told about is the one it already had")
        }

        /// And the other way a list moves under a hand: **a row growing where it stands**, which is what one of these
        /// rows does when it opens (デザイン規約 §左メニューの所作). The pointer stays inside the row that grew, so
        /// what it is on has not changed — and the place the ancestor knows has not moved either.
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
