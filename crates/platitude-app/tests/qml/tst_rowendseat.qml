import QtQuick
import QtTest
import platitude.ui
import "painted.js" as Painted

// The left menu's row-end column (デザイン規約 §アイコンの席 の ②「行末の列」): the remote badge after the name takes the
// seat of the name's letters, one width down the rows, as a badge after a name does anywhere.
Item {
    id: root
    width: 300
    height: 100

    HandWatch { id: hand }

    NavItemDelegate {
        id: row
        index: 0
        name: "topic"
        full: "topic"
        oid_hex: ""
        change: ""
        bucket: ""
        orig_path: ""
        orig_name: ""
        is_head: false
        has_remote: true
        only_remote: false
        has_pr: false
        depth: 0
        folder: false
        eol_mark: false
        ahead: 0
        behind: 0
        listWidth: root.width
        height: Theme.rowHeight
        paneHand: hand
        kindHint: "branch"
    }

    TestCase {
        name: "RowEndSeat"
        when: windowShown

        function test_the_badge_takes_the_seat_of_the_name_s_letters() {
            const badge = Painted.itemUnder(row, item => item.pullRequest !== undefined && item.gone !== undefined)
            verify(badge !== null, "the row wears its remote badge")
            const name = Painted.itemUnder(row, item => item.nameInk !== undefined)
            const seat = MarkSeat.cell(name.nameInk.pixelSize)
            tryCompare(badge.parent, "width", seat, undefined, "the column the badge stands in")
            compare(badge.implicitWidth, seat, "the badge's own")
        }
    }
}
