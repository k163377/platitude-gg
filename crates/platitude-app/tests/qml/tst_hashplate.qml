import QtQuick
import QtTest
import platitude.ui

// The one gesture the hash plate's two rows tell apart: a press that lets go where it landed is the control's, and a
// press that travels belongs to the words under it (デザイン規約 §右のペインの字は掴める).
// Only a real pointer can answer: the headless verbs enter the hand's own functions, and Qt hands the press to the
// `TextEdit` the digits are drawn in (`selectByMouse: false` and all), where it would never reach the button.
Item {
    id: root
    width: 400
    height: 200

    property int copies: 0
    property string copied: ""
    property int visits: 0
    property string visited: ""

    HashPlate {
        id: plate
        anchors.centerIn: parent
        sha8: "164db4f7"
        fullSha: "164db4f7aaaabbbbccccddddeeeeffff00001111"
        parentSha: "974a87e8aaaabbbbccccddddeeeeffff00002222"
        onCopyRequested: text => { root.copies++; root.copied = text }
        onParentClicked: oidHex => { root.visits++; root.visited = oidHex }
    }

    TestCase {
        name: "HashPlate"
        when: windowShown

        function init() {
            root.copies = 0
            root.copied = ""
            root.visits = 0
            root.visited = ""
            // A selection outlives the gesture that made it, so the board is cleared before the next one asks whether
            // it made any.
            for (let i = 0; i < plate.valueFields.length; i++)
                plate.valueFields[i].deselect()
        }

        /// A point of the row's value, in the root's coordinates.
        function pointIn(which, fx) {
            const f = plate.fieldFor(which)
            return f.mapToItem(root, f.width * fx, f.height / 2)
        }

        // First in the file on purpose: the answer stands until the pointer leaves the plate, and this platform
        // delivers no `hovered` to a `Control`, so nothing here can put the offer back once a press has taken it.
        // The open tip taking the new words is asked in the app (verify-ui, `hash-tip`).
        function test_a0_the_press_changes_what_the_control_offers() {
            const p = pointIn("hash", 0.5)
            compare(plate.tipWords(), "Copy full hash", "before the press it offers")
            mouseClick(root, p.x, p.y)
            compare(root.copies, 1, "the click still copied")
            compare(plate.tipWords(), "Copied!", "after the press it answers")
        }

        function test_a_click_on_the_digits_copies_the_whole_hash() {
            const p = pointIn("hash", 0.5)
            mouseClick(root, p.x, p.y)
            compare(root.copies, 1, "a click on the digits is the plate's")
            compare(root.copied, plate.fullSha)
        }

        // The mark and the digits are one target, so the press answers the same on both.
        function test_b_click_on_the_mark_copies_the_whole_hash() {
            const f = plate.fieldFor("hash")
            const p = f.mapToItem(root, f.width + Theme.spaceXs + Theme.iconMd / 2, f.height / 2)
            mouseClick(root, p.x, p.y)
            compare(root.copies, 1, "a click on the mark is the plate's")
            compare(root.copied, plate.fullSha)
        }

        function test_c_drag_over_the_digits_selects_them_and_copies_nothing() {
            const from = pointIn("hash", 0)
            const to = pointIn("hash", 1)
            mousePress(root, from.x, from.y)
            mouseMove(root, (from.x + to.x) / 2, from.y)
            mouseMove(root, to.x, to.y)
            mouseRelease(root, to.x, to.y)
            compare(plate.shaSelected, plate.sha8, "the drag put the shown hash in the field")
            compare(root.copies, 0, "a drag is not a click")
        }

        // Jitter inside the platform's drag distance is still a click, or a shaky hand would get a selection instead
        // of the copy.
        function test_d_a_press_that_barely_moves_is_still_a_click() {
            const p = pointIn("hash", 0.5)
            mousePress(root, p.x, p.y)
            mouseMove(root, p.x + 1, p.y + 1)
            mouseRelease(root, p.x + 1, p.y + 1)
            compare(root.copies, 1, "a press that did not travel is the plate's")
            compare(plate.shaSelected, "", "and it selected nothing")
        }

        function test_e_click_on_the_parent_goes_to_it() {
            const p = pointIn("parent", 0.5)
            mouseClick(root, p.x, p.y)
            compare(root.visits, 1, "a click on the parent hash is the link's")
            compare(root.visited, plate.parentSha)
        }

        function test_f_drag_over_the_parent_selects_it_and_goes_nowhere() {
            const from = pointIn("parent", 0)
            const to = pointIn("parent", 1)
            mousePress(root, from.x, from.y)
            mouseMove(root, (from.x + to.x) / 2, from.y)
            mouseMove(root, to.x, to.y)
            mouseRelease(root, to.x, to.y)
            compare(plate.parentSelected, plate.parentSha.substring(0, 8),
                    "the drag put the shown hash in the field")
            compare(root.visits, 0, "a drag is not a click")
        }

        // A press lets the last gesture's selection go whichever this one turns out to be, or the copy is taken under
        // words still washed by the drag before it.
        function test_g_a_click_after_a_drag_lets_the_selection_go() {
            const from = pointIn("hash", 0)
            const to = pointIn("hash", 1)
            mousePress(root, from.x, from.y)
            mouseMove(root, to.x, to.y)
            mouseRelease(root, to.x, to.y)
            compare(plate.shaSelected, plate.sha8)
            mouseClick(root, to.x, to.y)
            compare(root.copies, 1, "the click still copied")
            compare(plate.shaSelected, "", "and the drag's selection is gone")
        }

        // The plate is one thing: a press anywhere on it clears whichever row was holding a selection.
        function test_h_a_press_on_one_row_clears_the_other() {
            const from = pointIn("hash", 0)
            const to = pointIn("hash", 1)
            mousePress(root, from.x, from.y)
            mouseMove(root, to.x, to.y)
            mouseRelease(root, to.x, to.y)
            compare(plate.shaSelected, plate.sha8)
            const p = pointIn("parent", 0.5)
            mouseClick(root, p.x, p.y)
            compare(root.visits, 1, "the click still went to the parent")
            compare(plate.shaSelected, "", "and the hash let its selection go")
        }

        // One selection in the window: the second drag takes the first field's away with the caret.
        function test_j_a_second_drag_takes_the_first_selection_away() {
            const from = pointIn("hash", 0)
            const to = pointIn("hash", 1)
            mousePress(root, from.x, from.y)
            mouseMove(root, to.x, to.y)
            mouseRelease(root, to.x, to.y)
            compare(plate.shaSelected, plate.sha8)
            const pfrom = pointIn("parent", 0)
            const pto = pointIn("parent", 1)
            mousePress(root, pfrom.x, pfrom.y)
            mouseMove(root, pto.x, pto.y)
            mouseRelease(root, pto.x, pto.y)
            compare(plate.parentSelected, plate.parentSha.substring(0, 8))
            compare(plate.shaSelected, "", "the hash let go when the parent took the caret")
        }
    }
}
