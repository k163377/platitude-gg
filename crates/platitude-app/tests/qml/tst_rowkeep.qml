import QtQuick
import QtQuick.Layouts
import QtTest
import platitude.ui

// The row under a pane's hand kept where it was through a read (`RowKeep`, デザイン規約 §hover のツールチップ「外乱では閉じ
// ない」): rows a read wrote in or took out above it are taken back by the scroll, as far as it goes; what the scroll
// cannot take back moves under the hand as the read's (`HandWatch.rowsMoved(true)`). The lists are told row by row, as
// the models tell them (`NavSectionModel.reshape`) — a `ListModel`'s inserts and removes, never a reset.
Item {
    id: root
    width: 640
    height: 420

    /// What the hand heard of each move of the rows, in order: whether it came of a read (`HandWatch.disturbed`).
    property var shifted: []

    HandWatch {
        id: hand
    }
    Connections {
        target: hand
        function onRowsShifted() {
            root.shifted.push(hand.disturbed)
        }
    }

    /// A band over the lists that a read can grow, stepping them down the pane (a section above growing).
    Rectangle {
        id: band
        width: 300
        height: 0
        color: "transparent"
    }

    ListModel {
        id: rows
    }
    AppListView {
        id: list
        y: band.height
        width: 300
        height: 200
        model: rows
        rowsHand: hand
        onContentYChanged: if (!keep.busy) hand.rowsMoved()
        delegate: Rectangle {
            required property string name
            required property int index
            property bool pooled: false
            ListView.onPooled: pooled = true
            ListView.onReused: pooled = false
            width: list.width
            height: Theme.rowHeight
            color: "transparent"
        }
    }

    readonly property RowKeep keep: RowKeep {
        id: keep
        hand: hand
        views: [list]
    }

    /// Two lists stacked in a column, each as tall as its rows (the left panel's sections): a row read into the upper
    /// one steps the lower one down the pane — once the upper one has counted its rows, at the column's next polish.
    ListModel {
        id: upperRows
    }
    ListModel {
        id: lowerRows
    }
    ColumnLayout {
        id: stack
        x: 340
        width: 280
        spacing: 0
        AppListView {
            id: upper
            Layout.fillWidth: true
            Layout.preferredHeight: count * Theme.rowHeight
            model: upperRows
            rowsHand: hand
            delegate: Rectangle {
                required property string name
                required property int index
                width: upper.width
                height: Theme.rowHeight
                color: "transparent"
            }
        }
        AppListView {
            id: lower
            Layout.fillWidth: true
            Layout.preferredHeight: count * Theme.rowHeight
            model: lowerRows
            rowsHand: hand
            delegate: Rectangle {
                required property string name
                required property int index
                property bool pooled: false
                ListView.onPooled: pooled = true
                ListView.onReused: pooled = false
                width: lower.width
                height: Theme.rowHeight
                color: "transparent"
            }
        }
    }
    readonly property RowKeep stackKeep: RowKeep {
        hand: hand
        views: [upper, lower]
        layout: stack
    }

    TestCase {
        name: "RowKeep"
        when: windowShown

        function init() {
            band.height = 0
            list.reuseItems = true
            rows.clear()
            upperRows.clear()
            lowerRows.clear()
            list.contentY = 0
            root.shifted = []
            // The hand parked off the lists.
            mouseMove(root, root.width / 2, root.height - 2)
            tryVerify(() => hand.known && !list.contains(list.mapFromItem(root, hand.at.x, hand.at.y)))
        }
        function fill(n) {
            for (let i = 0; i < n; i++)
                rows.append({ "name": "row " + i })
            list.forceLayout()
        }
        /// The delegate showing `name`, null while the view has not built it.
        function rowNamed(name) {
            for (let i = 0; i < list.count; i++) {
                const row = list.itemAtIndex(i)
                if (row !== null && row.name === name)
                    return row
            }
            return null
        }
        /// The hand onto `name`'s row, and the read `change` makes under it, weighed as the panes weigh one.
        function handOn(name) {
            const row = rowNamed(name)
            verify(row !== null, name + " is built")
            const at = row.mapToItem(root, row.width / 2, row.height / 2)
            mouseMove(root, at.x, at.y)
            tryVerify(() => hand.over(row), undefined, "the hand is on " + name)
            root.shifted = []
            return row
        }
        function read(change) {
            keep.arriving()
            change()
            keep.arrived()
            tryVerify(() => !keep.busy, undefined, "the read is weighed")
        }

        function test_rows_written_in_between_the_top_and_the_hand_are_taken_back_by_the_scroll() {
            fill(40)
            list.contentY = 10 * Theme.rowHeight
            list.forceLayout()
            const row = handOn("row 13")
            const from = list.contentY
            read(() => rows.insert(11, { "name": "new" }))
            verify(hand.over(row), "the same row is under the hand")
            compare(list.contentY, from + Theme.rowHeight, "the view went down by the row written in")
            compare(root.shifted.length, 0, "nothing moved under the hand")
            verify(!hand.stillHeld, "and the hand is not held")
        }

        function test_rows_written_in_above_the_screen_move_nothing() {
            fill(40)
            list.contentY = 10 * Theme.rowHeight
            list.forceLayout()
            const row = handOn("row 13")
            const at = row.mapToItem(root, 0, 0).y
            read(() => rows.insert(0, { "name": "new" }))
            verify(hand.over(row), "the same row is under the hand")
            compare(row.mapToItem(root, 0, 0).y, at, "where it stood")
            compare(root.shifted.length, 0, "and nothing moved under the hand")
        }

        function test_what_the_scroll_cannot_take_back_moves_under_the_hand_as_the_reads() {
            fill(5)
            const row = handOn("row 2")
            read(() => rows.insert(0, { "name": "new" }))
            verify(!hand.over(row), "the row went down from under the hand")
            compare(root.shifted, [true], "as the read's")
            verify(hand.stillHeld, "and the hand is held until it moves")
        }

        function test_the_row_a_read_takes_out_moves_the_rows_under_the_hand_as_the_reads() {
            fill(5)
            handOn("row 2")
            read(() => rows.remove(2))
            compare(root.shifted, [true], "the row that came under the hand came with the read")
            verify(hand.stillHeld)
        }

        function test_the_rows_delegate_deleted_before_the_read_is_weighed_still_moves_the_rows_as_the_reads() {
            // A view keeping no delegate for reuse deletes the one of the row taken out, and Qt may have deleted it by
            // the time the read is weighed.
            list.reuseItems = false
            fill(5)
            handOn("row 2")
            keep.arriving()
            rows.remove(2)
            list.forceLayout()
            // Qt deletes it as the events are next taken, ahead of the turn the read is weighed in.
            tryVerify(() => keep.row === null, undefined, "the delegate is deleted")
            keep.arrived()
            tryVerify(() => !keep.busy, undefined, "the read is weighed")
            compare(root.shifted, [true], "the row that came under the hand came with the read")
            verify(hand.stillHeld)
        }

        function test_a_row_a_read_brings_under_a_hand_on_none_comes_as_the_reads() {
            fill(3)
            // The hand in the list, under its last row.
            mouseMove(list, list.width / 2, 3.5 * Theme.rowHeight)
            tryVerify(() => hand.known && keep.rowUnder(hand.at) === null, undefined, "the hand is on no row")
            root.shifted = []
            read(() => rows.insert(0, { "name": "new" }))
            list.forceLayout()
            verify(keep.rowUnder(hand.at) !== null, "a row is under the hand now")
            compare(root.shifted, [true], "and it came with the read")
            verify(hand.stillHeld)
        }

        function test_the_list_stepping_down_with_the_read_is_taken_back() {
            fill(40)
            list.contentY = 10 * Theme.rowHeight
            list.forceLayout()
            const row = handOn("row 13")
            const at = row.mapToItem(root, 0, 0).y
            read(() => {
                band.height = Theme.rowHeight
            })
            compare(row.mapToItem(root, 0, 0).y, at, "the row stands where it stood")
            compare(root.shifted.length, 0, "nothing moved under the hand")
        }

        function test_with_no_hand_over_the_rows_the_read_moves_them_as_it_does() {
            fill(5)
            const row = rowNamed("row 2")
            const at = row.mapToItem(root, 0, 0).y
            read(() => rows.insert(0, { "name": "new" }))
            list.forceLayout()
            compare(row.mapToItem(root, 0, 0).y, at + Theme.rowHeight, "the row went down with the row above")
            compare(root.shifted.length, 0, "and no hand was told")
        }

        /// What the keeper reads a row's place off (Qt's, pinned here): a delegate's `index` follows its row as the
        /// model tells its inserts and removes, before the view lays the read out — and a delegate whose row went is
        /// told so (-1).
        function test_a_delegates_index_follows_its_row_before_the_view_lays_the_read_out() {
            fill(5)
            const kept = rowNamed("row 3")
            const taken = rowNamed("row 1")
            rows.insert(0, { "name": "new" })
            compare(kept.index, 4, "the row's place, one down")
            rows.remove(2)
            compare(taken.index, -1, "the row taken out has none")
            list.forceLayout()
        }

        function test_a_row_pushed_past_the_views_cache_by_the_read_is_brought_back_under_the_hand() {
            fill(200)
            list.contentY = 50 * Theme.rowHeight
            list.forceLayout()
            handOn("row 55")
            const at = rowNamed("row 55").mapToItem(root, 0, 0).y
            // Forty rows written in between the top of the screen and the hand's: the hand's row goes past the view's
            // cache — and the view lays the read out before the keeper's turn comes, pooling its delegate.
            keep.arriving()
            for (let i = 0; i < 40; i++)
                rows.insert(52, { "name": "new " + i })
            keep.arrived()
            list.forceLayout()
            tryVerify(() => !keep.busy, undefined, "the read is weighed")
            const row = rowNamed("row 55")
            verify(row !== null, "the row is built")
            compare(row.mapToItem(root, 0, 0).y, at, "under the hand")
            verify(hand.over(row))
            compare(root.shifted.length, 0, "and nothing moved under the hand")
        }

        /// …by where it stood as the view pooled its delegate, which the view may use again for another row before the
        /// keeper's turn — another read landing then reads nothing off it.
        function test_a_row_whose_delegate_went_on_to_another_row_before_another_read_landed_is_brought_back() {
            fill(200)
            list.contentY = 50 * Theme.rowHeight
            list.forceLayout()
            const kept = handOn("row 55")
            const at = kept.mapToItem(root, 0, 0).y
            keep.arriving()
            for (let i = 0; i < 40; i++)
                rows.insert(52, { "name": "new " + i })
            keep.arrived()
            list.forceLayout()
            verify(kept.pooled, "the row's delegate is in the pool")
            // The view's next layouts, as a scroll to the list's end makes them: the delegate goes on to another row.
            list.contentY = list.clampY(list.contentHeight)
            list.forceLayout()
            verify(!kept.pooled && kept.name !== "row 55", "the delegate stands for another row")
            // Another section's read, landing before the keeper's turn.
            keep.arriving()
            keep.arrived()
            tryVerify(() => !keep.busy, undefined, "the reads are weighed")
            const row = rowNamed("row 55")
            verify(row !== null, "the row is built")
            compare(row.mapToItem(root, 0, 0).y, at, "under the hand")
        }

        /// The model tells a read from its arriving to its rows being in (`RowKeep.telling` — what a row asks of the
        /// model then waits a turn); the hand hears the read moving its rows until it is weighed (`HandWatch.reading`).
        function test_the_read_is_told_until_its_rows_are_in_and_moves_the_rows_until_it_is_weighed() {
            fill(5)
            keep.arriving()
            verify(keep.telling && hand.reading, "the read is told, moving the rows")
            keep.arrived()
            verify(!keep.telling, "its rows are in")
            verify(hand.reading, "the read moves the rows until it is weighed")
            tryVerify(() => !keep.busy, undefined, "the read is weighed")
            verify(!hand.reading)
        }

        function test_a_row_read_into_the_section_above_steps_the_hands_row_down_as_the_reads() {
            for (let i = 0; i < 3; i++) {
                upperRows.append({ "name": "upper " + i })
                lowerRows.append({ "name": "lower " + i })
            }
            tryVerify(() => upper.count === 3 && lower.count === 3 && lower.itemAtIndex(1) !== null
                      && lower.y === 3 * Theme.rowHeight, undefined, "the column is laid out")
            const row = lower.itemAtIndex(1)
            const at = row.mapToItem(root, row.width / 2, row.height / 2)
            mouseMove(root, at.x, at.y)
            tryVerify(() => hand.over(row), undefined, "the hand is on the lower list's row")
            root.shifted = []
            root.stackKeep.arriving()
            upperRows.append({ "name": "upper 3" })
            root.stackKeep.arrived()
            tryVerify(() => !root.stackKeep.busy, undefined, "the read is weighed")
            compare(root.shifted, [true], "the lower list stepped down under the hand, as the read's")
            verify(hand.stillHeld)
        }
    }
}
