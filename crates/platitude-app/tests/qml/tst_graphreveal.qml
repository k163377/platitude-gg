import QtQuick
import QtTest
import platitude.ui

// The graph bringing a held commit's row into the open for the choice's list (`GraphRowWalk.revealRow`,
// デザイン規約 §複数のコミットを選ぶ): a row standing whole in the view and clear of the current branch's stand-in stays
// where it is; one cut by an edge, under the stand-in's band or out of sight is centred; and the row the arrows step
// from stays put. The real walk, over a list cut down to what it reads.
Item {
    id: root
    width: 400
    height: Theme.graphRowHeight * 8

    ListModel {
        id: rows
    }

    // `GraphList` cut down to what the walk reads (`GraphPane`).
    ListView {
        id: list
        width: root.width
        height: Theme.graphRowHeight * 8
        model: rows
        delegate: Item {
            width: list.width
            height: Theme.graphRowHeight
        }
        readonly property HandWatch hand: HandWatch {}
        function haltGlide() {}
        function clampY(y) {
            return y
        }
        // Says every row is on screen: what the reveal decides is its own to weigh, not this answer's.
        function rowOnScreen(row) {
            return true
        }
    }
    // The current branch's stand-in cut down to what the walk asks of it (`GraphHeadPin.topBand` / `aboveAt`).
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
                return -1
            }
        }
        asking: false
    }

    TestCase {
        name: "GraphReveal"
        when: windowShown

        function init() {
            rows.clear()
            for (let i = 0; i < 40; i++)
                rows.append({ "oid_hex": "c" + i })
            list.contentY = 0
            list.forceLayout()
            list.currentIndex = 5
            pin.headRow = -1
        }

        /// Whether `row`'s middle stands within a pixel of the view's.
        function centred(row) {
            const middle = list.originY + (row + 0.5) * Theme.graphRowHeight
            return Math.abs(middle - (list.contentY + list.height / 2)) < 1
        }

        function test_a_row_whole_in_the_view_stays_where_it_is() {
            walk.revealRow(3)
            compare(list.contentY, 0)
            compare(list.currentIndex, 5, "the row the arrows step from stays")
        }

        function test_a_row_out_of_sight_is_centred() {
            walk.revealRow(30)
            verify(centred(30), "contentY " + list.contentY)
            compare(list.currentIndex, 5, "the row the arrows step from stays")
        }

        function test_a_row_cut_by_the_top_edge_is_centred() {
            list.contentY = Theme.graphRowHeight * 10.5
            walk.revealRow(10)
            verify(centred(10), "contentY " + list.contentY)
        }

        function test_a_row_cut_by_the_bottom_edge_is_centred() {
            list.contentY = Theme.graphRowHeight * 0.5
            walk.revealRow(8)
            verify(centred(8), "contentY " + list.contentY)
        }

        function test_a_row_under_the_stand_in_is_centred() {
            pin.headRow = 0
            list.contentY = Theme.graphRowHeight * 10
            walk.revealRow(11)
            verify(centred(11), "contentY " + list.contentY)
        }

        function test_a_row_clear_of_the_stand_in_stays_where_it_is() {
            pin.headRow = 0
            list.contentY = Theme.graphRowHeight * 10
            walk.revealRow(15)
            compare(list.contentY, Theme.graphRowHeight * 10)
        }

        function test_no_row_moves_nothing() {
            list.contentY = Theme.graphRowHeight * 10
            walk.revealRow(-1)
            compare(list.contentY, Theme.graphRowHeight * 10)
        }
    }
}
