import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// A heading's names past the name ceiling (`CardText.holdsNames`, デザイン規約 §寸法「名前の天井」): cut in the middle,
// both ends kept, and the whole line put up by a hand on the cut name — not by one on the words round it. A name under
// the ceiling, and a line that holds nothing, are drawn whole.
Item {
    id: root
    width: 1400
    height: 400
    // What `Main.qml` and the popup bases declare (rules-refs/app-ui.md「ToolTip.policy」).
    ToolTip.policy: ToolTip.Manual

    readonly property string longName:
        "bugfix/keep-the-long-branch-names-readable-in-every-menu-card-tooltip-question-bar-notice-and-the-button"
    function sentence(name) {
        return "Deleted " + name + " here"
    }

    PointerWatch { id: hand }
    SharedToolTip {
        id: shared
        host: root
        hand: hand
    }

    CardText {
        id: held
        width: 1300
        text: root.sentence(root.longName)
        marks: Words.marksOf(root.sentence, [[root.longName, "branch"]])
        holdsNames: true
    }
    CardText {
        id: brief
        y: 60
        width: 1300
        text: root.sentence("main")
        marks: Words.marksOf(root.sentence, [["main", "branch"]])
        holdsNames: true
    }
    CardText {
        id: whole
        y: 120
        width: 1300
        text: root.sentence(root.longName)
        marks: Words.marksOf(root.sentence, [[root.longName, "branch"]])
    }

    TestCase {
        name: "HeadingNames"
        when: windowShown

        /// The middle of the drawn name's `n`th character, in the root's coordinates.
        function nameCharAt(card, n) {
            const r = card.charRect(card.markRoomAt(0) + card.markRooms[0] + n)
            return card.mapToItem(root, r.x + r.width / 2, r.y + r.height / 2)
        }

        function test_a_name_past_the_ceiling_is_cut_in_the_middle() {
            tryVerify(() => held.markRooms.length > 0 && held.markRooms[0] > 0, undefined, "the mark's room is open")
            compare(held.held.cuts.length, 1)
            const drawn = held.held.text.substr(held.held.marks[0][0], held.held.marks[0][3])
            verify(drawn.indexOf("…") > 0, "cut in the middle: " + drawn)
            verify(root.longName.startsWith(drawn.substring(0, drawn.indexOf("…"))), "the head kept: " + drawn)
            verify(root.longName.endsWith(drawn.substring(drawn.indexOf("…") + 1)), "the tail kept: " + drawn)
            verify(held.held.text.startsWith("Deleted ") && held.held.text.endsWith(" here"), "the words kept")
            compare(held.tipMarks, held.marks, "the tip seats the whole names")
        }

        function test_a_name_under_the_ceiling_and_a_line_that_holds_none_are_whole() {
            compare(brief.held.cuts.length, 0)
            compare(brief.held.text, root.sentence("main"))
            compare(brief.tipMarks.length, 0)
            compare(whole.held.cuts.length, 0)
            compare(whole.held.text, root.sentence(root.longName))
        }

        function test_a_hand_on_the_cut_name_puts_the_whole_line_up() {
            tryVerify(() => held.markRooms.length > 0 && held.markRooms[0] > 0, undefined, "the mark's room is open")
            const on = nameCharAt(held, 3)
            mouseMove(root, on.x, on.y)
            tryVerify(() => held.cutPointed, undefined, "the hand is on the cut name")
            tryVerify(() => shared.sharedTip.visible && shared.sharedTip.parent === held, undefined, "the tip is up")
            compare(shared.sharedTip.text, root.sentence(root.longName), "the whole line")

            const off = held.mapToItem(root, held.charRect(2).x, held.charRect(2).y + held.charRect(2).height / 2)
            mouseMove(root, off.x, off.y)
            tryVerify(() => !held.cutPointed, undefined, "the words round the name put nothing up")
            mouseMove(root, root.width - 1, root.height - 1)
        }
    }
}
