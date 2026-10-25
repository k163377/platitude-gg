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

    /// The rows are moving for a read, not a reader (`HandWatch.rowsMoved(true)`, as `RowKeep` tells what it cannot
    /// take back).
    property bool reading: false

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
            onContentYChanged: paneHand.rowsMoved(root.reading)
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
    // A list a read writes rows into, weighed by its keeper (`RowKeep`), its rows' tips dropped as the list pools them
    // (`NavItemDelegate.pooled`).
    ListModel {
        id: readRows
    }
    Item {
        id: readPane
        y: 200
        width: 200
        height: 120
        HandWatch {
            id: readHand
        }
        AppListView {
            id: readList
            anchors.fill: parent
            model: readRows
            rowsHand: readHand
            onContentYChanged: if (!root.readKeep.busy) readHand.rowsMoved()
            delegate: Item {
                id: readRow
                required property string name
                required property int index
                property bool pooled: false
                ListView.onPooled: {
                    readRow.pooled = true
                    Tips.dropIn(readRow)
                }
                ListView.onReused: readRow.pooled = false
                width: readList.width
                height: 24
                MouseArea {
                    id: readArea
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.NoButton
                }
                readonly property TipAsk ask: TipAsk {
                    target: readRow
                    asked: readArea.containsMouse
                    words: readRow.name
                }
            }
        }
    }
    readonly property RowKeep readKeep: RowKeep {
        hand: readHand
        views: [readList]
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
        id: testCase
        name: "TipMoves"
        when: windowShown

        /// A row's delegate the read list pooled, held where Qt nulls it once it deletes the delegate.
        property Item pooledRow: null

        function init() {
            root.reading = false
            testCase.pooledRow = null
            readPane.width = 200
            readRows.clear()
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

        /// A read that moves the rows from under the hand reading a row's box leaves the box where it stands: off its
        /// row, which would take it from under the hand (`TipSeat.park`, デザイン規約 §hover のツールチップ「外乱では閉じ
        /// ない」). It says what it said, and goes once the hand leaves it.
        function test_a_box_the_hand_is_in_stays_where_it_stands_as_a_read_moves_its_row() {
            mouseMove(list, 100, 36)
            tryVerify(() => root.tipSays() === "row 1", undefined, "the hand resting on row 1 brings its tip")
            // Read off the box's place, the same on two looks in a row: a frame waited for may never come over a scene
            // at rest.
            let last = ""
            tryVerify(() => {
                const p = seat.box.contentItem.mapToItem(root, 0, 0)
                const now = [p.x, p.y, seat.box.contentItem.width, seat.box.contentItem.height].join(",")
                const stood = seat.box.contentItem.width > 0 && now === last
                last = now
                return stood
            }, undefined, "the box stands at its size")
            const at = seat.box.contentItem.mapToItem(root, 0, 0)
            const inside = seat.box.contentItem.mapToItem(root, 2, seat.box.contentItem.height / 2)
            mouseMove(root, inside.x, inside.y)
            tryVerify(() => seat.pointed, undefined, "the hand is in the box")

            root.reading = true
            list.contentY = 48
            root.reading = false
            verify(seat.box.visible, "the box stays")
            compare(seat.box.contentItem.mapToItem(root, 0, 0), at, "where it stood")
            // waits(paced): a box that must **not** go: the beat a leave starts, twice over.
            wait(Metrics.hoverKeepMs * 2)
            compare(root.tipSays(), "row 1", "saying what it said")

            mouseMove(root, root.width - 2, root.height - 2)
            tryVerify(() => !seat.box.visible, undefined, "and goes once the hand leaves it")
        }

        /// The read list scrolled 50 rows down, the hand resting on `item 52` and walked into its box — read off the
        /// box's place, the same on two looks in a row (a frame waited for may never come over a scene at rest).
        function handInReadBox() {
            for (let i = 0; i < 200; i++)
                readRows.append({ "name": "item " + i })
            readList.forceLayout()
            // From the list's first row: its origin moves as rows come and go above what is on screen.
            readList.positionViewAtBeginning()
            readList.contentY = readList.originY + 50 * 24
            readList.forceLayout()
            mouseMove(readList, 100, 2 * 24 + 12)
            tryVerify(() => root.tipSays() === "item 52", undefined, "the hand resting on item 52 brings its tip")
            let last = ""
            tryVerify(() => {
                const p = seat.box.contentItem.mapToItem(root, 0, 0)
                const now = [p.x, p.y, seat.box.contentItem.width, seat.box.contentItem.height].join(",")
                const stood = seat.box.contentItem.width > 0 && now === last
                last = now
                return stood
            }, undefined, "the box stands at its size")
            const inside = seat.box.contentItem.mapToItem(root, 2, seat.box.contentItem.height / 2)
            mouseMove(root, inside.x, inside.y)
            tryVerify(() => seat.pointed, undefined, "the hand is in the box")
            return seat.box.contentItem.mapToItem(root, 0, 0)
        }

        /// A read writing forty rows in between the top of the read list and the hand's `item 52`, which lands at 92:
        /// the view lays it out before the keeper's turn comes, pooling the row's delegate.
        function pushPastTheCache() {
            root.readKeep.arriving()
            for (let i = 0; i < 40; i++)
                readRows.insert(51, { "name": "new " + i })
            root.readKeep.arrived()
            readList.forceLayout()
        }
        /// Whether `item` is `outer` or inside it.
        function inside(item, outer) {
            for (let at = item; at !== null && at !== undefined; at = at.parent) {
                if (at === outer)
                    return true
            }
            return false
        }

        /// A read that pushes the row past the list's cache pools its delegate, yet the row is still there: the box the
        /// hand is reading stays where it stands (`Tips.dropIn` holds it off the delegate, `RowKeep` brings the row
        /// back).
        function test_a_box_the_hand_is_in_stays_as_a_read_pushes_its_row_past_the_lists_cache() {
            const at = handInReadBox()
            pushPastTheCache()
            tryVerify(() => !root.readKeep.busy, undefined, "the read is weighed")
            verify(seat.box.visible, "the box stays")
            compare(root.tipSays(), "item 52", "saying what it said")
            compare(seat.box.contentItem.mapToItem(root, 0, 0), at, "where it stood")
        }

        /// …saying what its row said, though the view goes on to use the pooled delegate for another row and hands it
        /// that row's words (`TipSeat.detach`).
        function test_a_box_held_off_its_rows_pooled_delegate_says_what_its_row_said() {
            handInReadBox()
            const pooled = readList.itemAtIndex(52)
            pushPastTheCache()
            verify(pooled.pooled, "the row's delegate is in the pool")
            pooled.name = "another"
            compare(root.tipSays(), "item 52", "the box says what its row said")
            tryVerify(() => !root.readKeep.busy, undefined, "the read is weighed")
            compare(root.tipSays(), "item 52", "and does with its row stood again")
        }

        /// …and stays as the list lets its pool go: a list that moves or changes size deletes the delegates it pooled
        /// (`QQuickListView::geometryChange`).
        function test_a_box_held_off_its_rows_pooled_delegate_stays_as_the_list_lets_its_pool_go() {
            const at = handInReadBox()
            testCase.pooledRow = readList.itemAtIndex(52)
            pushPastTheCache()
            verify(testCase.pooledRow.pooled, "the row's delegate is in the pool")
            readPane.width = 201
            tryVerify(() => testCase.pooledRow === null, undefined, "the list deleted it")
            tryVerify(() => !root.readKeep.busy, undefined, "the read is weighed")
            verify(seat.box.visible, "the box stays")
            compare(root.tipSays(), "item 52", "saying what its row said")
            compare(seat.box.contentItem.mapToItem(root, 0, 0), at, "where it stood")
        }

        /// …and, the row stood again, the row's own ask takes the box back on (`TipSeat.takeBack`): the box says the
        /// row's words as they change (デザイン規約 §hover のツールチップ「外乱では閉じない」: what stays changes its words),
        /// and the hand walking back onto the row is on the target the box stands for — it stays up.
        function test_a_box_held_off_its_rows_pooled_delegate_goes_back_onto_its_row_stood_again() {
            handInReadBox()
            pushPastTheCache()
            tryVerify(() => !root.readKeep.busy, undefined, "the read is weighed")
            const row = readList.itemAtIndex(92)
            verify(row !== null && row.name === "item 52", "the row stands again")
            verify(testCase.inside(seat.current.target, row), "the box is the row's again")
            readRows.setProperty(92, "name", "item 52, renamed")
            compare(root.tipSays(), "item 52, renamed", "the box says the row's words")

            let closes = 0
            const closed = () => { closes++ }
            seat.box.closed.connect(closed)
            const on = row.mapToItem(readList, 100, 12)
            mouseMove(readList, on.x, on.y)
            mouseMove(readList, on.x + 2, on.y)
            tryVerify(() => row.ask.asked, undefined, "the hand is back on the row")
            // waits(paced): a box that must **not** go: the beat a leave starts, twice over.
            wait(Metrics.hoverKeepMs * 2)
            seat.box.closed.disconnect(closed)
            compare(closes, 0, "the box stayed up through the walk back")
            verify(seat.box.visible)
        }

        /// A row that leaves the list for no read takes down the box the hand is reading off it: its delegate stands
        /// for something else from here (`Tips.dropIn`).
        function test_a_box_the_hand_is_in_goes_as_its_row_leaves_the_list_for_no_read() {
            handInReadBox()
            const pooled = readList.itemAtIndex(52)
            for (let i = 0; i < 40; i++)
                readRows.insert(51, { "name": "new " + i })
            readList.forceLayout()
            verify(pooled.pooled, "the row's delegate is in the pool")
            tryVerify(() => !seat.box.visible, undefined, "the box went")
        }

        /// …and one whose row the read took out goes, the hand in it or not: what it said is gone.
        function test_a_box_the_hand_is_in_goes_with_its_row() {
            handInReadBox()
            root.readKeep.arriving()
            readRows.remove(52)
            root.readKeep.arrived()
            readList.forceLayout()
            tryVerify(() => !root.readKeep.busy, undefined, "the read is weighed")
            tryVerify(() => !seat.box.visible, undefined, "the box went with its row")
        }

        /// …and with the hand on the row, not in the box, the row going from under the hand takes the box with it.
        function test_a_read_that_moves_the_hands_row_takes_its_box_down() {
            mouseMove(list, 100, 36)
            tryVerify(() => root.tipSays() === "row 1", undefined, "the hand resting on row 1 brings its tip")
            root.reading = true
            list.contentY = 48
            root.reading = false
            verify(!seat.box.visible, "the box went with its row's place")
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
