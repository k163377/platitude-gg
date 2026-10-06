import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// A right-click on a row of the card a chip unfolds into asks for the graph row's menu aimed at that row
// (デザイン規約 §グラフ行の右クリック「行のどこを押しても同じ 1 枚が出る」). Every row asks, the ones that lead
// nowhere included: a worktree's folder has its WORKTREE card, a branch another worktree holds its `Open` row, and the
// detached HEAD's marker the commit's own rows. Pressed with a real pointer, since what refused it was the handler.
Item {
    id: root
    width: 400
    height: 300

    property var asked: []

    /// A record of `kind`, every field spelled out as `encode::chips_of` hands it.
    function record(kind, name, held) {
        return { "kind": kind, "name": name, "isHead": false, "hasRemote": false, "hasPr": false, "here": true,
                 "held": held, "locked": false, "remote": "", "key": kind + ":" + name }
    }

    RefListPopup {
        id: card
        chipRoom: 300
        onMenuAsked: chip => root.asked = root.asked.concat([chip.key])
    }

    TestCase {
        name: "ListRowMenu"
        when: windowShown

        function test_every_row_asks_for_the_menu() {
            card.records = [root.record("branch", "main", false), root.record("worktree", "feature-worktree", false),
                            root.record("branch", "topic", true), root.record("head", "HEAD", false)]
            card.listRoom = root.height
            card.layOutRows()
            card.open()
            tryCompare(card, "opened", true)
            for (let i = 0; i < card.records.length; i++) {
                const row = card.rowAt(i)
                verify(row !== null)
                mouseClick(row, row.width / 2, row.height / 2, Qt.RightButton)
            }
            compare(root.asked, ["branch:main", "worktree:feature-worktree", "branch:topic", "head:HEAD"])
            card.close()
        }
    }
}
