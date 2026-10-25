import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The window's tooltip (`TipSeat`) over targets that move under a hand that does not, and under a standing menu, with
// a real pointer and Qt's own hover — which a scroll hands to whatever row arrives under a still pointer
// (`tst_hoverunderstillhand`):
//  - a row's tip is asked through the list's hand (`TipAsk.hand`, found through `AppListView.rowsHand`): the rows
//    moving take the box down at once, and the row that arrived under the still hand asks nothing until the hand has
//    moved — any move, the rest being the judgement (デザイン規約 §左メニューの所作「手が動いたかの見切り」);
//  - behind a menu standing in the window nothing comes out and what is out goes the moment the menu stands; a row of
//    the menu itself is not behind it, nor is a refusal of what is typed (デザイン規約 §メニュー);
//  - what the menu took down comes back once the hand has moved, not as the menu goes (the product's file row).
Item {
    id: root
    width: 600
    height: 400

    HandWatch { id: windowHand }
    TipSeat {
        id: seat
        host: root
        hand: windowHand
    }

    // A pane whose rows scroll, its hand on the pane as the app's panes keep theirs.
    Item {
        id: pane
        width: 200
        height: 120
        HandWatch {
            id: paneHand
        }
        AppListView {
            id: list
            anchors.fill: parent
            model: 30
            rowsHand: paneHand
            onContentYChanged: paneHand.rowsMoved()
            delegate: Item {
                id: row
                required property int index
                width: list.width
                height: 24
                MouseArea {
                    id: area
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.NoButton
                }
                readonly property alias tip: ask
                readonly property TipAsk ask: TipAsk {
                    id: ask
                    target: row
                    asked: area.containsMouse
                    words: "row " + row.index
                }
            }
        }
    }
    // A target that does not move, set apart from the list.
    Item {
        id: still
        x: 300
        y: 20
        width: 200
        height: 40
        HoverHandler { id: onStill }
        readonly property TipAsk stillTip: TipAsk { target: still; asked: onStill.hovered; words: "still" }
    }
    // A commit's changed file, the product's own row (`DetailsPane.fileList`): its box says the path.
    ListModel {
        id: files
        ListElement {
            path: "src/far/away/a.rs"; name: "a.rs"; change: "M"; folder: false; collapsed: false; depth: 0
            orig_path: ""; orig_name: ""
        }
    }
    AppListView {
        id: fileList
        x: 300
        y: 100
        width: 200
        height: 48
        model: files
        delegate: FileRowDelegate {
            listWidth: fileList.width
        }
    }
    // A name box's refusal: what is typed asks, not a hand.
    Item {
        id: typedBox
        x: 300
        y: 200
        width: 120
        height: 24
        property bool refused: false
        readonly property TipAsk typedTip: TipAsk {
            target: typedBox; typed: true; asked: typedBox.refused; words: "A branch called that already exists"
        }
    }
    AppMenu {
        id: menu
        AppMenuItem {
            text: "plain"
        }
        AppMenuItem {
            id: heldRow
            text: "held"
            blockedReason: "Not while the other one runs"
        }
    }

    function tipSays() {
        return seat.box.visible ? seat.box.text : ""
    }

    TestCase {
        name: "TipMoves"
        when: windowShown

        function init() {
            list.contentY = 0
            typedBox.refused = false
            heldRow.tipForced = false
            if (menu.visible)
                menu.close()
            // Away from everything, and the box down.
            mouseMove(root, root.width - 2, root.height - 2)
            tryVerify(() => !seat.box.visible, undefined, "no box at the start")
        }

        function test_a_row_that_arrived_under_a_still_hand_asks_nothing_until_the_hand_moves() {
            mouseMove(list, 100, 36)
            tryVerify(() => root.tipSays() === "row 1", undefined, "the hand resting on row 1 brings its tip")

            // What a wheel notch, a find landing off screen, a fold above does: the rows go, the hand stays.
            list.contentY = 48
            verify(!seat.box.visible, "the rows moving take the box down in that turn, no beat")
            // waits(paced): the subject is a box that must **not** come: the rest it would count, twice over.
            wait(Metrics.tipDelayMs * 2)
            verify(!seat.box.visible, "row 3, which Qt now says the hand is on, asks nothing under a still hand")

            mouseMove(list, 102, 36)
            tryVerify(() => root.tipSays() === "row 3", undefined,
                      "the hand's own move: the row under it asks, after the rest")
        }

        function test_a_box_behind_a_menu_goes_as_the_menu_stands_and_the_menus_own_row_asks() {
            mouseMove(still, 100, 20)
            tryVerify(() => root.tipSays() === "still", undefined, "the fixed target's tip is out")
            menu.popup(root, 20, 200)
            verify(menu.visible, "a menu stands")
            verify(!seat.box.visible, "what was out went the moment it stood")

            // A row of the menu asks, as its hover does (`AppMenuItem.tipForced`): inside the menu, not behind it.
            heldRow.tipForced = true
            tryVerify(() => root.tipSays() === "Not while the other one runs", undefined,
                      "the menu's own row has its tip")
            menu.close()
        }

        // The menu is the reader's input, so what it took down is not put back under the same still hand when it goes
        // (`HoverSeat.onBlockedByChanged`). Raised over the hand, as a right-click raises it: the row's own hover goes
        // under the menu, and that leave is no leave of the hand's (`HoverSeat.unpoint`).
        function test_a_file_rows_box_a_menu_took_down_waits_for_the_hand_to_move() {
            const row = fileList.itemAtIndex(0)
            mouseMove(row, 60, 12)
            tryVerify(() => seat.box.visible, undefined, "the hand resting on the commit's file row brings its path")
            const at = row.mapToItem(root, 60, 12)
            menu.popup(root, at.x, at.y)
            verify(!seat.box.visible, "the box goes the moment the menu stands")
            menu.close()
            // waits(paced): the subject is a box that must **not** come back under the still hand: its rest, twice
            // over.
            wait(Metrics.tipDelayMs * 2)
            verify(!seat.box.visible, "the hand has not moved: the box the menu took down stays down")
            mouseMove(row, 62, 12)
            tryVerify(() => seat.box.visible, undefined, "the hand's own move brings it back after the rest")
        }

        function test_a_refusal_of_what_is_typed_stands_under_a_menu() {
            typedBox.refused = true
            tryVerify(() => seat.box.visible && seat.box.text === "A branch called that already exists",
                      undefined, "the refusal is out")
            menu.popup(root, 20, 200)
            verify(menu.visible)
            verify(seat.box.visible, "no hover opened it, and the menu standing takes nothing of it")
            menu.close()
        }
    }
}
