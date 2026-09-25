import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// Where a press on the lines an open row puts out goes (デザイン規約 §左メニューの所作): in the band of a line going
// somewhere of its own, the graph goes to that commit (`followFact`); anywhere else, the press is the row's.
// The real part under a real pointer (`qmltestrunner` delivers presses and hovers — verify-ui スキル); the row is a
// stub that writes down what it was handed.
Item {
    id: root
    width: 400
    height: 200

    property var followed: []
    property int presses: 0
    property int doubles: 0

    /// The four calls the part makes of its row.
    Item {
        id: stubRow
        function followFact(to) {
            root.followed = root.followed.concat([to.key])
        }
        function rowPressed(button, modifiers) {
            root.presses++
        }
        function rowDoubled(button) {
            root.doubles++
        }
        function factsMenuAsked() {
        }
    }

    NavRowFacts {
        id: facts
        width: 300
        row: stubRow
        // The table's shape (`NavFacts.lines`): a line naming a branch, the copy holding it — the same commit, so
        // the one band (`withAbove`) — and one naming nothing.
        lines: [{ "mark": "branch", "markTint": Theme.accent, "text": "main", "tone": Theme.textSecondary,
                  "ahead": 0, "behind": 0, "note": "", "to": { "key": "branch:main", "oid": "abc" },
                  "withAbove": false },
                { "mark": "tree", "markTint": Theme.success, "text": "a", "tone": Theme.textSecondary,
                  "ahead": 0, "behind": 0, "note": "", "to": { "key": "worktree:C:/a", "oid": "abc" },
                  "withAbove": true },
                { "mark": "", "markTint": Theme.warning, "text": "Folder is gone", "tone": Theme.warning,
                  "ahead": 0, "behind": 0, "note": "", "to": null, "withAbove": false }]
    }

    TestCase {
        name: "FactsFollow"
        when: windowShown

        function init() {
            root.followed = []
            root.presses = 0
            root.doubles = 0
            // The lines are laid out a pass after they are built; a point read earlier is the mark, not the words.
            tryVerify(() => facts.lineWordsMiddle(0).x > Theme.iconXs, undefined, "the words have been laid out")
        }

        function test_a_press_on_the_name_goes_where_it_names() {
            const at = facts.lineWordsMiddle(0)
            mouseClick(facts, at.x, at.y)
            compare(root.followed, ["branch:main"], "to where the line names")
            compare(root.presses, 0, "and not as the row's own click")
        }

        /// The copy holding the branch is where the branch is, so a hand on either lights both.
        function test_two_lines_going_to_one_place_are_one_band() {
            const copy = facts.lineWordsMiddle(1)
            mouseMove(facts, copy.x, copy.y)
            tryVerify(() => facts.lineAimed(1), undefined, "the copy's line is lit")
            verify(facts.lineAimed(0), "and the branch's line with it, in the one band")
            mouseClick(facts, copy.x, copy.y)
            compare(root.followed, ["branch:main"], "the press goes where the band goes")
            compare(root.presses, 0)
        }

        function test_the_whole_line_is_the_way_there() {
            const at = facts.lineWordsMiddle(0)
            mouseClick(facts, facts.width - 2, at.y)
            mouseClick(facts, facts.bandReach + 2, at.y)
            mouseClick(facts, 1, at.y)
            compare(root.followed, ["branch:main", "branch:main", "branch:main"],
                    "the far end, the mark and the edge of the band all go there")
            compare(root.presses, 0, "and the row under the lines hears none of it")
        }

        /// The band stops half a gap under the last line.
        function test_the_foot_under_the_last_band_is_the_rows_own() {
            mouseClick(facts, facts.width / 2, facts.height - 1)
            compare(root.followed, [])
            compare(root.presses, 1)
        }

        /// A line naming nothing (a state, a reading git cannot reach) is the row's all the way across.
        function test_a_press_on_a_line_going_nowhere_is_the_rows_own() {
            const at = facts.lineWordsMiddle(2)
            mouseClick(facts, at.x, at.y)
            compare(root.followed, [])
            compare(root.presses, 1)
        }

        /// The first press already followed; handed to the row, the double would switch to a name the hand was not on.
        function test_a_double_click_on_the_line_is_not_the_rows_double_click() {
            const at = facts.lineWordsMiddle(0)
            mouseDoubleClickSequence(facts, at.x, at.y)
            compare(root.followed, ["branch:main"], "followed once — Qt withholds the second click of a double")
            compare(root.doubles, 0, "and the row's double-click never came")
            const nowhere = facts.lineWordsMiddle(2)
            mouseDoubleClickSequence(facts, nowhere.x, nowhere.y)
            compare(root.doubles, 1, "while one on a line going nowhere is the row's, as it always was")
        }

        function test_a_drag_over_the_name_takes_the_words_and_goes_nowhere() {
            const at = facts.lineWordsMiddle(0)
            mousePress(facts, 2, at.y)
            mouseMove(facts, at.x, at.y)
            mouseMove(facts, facts.width - 2, at.y)
            mouseRelease(facts, facts.width - 2, at.y)
            verify(facts.sweptText() !== "", "the words came away")
            compare(root.followed, [])
            compare(root.presses, 0)
        }

        function test_the_hand_on_the_line_is_answered_and_let_go() {
            const at = facts.lineWordsMiddle(0)
            mouseMove(facts, at.x, at.y)
            tryVerify(() => facts.lineAimed(0), undefined, "the band is laid under the line, over the words")
            verify(facts.handShown, "and the pointer is the hand")
            mouseMove(facts, facts.width - 2, at.y)
            verify(facts.lineAimed(0), "at the far end")
            mouseMove(facts, 1, at.y)
            verify(facts.lineAimed(0), "and in the gap outside the lines the band reaches into")
            // The move is delivered in the call, and the handlers answer it there.
            const nowhere = facts.lineWordsMiddle(2)
            mouseMove(facts, nowhere.x, nowhere.y)
            verify(!facts.lineAimed(0), "off the line it goes")
            verify(!facts.lineAimed(2), "and a line going nowhere never wears it")
            verify(!facts.handShown, "the pointer is the text one again")
        }
    }
}
