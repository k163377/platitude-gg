import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// A double-click on a row of the card a chip unfolds into goes where that row leads (デザイン規約 §グラフ行のダブルクリック
// 「カードの行も、行の上と同じ所作で答える」): a branch another worktree holds goes to that worktree, as the graph row's
// double-click does; the current branch, a tag and the two markers (the detached HEAD, a worktree's folder) go
// nowhere. In at the row's own `doubleClick`, where a row takes or turns away the press: `mouseDoubleClickSequence`
// does not reach a `TapHandler` (`tst_tapmodifiers`).
Item {
    id: root
    width: 400
    height: 300

    property var picked: []

    /// A record of `kind`, every field spelled out as `encode::chips_of` hands it.
    function record(kind, name, held) {
        return { "kind": kind, "name": name, "isHead": false, "hasRemote": false, "hasPr": false, "here": true,
                 "held": held, "locked": false, "remote": "", "key": kind + ":" + name }
    }

    RefListPopup {
        id: card
        chipRoom: 300
        currentBranch: "main"
        onPicked: chip => root.picked = root.picked.concat([chip.key])
    }

    TestCase {
        name: "ListRowPick"
        when: windowShown

        function doubleClickRow(i) {
            card.listRoom = root.height
            card.layOutRows()
            card.open()
            tryCompare(card, "opened", true)
            verify(card.doubleClickRow(i))
            card.close()
            tryCompare(card, "opened", false)
        }

        function test_each_row_goes_where_it_leads() {
            card.records = [root.record("branch", "main", false), root.record("branch", "topic", true),
                            root.record("worktree", "topic-twin", false), root.record("branch", "side", false),
                            root.record("tag", "v1.0", false), root.record("head", "HEAD", false)]
            for (let i = 0; i < card.records.length; i++)
                doubleClickRow(i)
            compare(root.picked, ["branch:topic", "branch:side"])
        }
    }
}
