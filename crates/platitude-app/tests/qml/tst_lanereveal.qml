import QtQuick
import QtTest
import platitude.ui

// A commit clicked, stepped onto or jumped to whose node lies past the lane column's edge dashes the lanes sideways
// to it (`GraphColumnMetrics.revealLane`, the lanes' `ScrollSend.dashTo`), and a hand on the lanes stops a dash in
// flight (`panTo`). The lane strip brings the node on only at a release that did not travel
// (`GraphLanePan.releaseLanded`): at the press, a pan's grab would see the lanes slide out from under the hand.
//
// The strip is the real one with real presses; the list under it is a stand-in answering the three things the strip
// asks of a view (`indexAt`, `itemAtIndex`, `contentY`), so a row is a lane and a count of the presses it took.
Item {
    id: root
    width: 600
    height: 300

    readonly property int rowH: 30

    /// Six lanes in a column two lanes wide: lanes 0 and 1 shown unpanned, 5 the farthest past the right edge.
    readonly property GraphColumnMetrics columns: GraphColumnMetrics {
        id: columns
        paneW: root.width
        maxLanes: 6
        graphColWManual: Metrics.laneInset + 2 * Metrics.laneW
    }
    /// The same lanes squeezed to the column's floor, too narrow for the gaps either side of a revealed node.
    readonly property GraphColumnMetrics floor: GraphColumnMetrics {
        id: floor
        paneW: root.width
        maxLanes: 6
        graphColWManual: 0
    }

    component StubRow: QtObject {
        property int node_lane: 0
        property bool naming: false
        property int picks: 0
        function pickAtPress(modifiers) {
            picks++
        }
        function doubleClick(modifiers) {
        }
    }
    // Property values, not bare children of this Item: the incremental GC takes a bare `QtObject` child
    // (rules-refs/app-ui.md, the line on `QtObject` parts inside an Item).
    readonly property StubRow farRow: StubRow {
        node_lane: 5
    }
    readonly property StubRow nearRow: StubRow {
        node_lane: 0
    }

    Item {
        id: view
        anchors.fill: parent
        property real contentY: 0
        property var footerItem: null
        readonly property var rows: [root.farRow, root.nearRow]
        function indexAt(x, y) {
            const i = Math.floor(y / root.rowH)
            return i >= 0 && i < view.rows.length ? i : -1
        }
        function itemAtIndex(i) {
            return i >= 0 && i < view.rows.length ? view.rows[i] : null
        }
    }
    GraphLanePan {
        id: strip
        columns: columns
        view: view
    }

    /// The arrows' walk, the real one (`GraphRowWalk`), over a list cut down to what it reads: rows whose nodes stand
    /// on lane 0 and lane 5 by turns, in a column of their own.
    readonly property GraphColumnMetrics walkColumns: GraphColumnMetrics {
        id: walkColumns
        paneW: root.width
        maxLanes: 6
        graphColWManual: Metrics.laneInset + 2 * Metrics.laneW
    }
    readonly property var walkLanes: [0, 5, 0, 5, 0, 5, 0, 5]
    ListView {
        id: walkList
        width: 100
        height: Theme.graphRowHeight * 8
        model: root.walkLanes.length
        delegate: Item {
            width: walkList.width
            height: Theme.graphRowHeight
        }
        readonly property var columns: walkColumns
        // The walk listens to the hand over the rows (`GraphRowWalk.handWatch`); none comes here.
        readonly property HandWatch hand: HandWatch {}
        property string namingOid: ""
        readonly property real landingY: walkList.contentY
        function haltSend() {}
        function landSend() {}
        // The rows' dash is `GraphList`'s; here only the lanes are watched, so the rows land at once.
        function dashToRow(row) {
            walkList.positionViewAtIndex(row, ListView.Center)
        }
        function clampY(y) {
            return y
        }
        function rowOnScreen(row) {
            return true
        }
    }
    GraphRowWalk {
        id: walk
        view: walkList
        graphModel: QtObject {
            property bool provisionalOn: false
            function oidAt(row) {
                return row >= 0 && row < root.walkLanes.length ? "c" + row : ""
            }
            function laneAt(row) {
                return row >= 0 && row < root.walkLanes.length ? root.walkLanes[row] : -1
            }
        }
        asking: false
    }
    /// Takes the keyboard off the walk's list.
    Item {
        id: walkElsewhere
        y: walkList.height
        width: 10
        height: 10
    }

    /// Whether the node on `lane` lies whole in the column as `metrics` stands — what the lane cell draws
    /// (`GraphLaneCell.nodeMidX`).
    function nodeWhole(metrics, lane) {
        const mid = Metrics.laneInset + lane * Metrics.laneW + Metrics.laneW / 2 - metrics.graphX
        return mid - Metrics.nodeIcon / 2 >= 0 && mid + Metrics.nodeIcon / 2 <= metrics.graphColW
    }

    TestCase {
        name: "RevealLane"
        when: windowShown

        function init() {
            columns.panTo(0)
            floor.panTo(0)
        }

        /// A reveal that moves the lanes dashes them, and lands on `x`.
        function dashesTo(metrics, x, what) {
            verify(metrics.lanesDashing, what + ": the lanes dash rather than jump")
            tryCompare(metrics, "graphX", x, undefined, what)
            tryVerify(() => !metrics.lanesDashing, undefined, what + ": and the dash ends there")
        }

        function test_a_node_already_whole_leaves_the_lanes_where_they_are() {
            columns.revealLane(1)
            verify(!columns.lanesDashing, "lane 1 is whole unpanned")
            compare(columns.graphX, 0)
            columns.panTo(columns.graphXMax)
            columns.revealLane(5)
            verify(!columns.lanesDashing, "lane 5 is whole panned to the end")
            compare(columns.graphX, columns.graphXMax)
        }

        function test_a_node_past_the_right_edge_comes_to_where_the_last_lane_ends() {
            columns.revealLane(5)
            dashesTo(columns, columns.graphXMax, "the last lane lands where panning to the end puts it")
            columns.panTo(0)
            columns.revealLane(2)
            const atEnd = Metrics.laneInset + 3 * Metrics.laneW + Theme.spaceSm - columns.graphColW
            dashesTo(columns, atEnd, "lane 2 lands with the gap the last lane keeps at the far end")
            verify(root.nodeWhole(columns, 2), "whole")
        }

        function test_a_node_past_the_left_edge_comes_to_where_lane_0_stands_unpanned() {
            columns.panTo(columns.graphXMax)
            columns.revealLane(0)
            dashesTo(columns, 0, "lane 0 goes back to rest")
            columns.panTo(columns.graphXMax)
            columns.revealLane(2)
            dashesTo(columns, 2 * Metrics.laneW, "lane 2 stands where lane 0 does at rest")
            verify(root.nodeWhole(columns, 2), "whole")
        }

        function test_a_node_cut_by_an_edge_counts_as_not_shown() {
            columns.panTo(Metrics.laneW / 2)
            verify(!root.nodeWhole(columns, 0), "the setup cuts lane 0 at the left edge")
            columns.revealLane(0)
            dashesTo(columns, 0, "lane 0 comes back whole")
        }

        function test_a_column_too_narrow_for_both_gaps_takes_the_left_place_and_stays() {
            verify(floor.graphColW < Metrics.laneInset + Metrics.laneW + Theme.spaceSm,
                   "the floor holds neither gap beside a node")
            floor.panTo(floor.graphXMax)
            floor.revealLane(3)
            dashesTo(floor, 3 * Metrics.laneW, "lane 3 stands where lane 0 does at rest")
            floor.revealLane(3)
            verify(!floor.lanesDashing, "a second click on the same row moves nothing")
            compare(floor.graphX, 3 * Metrics.laneW)
        }

        function test_a_hand_on_the_lanes_stops_a_dash_in_flight() {
            columns.revealLane(5)
            verify(columns.lanesDashing)
            columns.panTo(10)
            verify(!columns.lanesDashing, "the dash stops for the hand")
            // waits(paced): the dash's length twice over; a dash left running would have dragged the lanes on.
            wait(columns.laneSend.dashMs * 2)
            compare(columns.graphX, 10, "and the lanes stay where the hand put them")
        }
    }

    TestCase {
        name: "LaneStripReveal"
        when: windowShown

        readonly property real farY: root.rowH / 2
        readonly property real nearY: root.rowH + root.rowH / 2

        function init() {
            columns.panTo(0)
            root.farRow.picks = 0
            root.nearRow.picks = 0
        }

        function test_a_click_picks_at_the_press_and_brings_the_node_on_at_the_release() {
            mousePress(strip, 20, farY, Qt.LeftButton)
            compare(root.farRow.picks, 1, "the row is picked at the press")
            verify(!columns.lanesDashing, "the lanes stay put while the button is down")
            compare(columns.graphX, 0)
            mouseRelease(strip, 20, farY, Qt.LeftButton)
            verify(columns.lanesDashing, "the release sets lane 5 on its way")
            tryCompare(columns, "graphX", columns.graphXMax)
        }

        function test_a_drag_pans_from_where_the_hand_grabbed_and_brings_nothing_on() {
            mousePress(strip, 30, farY, Qt.LeftButton)
            for (let i = 1; i <= 4; ++i)
                mouseMove(strip, 30 - i * 5, farY)
            compare(columns.graphX, 20, "the lanes follow the hand from where they were")
            mouseRelease(strip, 10, farY, Qt.LeftButton)
            verify(!columns.lanesDashing, "a pan's release sets nothing on its way")
            compare(columns.graphX, 20, "and leaves the lanes where the hand let go")
            compare(root.farRow.picks, 1, "the press still picked the row")
        }

        function test_a_press_takes_the_lanes_from_a_dash_in_flight() {
            columns.revealLane(5)
            verify(columns.lanesDashing)
            mousePress(strip, 20, nearY, Qt.LeftButton)
            verify(!columns.lanesDashing, "the hand on the lanes stops the dash")
            const held = columns.graphX
            // waits(paced): the dash's length twice over; a dash left running would have moved the lanes on.
            wait(columns.laneSend.dashMs * 2)
            compare(columns.graphX, held, "and the lanes stand still under the hand")
            mouseRelease(strip, 20, nearY, Qt.LeftButton)
        }

        function test_the_release_brings_on_the_row_pressed_not_the_one_under_the_hand() {
            columns.panTo(columns.graphXMax)
            mousePress(strip, 20, nearY, Qt.LeftButton)
            mouseMove(strip, 20, farY)
            mouseRelease(strip, 20, farY, Qt.LeftButton)
            compare(root.nearRow.picks, 1)
            compare(root.farRow.picks, 0)
            tryCompare(columns, "graphX", 0, undefined, "lane 0, the pressed row's")
        }

        function test_the_automation_click_is_the_same_click() {
            verify(strip.clickAt(Qt.point(20, farY)), "the click lands on a row")
            compare(root.farRow.picks, 1)
            tryCompare(columns, "graphX", columns.graphXMax)
        }
    }

    // The arrows bring the node on where they read the row (`GraphRowWalk.landStep`): a press at once, a held run once
    // where it stops — the highlight runs, the lanes do not swing under it (デザイン規約 §矢印で履歴を辿る). A jump
    // brings it on where it lands (`jumpToRow`, `revealRow`).
    TestCase {
        name: "WalkReveal"
        when: windowShown

        function init() {
            tryVerify(() => !walk.settling, undefined, "no settle left over from the last case")
            walkList.currentIndex = 0
            walkColumns.panTo(0)
        }

        function test_a_step_brings_on_the_node_of_the_row_it_lands_on() {
            verify(walk.stepRow(1, false))
            verify(walkColumns.lanesDashing, "lane 5 is on its way at once")
            tryCompare(walkColumns, "graphX", walkColumns.graphXMax)
        }

        function test_a_held_run_moves_the_lanes_once_where_it_stops() {
            verify(walk.stepRow(1, false))
            tryCompare(walkColumns, "graphX", walkColumns.graphXMax, undefined, "the press brings lane 5 on")
            tryVerify(() => !walkColumns.lanesDashing)
            verify(walk.stepRow(1, true))
            verify(!walkColumns.lanesDashing, "a held step onto lane 0 leaves the lanes")
            verify(walk.stepRow(1, true))
            verify(walk.stepRow(1, true))
            verify(!walkColumns.lanesDashing, "and so does the rest of the run")
            compare(walkColumns.graphX, walkColumns.graphXMax)
            walk.keyUp()
            tryCompare(walkColumns, "graphX", 0, undefined, "where the run stops, its row's lane 0 comes on")
        }

        function test_the_keyboard_leaving_the_list_ends_a_held_run_where_it_stopped() {
            walkList.forceActiveFocus()
            verify(walk.stepRow(1, false))
            tryCompare(walkColumns, "graphX", walkColumns.graphXMax, undefined, "the press brings lane 5 on")
            tryVerify(() => !walkColumns.lanesDashing)
            verify(walk.stepRow(1, true))
            walkElsewhere.forceActiveFocus()
            verify(!walk.keyHeld, "a key that comes up somewhere else still ends the run")
            tryCompare(walkColumns, "graphX", 0, undefined, "and the run's row's lane 0 comes on")
        }

        function test_a_jump_brings_on_the_node_of_the_row_it_lands_on() {
            walk.jumpToRow(3)
            compare(walkList.currentIndex, 3)
            tryCompare(walkColumns, "graphX", walkColumns.graphXMax, undefined, "lane 5 comes on")
        }

        function test_a_row_brought_into_the_open_brings_its_node_on_though_the_view_stays() {
            walkColumns.panTo(walkColumns.graphXMax)
            const wasY = walkList.contentY
            walk.revealRow(2)
            compare(walkList.contentY, wasY, "the row stood in the open already")
            compare(walkList.currentIndex, 0, "and the selection is not the reveal's")
            tryCompare(walkColumns, "graphX", 0, undefined, "its lane 0 comes on")
        }
    }
}
