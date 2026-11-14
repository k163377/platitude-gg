import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui
import "chips.js" as Chips

// A right-click on a row of the card a chip unfolds into asks for the graph row's menu aimed at that row
// (デザイン規約 §グラフ行の右クリック「行のどこを押しても同じ 1 枚が出る」). Every row asks, the ones that lead
// nowhere included: the row of a worktree with no branch has its WORKTREE card and the detached HEAD's marker the
// commit's own rows; a branch another worktree holds has its `Open` row. Pressed with a real pointer, since the handler
// is what takes it.
Item {
    id: root
    width: 400
    height: 300

    property var asked: []

    RefListPopup {
        id: card
        chipRoom: 300
        onMenuAsked: chip => root.asked = root.asked.concat([chip.key])
    }

    TestCase {
        name: "ListRowMenu"
        when: windowShown

        function test_every_row_asks_for_the_menu() {
            card.records = [Chips.chip("branch", "main"), Chips.chip("worktree", "feature-worktree"),
                            Chips.chip("branch", "topic", { "held": true }), Chips.chip("head", "HEAD")]
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
