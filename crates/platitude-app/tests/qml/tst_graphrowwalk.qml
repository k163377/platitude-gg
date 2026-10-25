import QtQuick
import QtTest
import platitude.ui

// The graph keeping the reader's place across a reread (`GraphRowWalk.shiftRows`): commits written in above the rows
// put the view back over them in the same turn, at the top only for a hand over the rows — the view stays on the
// newest otherwise — and what the view cannot take back moves under the hand as the reread's (デザイン規約 §hover の
// ツールチップ「外乱では閉じない」). How far is the hand's row's to say (`handShift`). The real walk, over a list cut
// down to what it reads.
Item {
    id: root
    width: 400
    height: Theme.graphRowHeight * 8

    /// What the hand heard of each move of the rows, in order: whether it came of a reread (`HandWatch.disturbed`).
    property var shifted: []
    /// What the rows heard of the view as they moved: putting itself back over them, or laying out a reread it does
    /// not take back.
    property var laidOut: []
    readonly property string worktree: "0000000000000000000000000000000000000000"

    ListModel {
        id: rows
    }
    /// `n` commits, `c0` the newest.
    function fill(n) {
        rows.clear()
        for (let i = 0; i < n; i++)
            rows.append({ "oid_hex": "c" + i })
    }
    /// `n` commits written in at row `at` as the graph's reread writes them (`splice_notified`): `n` rows more at the
    /// end, then every row from `at` down rewritten in place, from the last up — no row is inserted, so no row the view
    /// had built moves for it.
    function writeIn(at, n) {
        const before = []
        for (let i = 0; i < rows.count; i++)
            before.push(rows.get(i).oid_hex)
        for (let i = 0; i < n; i++)
            rows.append({ "oid_hex": "" })
        for (let i = rows.count - 1; i >= at; i--)
            rows.setProperty(i, "oid_hex", i >= at + n ? before[i - n] : "n" + (i - at))
    }
    /// The middle of row `row`'s place, in the scene.
    function over(row) {
        return Qt.point(10, row * Theme.graphRowHeight + Theme.graphRowHeight / 2 - list.contentY)
    }

    HandWatch {
        id: hand
    }
    Connections {
        target: hand
        function onRowsShifted() {
            root.shifted.push(hand.disturbed)
        }
    }

    // `GraphList` cut down to what the walk reads and what the pane makes of its moves (`GraphPane`).
    ListView {
        id: list
        width: root.width
        height: Theme.graphRowHeight * 8
        model: rows
        delegate: Item {
            width: list.width
            height: Theme.graphRowHeight
            onYChanged: root.laidOut.push(list.keepingRows ? "kept" : list.rereading ? "reread" : "moved")
        }
        readonly property HandWatch hand: hand
        property bool keepingRows: false
        property bool rereading: false
        function haltGlide() {}
        function clampY(y) {
            const minY = list.originY - list.topMargin
            const maxY = Math.max(minY, list.originY + list.contentHeight - list.height + list.bottomMargin)
            return Math.max(minY, Math.min(y, maxY))
        }
        function rowOnScreen(row) {
            return false
        }
        onContentYChanged: {
            if (list.keepingRows)
                hand.rowsKept()
            else
                hand.rowsMoved(list.rereading)
        }
    }
    // The current branch's stand-in cut down to what the walk asks of it (`GraphHeadPin.topBand` / `aboveAt`): the band
    // it covers at the top edge, its fade included, and whether HEAD's row is above the view at a given place.
    readonly property QtObject pin: QtObject {
        id: pin
        property int headRow: -1
        readonly property real topBand: list.topMargin + Theme.graphRowHeight * 5 / 2
        function aboveAt(contentY) {
            return pin.headRow >= 0 && list.originY + pin.headRow * Theme.graphRowHeight < contentY
        }
    }
    GraphRowWalk {
        id: walk
        view: list
        standIn: pin
        graphModel: QtObject {
            property bool provisionalOn: false
            function oidAt(row) {
                return row >= 0 && row < rows.count ? rows.get(row).oid_hex : ""
            }
            function rowOf(oidHex) {
                for (let i = 0; i < rows.count; i++) {
                    if (rows.get(i).oid_hex === oidHex)
                        return i
                }
                return -1
            }
        }
        asking: false
    }

    TestCase {
        name: "GraphRowWalk"
        when: windowShown

        function init() {
            hand.at = Qt.point(-1, -1)
            hand.covered = 0
            fill(40)
            list.contentY = 0
            list.forceLayout()
            hand.stillHeld = false
            walk.handOid = ""
            walk.handRow = -1
            pin.headRow = -1
            root.shifted = []
            root.laidOut = []
        }

        function test_at_the_top_the_view_stays_on_the_newest_with_no_hand_over_the_rows() {
            walk.shiftRows(1)
            compare(list.contentY, 0, "the newest row is the one at the top")
            compare(root.shifted, [], "and nothing was told")
        }

        function test_at_the_top_a_hand_over_the_rows_keeps_them_where_they_are() {
            hand.at = Qt.point(10, 10)
            walk.shiftRows(1)
            compare(list.contentY, Theme.graphRowHeight, "the view put itself back over the rows")
            verify(!hand.stillHeld, "nothing moved under the hand")
            compare(root.shifted, [], "nor was a move told")
        }

        function test_a_hand_in_a_card_over_the_rows_is_over_them() {
            hand.covered = 1
            walk.shiftRows(2)
            compare(list.contentY, 2 * Theme.graphRowHeight)
        }

        function test_down_the_rows_the_view_keeps_them_with_no_hand_over_them() {
            list.contentY = 5 * Theme.graphRowHeight
            root.shifted = []
            walk.shiftRows(1)
            compare(list.contentY, 6 * Theme.graphRowHeight)
            compare(root.shifted, [])
        }

        function test_what_the_view_cannot_take_back_moves_under_the_hand_as_the_rereads() {
            // The history no taller than the view: nothing to scroll.
            fill(8)
            list.forceLayout()
            hand.at = Qt.point(10, 10)
            walk.shiftRows(1)
            compare(list.contentY, 0, "the view could not take it back")
            verify(hand.stillHeld, "the rows moved under the hand")
            compare(root.shifted, [true], "as the reread's")
        }

        /// The reread's rows are laid out as the view weighs them, not at the next frame: the row it built at the end
        /// lands as the view takes the shift back where it does…
        function test_a_row_a_reread_builds_lands_as_the_view_takes_the_rows_back() {
            fill(5)
            list.forceLayout()
            hand.at = root.over(1)
            root.writeIn(0, 1)
            root.laidOut = []
            walk.shiftRows(1)
            verify(root.laidOut.length > 0, "the row built at the end landed as the view weighed the shift")
            verify(root.laidOut.every(how => how === "kept"), "taken back with the view: " + root.laidOut)
        }
        /// …and as the reread's own move where it does not: at the top with no hand over the rows…
        function test_a_row_a_reread_builds_at_the_pinned_top_lands_as_the_rereads() {
            fill(5)
            list.forceLayout()
            root.writeIn(0, 1)
            root.laidOut = []
            walk.shiftRows(1)
            verify(root.laidOut.length > 0, "the row built at the end landed as the view weighed the shift")
            verify(root.laidOut.every(how => how === "reread"), "as the reread's: " + root.laidOut)
        }
        /// …and where nothing was written in above the rows at all.
        function test_a_row_a_reread_builds_with_nothing_above_lands_as_the_rereads() {
            fill(5)
            list.forceLayout()
            root.writeIn(3, 1)
            root.laidOut = []
            walk.shiftRows(0)
            verify(root.laidOut.length > 0, "the row built at the end landed as the view weighed the shift")
            verify(root.laidOut.every(how => how === "reread"), "as the reread's: " + root.laidOut)
        }

        /// The shift is the hand's row's: commits written in between the newest and the row the hand is over move that
        /// row down by as many…
        function test_commits_written_in_above_the_hands_row_are_its_shift() {
            hand.at = root.over(3)
            walk.noteHand()
            compare(walk.handOid, "c3", "the hand is over the fourth commit")
            root.writeIn(1, 1)
            compare(walk.handShift(), 1, "one commit written in above it")
        }
        /// …and commits written in below it leave it where it is.
        function test_commits_written_in_below_the_hands_row_leave_it() {
            hand.at = root.over(3)
            walk.noteHand()
            root.writeIn(5, 1)
            compare(walk.handShift(), 0)
        }
        /// The worktree's row stays at the top whatever is written in under it.
        function test_the_worktrees_row_under_the_hand_never_moves() {
            rows.set(0, { "oid_hex": root.worktree })
            hand.at = root.over(0)
            walk.noteHand()
            root.writeIn(1, 1)
            compare(walk.handShift(), 0)
        }
        /// A hand gone into a card is weighed by the row it was last heard over, which each pass finds again.
        function test_a_hand_in_a_card_is_weighed_by_the_row_it_was_last_heard_over() {
            hand.at = root.over(2)
            walk.noteHand()
            hand.at = Qt.point(-1, -1)
            hand.covered = 1
            walk.noteHand()
            root.writeIn(0, 1)
            compare(walk.handShift(), 1, "the commit it was over is a row down")
            walk.noteHand()
            root.writeIn(0, 1)
            compare(walk.handShift(), 1, "found again after the pass: one more, not two")
        }
        /// HEAD's row under the hand at the top, and a commit made on its branch elsewhere: taking it back would take
        /// that row under the stand-in rising at the top edge (`GraphHeadPin.topBand`, its fade included), so the view
        /// gives back what the band covers — up to three commits, the rows come down as they came (デザイン規約
        /// §hover のツールチップ「外乱では閉じない」).
        function test_a_few_commits_on_the_branch_under_the_hand_move_the_rows_down() {
            pin.headRow = 0
            hand.at = root.over(0)
            walk.noteHand()
            root.writeIn(0, 1)
            walk.shiftRows(1)
            compare(list.contentY, 0, "the view stayed on the newest")
            verify(hand.stillHeld, "the rows came down under the hand")
            compare(root.shifted, [true], "as the reread's")
        }
        /// …and more of them: the view takes back all but those, and the hand's row stands just clear of the band.
        function test_more_commits_bring_the_hands_row_to_just_under_the_stand_in() {
            pin.headRow = 0
            hand.at = root.over(0)
            walk.noteHand()
            root.writeIn(0, 5)
            walk.shiftRows(5)
            compare(list.contentY, 2 * Theme.graphRowHeight, "two of the five taken back")
            const top = 5 * Theme.graphRowHeight - list.contentY
            verify(top >= pin.topBand, "the hand's row stands clear of the stand-in: " + top)
            verify(top - Theme.graphRowHeight < pin.topBand, "and no further down than that: " + top)
            compare(root.shifted, [true], "the rest came down under the hand as the reread's")
        }
        /// Commits that leave HEAD where it was raise no stand-in: every one is taken back.
        function test_commits_above_a_head_that_stays_are_all_taken_back() {
            hand.at = root.over(0)
            walk.noteHand()
            root.writeIn(0, 1)
            root.writeIn(0, 1)
            pin.headRow = 2
            walk.shiftRows(2)
            compare(list.contentY, 2 * Theme.graphRowHeight)
            compare(root.shifted, [])
        }
        /// The stand-in already up over a view down the rows does not rise with the shift: every commit is taken back,
        /// and the hand's row stays where it is.
        function test_with_the_stand_in_already_up_every_commit_is_taken_back() {
            list.contentY = 10 * Theme.graphRowHeight
            pin.headRow = 0
            hand.at = root.over(11)
            walk.noteHand()
            root.shifted = []
            root.writeIn(0, 1)
            walk.shiftRows(1)
            compare(list.contentY, 11 * Theme.graphRowHeight)
            compare(root.shifted, [])
        }
        /// …and stays up as commits come in above HEAD too: every one is taken back, the hand's row where it was in the
        /// band's fade — the stand-in was up before the reread, as the walk noted it (`noteHand`).
        function test_commits_above_a_head_already_out_of_view_are_all_taken_back() {
            list.contentY = 3 * Theme.graphRowHeight
            pin.headRow = 2
            hand.at = root.over(4)
            walk.noteHand()
            root.shifted = []
            root.writeIn(0, 1)
            pin.headRow = 3
            walk.shiftRows(1)
            compare(list.contentY, 4 * Theme.graphRowHeight)
            compare(root.shifted, [])
        }
        /// HEAD gone above the view rises the stand-in over a view down the rows: the hand's row comes down just clear
        /// of its band — the view gives back what the band covers, and goes back up for the rest.
        function test_head_going_above_the_view_brings_the_hands_row_down_clear_of_the_stand_in() {
            list.contentY = 10 * Theme.graphRowHeight
            pin.headRow = 12
            hand.at = root.over(11)
            walk.noteHand()
            root.shifted = []
            root.writeIn(0, 1)
            pin.headRow = 0
            walk.shiftRows(1)
            const top = 12 * Theme.graphRowHeight - list.contentY
            verify(top >= pin.topBand, "the hand's row stands clear of the stand-in: " + top)
            verify(top - Theme.graphRowHeight < pin.topBand, "and no further down than that: " + top)
            compare(root.shifted, [true], "the rows came down under the hand as the reread's")
        }
        /// No hand over the rows, no answer: the page weighs the reread by the newest commit.
        function test_no_hand_over_the_rows_is_no_answer() {
            hand.at = root.over(2)
            walk.noteHand()
            hand.at = Qt.point(-1, -1)
            root.writeIn(0, 1)
            compare(walk.handShift(), undefined)
        }
    }
}
