import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// Where a press on the lines an open row puts out goes (デザイン規約 §左メニューの所作): in the band of a line going
// somewhere of its own, the graph goes to that commit (the row is handed where — `followFact`); anywhere else, the
// press is the row's the lines are under.
//
// **The real part, and a real pointer**: `NavRowFacts` takes plain values and a row to hand its presses to, so the
// row here only writes down what it was handed, and `qmltestrunner` is the one place a press and a hover are
// delivered for real (verify-ui スキル). What decides where a press goes is the part's own: which band it landed
// in, and whether it moved.
Item {
    id: root
    width: 400
    height: 200

    /// What the row was handed: the keys a press on a line went to, and the presses and double-clicks that were the
    /// row's own.
    property var followed: []
    property int presses: 0
    property int doubles: 0

    /// The row the lines belong to — the four calls the part makes of one.
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
            // The lines are laid out a pass after they are built: a point read before that is the line's origin,
            // which is the mark and not the words.
            tryVerify(() => facts.lineWordsMiddle(0).x > Theme.iconXs, undefined, "the words have been laid out")
        }

        /// The words of a line going somewhere: the press goes there, and the row the lines are under hears nothing.
        function test_a_press_on_the_name_goes_where_it_names() {
            const at = facts.lineWordsMiddle(0)
            mouseClick(facts, at.x, at.y)
            compare(root.followed, ["branch:main"], "to where the line names")
            compare(root.presses, 0, "and not as the row's own click")
        }

        /// **Two lines going to one commit are one band**: the copy holding the branch is where the branch is, so a
        /// hand on either lights both and a press on either goes to the one place.
        function test_two_lines_going_to_one_place_are_one_band() {
            const copy = facts.lineWordsMiddle(1)
            mouseMove(facts, copy.x, copy.y)
            tryVerify(() => facts.lineAimed(1), undefined, "the copy's line is lit")
            verify(facts.lineAimed(0), "and the branch's line with it, in the one band")
            mouseClick(facts, copy.x, copy.y)
            compare(root.followed, ["branch:main"], "the press goes where the band goes")
            compare(root.presses, 0)
        }

        /// **The whole line is the target** — the mark, the room beside a short name and the far end where the
        /// measure stands — and the gap's worth out to either side the band reaches.
        function test_the_whole_line_is_the_way_there() {
            const at = facts.lineWordsMiddle(0)
            mouseClick(facts, facts.width - 2, at.y)
            mouseClick(facts, facts.bandReach + 2, at.y)
            mouseClick(facts, 1, at.y)
            compare(root.followed, ["branch:main", "branch:main", "branch:main"],
                    "the far end, the mark and the edge of the band all go there")
            compare(root.presses, 0, "and the row under the lines hears none of it")
        }

        /// Past the last line's band is the row's own again: the band stops half a gap under the line.
        function test_the_foot_under_the_last_band_is_the_rows_own() {
            mouseClick(facts, facts.width / 2, facts.height - 1)
            compare(root.followed, [])
            compare(root.presses, 1)
        }

        /// A line naming nothing — a state, a reading git cannot reach — is the row's to click all the way across.
        function test_a_press_on_a_line_going_nowhere_is_the_rows_own() {
            const at = facts.lineWordsMiddle(2)
            mouseClick(facts, at.x, at.y)
            compare(root.followed, [])
            compare(root.presses, 1)
        }

        /// **A double-click on the line is two presses of it, and the first already went.** Handed to the row it
        /// would be the row's double-click — a switch to a name the hand was not on.
        function test_a_double_click_on_the_line_is_not_the_rows_double_click() {
            const at = facts.lineWordsMiddle(0)
            mouseDoubleClickSequence(facts, at.x, at.y)
            compare(root.followed, ["branch:main"], "followed once — Qt withholds the second click of a double")
            compare(root.doubles, 0, "and the row's double-click never came")
            const nowhere = facts.lineWordsMiddle(2)
            mouseDoubleClickSequence(facts, nowhere.x, nowhere.y)
            compare(root.doubles, 1, "while one on a line going nowhere is the row's, as it always was")
        }

        /// A drag over the words takes them away — the reader is copying, and nothing is followed.
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

        /// The hand anywhere on the line is answered by the band laid under it and the pointer the area turns, and
        /// walking off onto a line going nowhere takes both back.
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
