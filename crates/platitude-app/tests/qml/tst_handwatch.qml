import QtQuick
import QtTest
import platitude.ui

// The hand over a surface as the rows read it (`HandWatch`): a move is the place changing, not Qt telling the point
// again; walking off counts; the rows moving under a still hand hold it until its own next move; a popup over the hand
// is the hand gone. A real pointer, since the facts are Qt's delivery.
Item {
    id: root
    width: 300
    height: 300

    Item {
        id: pane
        anchors.fill: parent
        HandWatch {
            id: hand
        }
    }
    /// Another surface over the pane's corner, with a handler of its own that takes the hover there: the right part of
    /// the menu below reaches over it.
    Item {
        x: 240
        y: 70
        width: 60
        height: 70
        HoverHandler {
            id: besideHand
        }
    }
    /// How often what hangs off the rows was told they moved (`HandWatch.rowsShifted`).
    property int told: 0
    /// Whether the hold stood when the rows last read the hand (`HandWatch.moves`), and how often Qt told the
    /// handler the point.
    property bool heldAsRead: false
    property int retold: 0
    Connections {
        target: hand
        function onRowsShifted() { ++root.told }
        function onMovesChanged() { root.heldAsRead = hand.stillHeld }
        function onPointChanged() { ++root.retold }
    }
    /// A popup standing over the hand: Qt hands nothing under it its hover.
    AppMenu {
        id: menu
        x: 80
        y: 80
        widthFloor: 200
        AppMenuItem {
            text: "Act"
        }
    }
    /// A list moving under the hand, as the sidebar's does.
    Item {
        id: rows
        width: 300
        height: 100
    }

    TestCase {
        name: "HandWatch"
        when: windowShown

        function init() {
            if (menu.visible)
                menu.dismiss()
            tryVerify(() => !menu.visible)
            hand.covered = 0
            mouseMove(root, root.width - 1, root.height - 1)
            tryVerify(() => hand.known, undefined, "the hand is on the pane")
            hand.stillHeld = false
            rows.y = 0
        }

        function test_a_move_is_the_place_changing() {
            mouseMove(root, 100, 100)
            tryVerify(() => hand.at.x === 100 && hand.at.y === 100)
            const walked = hand.moves
            const retold = root.retold
            // The rows moving under the pointer have Qt tell the point again at the same place
            // (`tst_hoverunderstillhand.qml`); told again, the place is what it was.
            rows.y = 40
            tryVerify(() => root.retold > retold, undefined, "Qt told the handler the point again")
            compare(hand.moves, walked, "told the same place is not a move")
            mouseMove(root, 100, 101)
            tryCompare(hand, "moves", walked + 1, undefined, "a pixel is a move")
        }

        function test_walking_off_is_a_move_and_the_place_is_gone() {
            mouseMove(root, 100, 100)
            tryVerify(() => hand.at.x === 100)
            const walked = hand.moves
            mouseMove(root, root.width + 10, 100)
            tryVerify(() => !hand.known, undefined, "off the pane, the hand is nowhere")
            compare(hand.moves, walked + 1, "and walking off counted")
        }

        function test_rows_moved_under_the_hand_hold_until_its_own_move() {
            mouseMove(root, 100, 100)
            tryVerify(() => hand.at.x === 100)
            const shifts = hand.shifts
            hand.rowsMoved()
            verify(hand.stillHeld)
            compare(hand.shifts, shifts + 1, "the rows read the hand over them again")
            // The rows moving under the pointer have Qt tell the point again at the same place: no move.
            const retold = root.retold
            rows.y = 40
            tryVerify(() => root.retold > retold, undefined, "Qt told the handler the point again")
            verify(hand.stillHeld, "told the same place again is not the hand's move")
            mouseMove(root, 101, 100)
            tryVerify(() => !hand.stillHeld, undefined, "a pixel of the hand's own lifts it")
            verify(root.heldAsRead, "and the rows read where the hand moved to before the hold lifted")
        }

        /// A popup standing over the still hand and going is no move of the hand's: the hold stands.
        function test_a_popup_over_a_still_hand_lifts_no_hold() {
            mouseMove(root, 100, 100)
            tryVerify(() => hand.at.x === 100)
            hand.rowsMoved()
            verify(hand.stillHeld)
            menu.offerHere()
            tryVerify(() => menu.opened && !hand.known, undefined, "the menu over the hand takes it off the pane")
            verify(menu.pointerInside && hand.hidden, "the menu has the hand, hidden from the pane")
            verify(hand.stillHeld, "gone under a popup, the hand has not moved")
            menu.dismiss()
            tryVerify(() => hand.known && hand.at.x === 100, undefined, "heard again where it was as the menu goes")
            verify(hand.stillHeld, "and held still")
            mouseMove(root, 100, 101)
            tryVerify(() => !hand.stillHeld, undefined, "the hand's own move lifts it")
        }

        /// The same with the hand on the menu's row rather than its band: the row's handler keeps the hand until Qt
        /// has handed it back to the pane (`AppMenuItem.handOn`), so nothing tells the hand went before the pane hears
        /// it where it was.
        function test_a_menu_put_down_with_the_hand_on_its_row_lifts_no_hold() {
            menu.offerHere()
            tryVerify(() => menu.opened)
            const row = menu.itemAt(0)
            const on = row.mapToItem(root, row.width / 2, row.height / 2)
            const x = Math.round(on.x)
            const y = Math.round(on.y)
            menu.dismiss()
            tryVerify(() => !menu.visible)
            mouseMove(root, x, y)
            tryVerify(() => hand.at.x === x && hand.at.y === y, undefined, "the hand where the menu's row will stand")
            hand.rowsMoved()
            menu.offerHere()
            tryVerify(() => menu.opened && !hand.known && row.handOn, undefined, "the menu's row over the hand")
            verify(hand.hidden && hand.stillHeld, "hidden from the pane, and held")
            menu.dismiss()
            tryVerify(() => hand.known && hand.at.x === x, undefined, "heard again where it was as the menu goes")
            verify(hand.stillHeld, "and held still")
        }

        /// A menu standing away from the hand hides nothing: the hand gone from the pane meanwhile has left it, as with
        /// no menu at all.
        function test_a_hand_gone_with_a_menu_standing_elsewhere_has_left() {
            mouseMove(root, 30, 30)
            tryVerify(() => hand.at.x === 30)
            hand.rowsMoved()
            menu.offerHere()
            tryVerify(() => menu.opened, undefined, "the menu stands")
            verify(hand.known && !menu.pointerInside, "away from the hand, which the pane still hears")
            mouseMove(root, root.width + 10, 30)
            tryVerify(() => !hand.known, undefined, "the hand is out of the window")
            verify(!hand.hidden, "in nothing standing over the pane")
            verify(!hand.stillHeld, "so it left, and the hold lifts")
        }

        /// The hand hidden under the menu, walking out of it to where the pane does not reach, has left: the menu says
        /// it lost the hand (`OutsidePress.handLeft`), and nothing has it.
        function test_a_hand_that_walks_out_of_a_menu_to_elsewhere_has_left() {
            mouseMove(root, 100, 100)
            tryVerify(() => hand.at.x === 100)
            hand.rowsMoved()
            menu.offerHere()
            tryVerify(() => menu.opened && !hand.known, undefined, "the menu over the hand takes it off the pane")
            verify(hand.hidden && hand.stillHeld, "hidden under it, and held still")
            mouseMove(root, root.width + 10, 100)
            tryVerify(() => !menu.pointerInside, undefined, "the hand walked out of the menu")
            verify(!hand.hidden, "out of the window: it left")
            verify(!hand.stillHeld, "and the hold lifts")
        }

        /// A hand elsewhere has left every row already: the rows read nothing again, and only what hangs off them is
        /// told — so a move that takes many rows costs no read per row per row.
        function test_rows_moved_under_no_hand_hold_nothing_but_are_still_told() {
            mouseMove(root, root.width + 10, 100)
            tryVerify(() => !hand.known)
            const shifts = hand.shifts
            const told = root.told
            hand.rowsMoved()
            verify(!hand.stillHeld, "a hand elsewhere comes onto the rows by walking")
            compare(hand.shifts, shifts, "the rows have no hand to read again")
            compare(root.told, told + 1, "but the rows did move: what hangs off them is told")
        }

        /// The hand in a card hung off the rows (`covered`, which the card's seat counts) is over the rows though
        /// the pane does not hear it: the rows moving hold, from the first place heard once the card has gone.
        function test_rows_moved_under_a_hand_in_a_card_hold_from_the_first_place_heard() {
            mouseMove(root, root.width + 10, 100)
            tryVerify(() => !hand.known)
            hand.covered = 1
            hand.rowsMoved()
            verify(hand.stillHeld, "held, the hand being in a card over the rows")
            verify(hand.heldAt.x < 0, "from no place yet")
            hand.covered = 0
            mouseMove(root, 100, 100)
            tryVerify(() => hand.at.x === 100)
            verify(hand.stillHeld, "the first place heard is where the hold is from: being heard is not a move")
            compare(hand.heldAt.x, 100)
            mouseMove(root, 102, 101)
            tryVerify(() => !hand.stillHeld, undefined, "a move from the place heard lifts it")
        }

        /// A menu reaching past the pane, put down with the hand in the part over the surface beside: Qt hands that
        /// surface the hand, and the pane never hears it — the hand had left the pane, and the hold lifts. Put down
        /// with the hand over the pane, the pane hears it again where it was (`test_a_popup_over_a_still_hand_…`).
        function test_a_menu_put_down_over_the_surface_beside_leaves_the_hand_there() {
            mouseMove(root, 100, 100)
            tryVerify(() => hand.at.x === 100)
            hand.rowsMoved()
            menu.offerHere()
            tryVerify(() => menu.opened && !hand.known, undefined, "the menu over the hand takes it off the pane")
            mouseMove(root, 260, 95)
            tryVerify(() => menu.pointerInside, undefined, "still in the menu, over the surface beside")
            verify(hand.hidden && hand.stillHeld, "hidden from the pane, and held")
            menu.dismiss()
            tryVerify(() => besideHand.hovered && !menu.pointerInside, undefined, "the surface beside has the hand")
            verify(!hand.known && !hand.hidden, "the pane never heard it: it left")
            verify(!hand.stillHeld, "and the hold lifts")
        }

        /// The hand in a card hung off the rows (`covered`) is hidden from the pane; the card losing it with the pane
        /// not hearing it has it left (`uncovered`).
        function test_a_hand_a_card_lost_where_the_pane_does_not_hear_it_has_left() {
            mouseMove(root, 100, 100)
            tryVerify(() => hand.at.x === 100)
            hand.covered = 1
            mouseMove(root, root.width + 10, 100)
            tryVerify(() => !hand.known)
            verify(hand.hidden, "gone with a card over the rows that has it: hidden")
            hand.uncovered()
            verify(!hand.hidden, "the card lost it and the pane does not hear it: the hand left")
            compare(hand.covered, 0)
        }

        function test_a_second_surface_feeds_the_same_count() {
            mouseMove(root, 100, 100)
            tryVerify(() => hand.at.x === 100)
            const walked = hand.moves
            hand.stir(Qt.point(400, 400))
            compare(hand.moves, walked + 1)
            compare(hand.at.x, 400)
            hand.stir(Qt.point(-1, -1))
            compare(hand.moves, walked + 2)
            verify(!hand.known)
        }
    }
}
