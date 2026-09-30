import QtQuick
import QtTest
import platitude.ui

// Who gives way on the details pane's date line when the row is short (デザイン規約 §co-author の表示): the credit's
// name is cut, then folded to the face and its count, and only then is the date cut from its tail — the minutes
// first. Whatever gives, the line's words stay clear of the parent line beside them, whose two parents always stand.
// The row is set here by width alone; the pane's floor and the fonts are the app's (`details-align`).
Item {
    id: root
    width: 800
    height: 100

    QtObject {
        id: commit
        property string authorName: "Alexandria Montgomery-Vanderbilt"
        property string authorEmail: "demo@example.com"
        property int authorTime: 1790751420
        property int avatar: 3
        property string avatarUrl: ""
        property string sha8: "3c1b3390"
        property string shaHex: "3c1b3390aaaabbbbccccddddeeeeffff00001111"
        property var coAuthors: []
        property var parentHexes: []
        property string parentHex: parentHexes.length > 0 ? parentHexes[0] : ""
    }

    CommitAuthorRow {
        id: row
        details: commit
        paneWidth: row.width
    }

    TestCase {
        name: "AuthorRowRoom"
        when: windowShown

        function init() {
            const mates = []
            for (let i = 1; i <= 12; i++)
                mates.push({ name: "Pair Programmer " + i, email: "pair" + i + "@example.com", face: i })
            commit.coAuthors = mates
            const parents = []
            for (let i = 0; i < 13; i++)
                parents.push(i + "a1b2c3d4aaaabbbbccccddddeeeeffff000044" + (10 + i))
            commit.parentHexes = parents
        }

        /// The row handed `width` and laid out to it: the date and the credit stand at the rooms they were given. Both,
        /// since either can hold still across a step while the other moves.
        function settleAt(width) {
            row.width = width
            tryVerify(() => Math.abs(row.dateWidth - row.dateRoom) < 0.5
                            && Math.abs(row.creditWidth - row.creditRoom) < 0.5)
        }
        function apart() {
            return row.baselines().apart
        }
        /// Narrows the row step by step until `reached` holds, checking at every step that nothing met.
        function narrowUntil(reached) {
            for (let width = root.width; width > 100; width -= 4) {
                settleAt(width)
                verify(apart() >= Theme.spaceXs, "the line's words ran into the arrow at " + width + ": " + apart())
                if (reached())
                    return width
            }
            return -1
        }

        // Room enough: the credit named, the date whole.
        function test_a_a_wide_row_cuts_nothing() {
            settleAt(root.width)
            verify(!row.matesFolded)
            verify(!row.dateClipped)
            verify(apart() >= Theme.spaceXs)
        }

        // Shorter, the credit's name folds away before the date loses a character; the count stays.
        function test_b_the_credit_folds_before_the_date_is_cut() {
            verify(narrowUntil(() => row.matesFolded) > 0, "the credit folded somewhere on the way down")
            verify(!row.dateClipped, "and the date was still whole when it did")
            compare(row.matesSaid, "+11", "the count is the same folded")
        }

        // Shorter still, the date is cut from its tail, the credit folded, two parents standing.
        function test_c_past_the_fold_the_date_is_cut_and_nothing_meets() {
            verify(narrowUntil(() => row.dateClipped) > 0, "the date gave way at last")
            verify(row.matesFolded)
            compare(row.plate.parentsShown, 2, "two parents stand")
        }
    }
}
