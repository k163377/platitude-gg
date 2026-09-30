import QtQuick
import QtTest
import platitude.ui

// The credit line on the details pane's date row when the row cannot give the first name even three characters and
// the mark (規約 §co-author の表示): the name goes, the face stands for the first and the count stays as it was —
// "◯ +2". Folding must not change what the line asks of its holder, or the width it is handed and the fold chase each
// other.
Item {
    id: root
    width: 600
    height: 100

    readonly property var three: [
        { name: "Claude Opus 5", email: "noreply@anthropic.com", face: 1 },
        { name: "Claude Fable 5", email: "noreply@anthropic.com", face: 2 },
        { name: "Claude Opus 4.8", email: "noreply@anthropic.com", face: 3 }
    ]

    CoAuthorLine {
        id: line
        records: root.three
    }

    TestCase {
        name: "MateFold"
        when: windowShown

        function init() {
            line.records = root.three
            line.width = line.naturalWidth
        }

        /// The width at which the name has exactly its floor beside the face and the count.
        function floorWidth() {
            return Theme.iconMd + Theme.spaceXs + line.countSeat + line.nameFloor
        }

        function test_a_room_enough_names_the_first() {
            compare(line.folded, false)
            verify(line.nameStands)
            compare(line.countSaid, "+2", "everybody after the first")
        }

        // Down to the floor the name is cut, not folded: three characters and the mark still name somebody.
        function test_b_at_its_floor_the_name_is_cut() {
            line.width = floorWidth()
            compare(line.folded, false)
            verify(line.nameStands)
        }

        // The face is the first: the count says the same with or without the name.
        function test_c_short_of_its_floor_the_name_goes_and_the_count_stays() {
            const said = line.countSaid
            line.width = floorWidth() - 1
            compare(line.folded, true)
            verify(!line.nameStands, "the name is gone")
            compare(line.countSaid, said, "the count did not move")
            compare(line.countSaid, "+2")
            compare(line.valueFields.length, 0, "nothing named, nothing to sweep")
            verify(line.clipped, "a run reads nobody named as cut")
        }

        // What the line asks for stays the unfolded line's, so a wider row unfolds it again.
        function test_d_folding_asks_for_the_same_width() {
            const asked = line.implicitWidth
            line.width = floorWidth() - 1
            compare(line.folded, true)
            compare(line.implicitWidth, asked)
            line.width = asked
            compare(line.folded, false)
        }

        // One co-author folds to the face alone: nobody after the first, so no count.
        function test_e_one_co_author_folds_to_the_face() {
            line.records = [root.three[0]]
            line.width = Theme.iconMd + Theme.spaceXs + line.nameFloor - 1
            compare(line.folded, true)
            verify(!line.nameStands)
            verify(!line.countStands, "no count comes out to stand in for the name")
        }

        // The rule and the hover end where the drawing does: folded, at the count.
        function test_f_folded_the_rule_ends_at_the_count() {
            line.width = floorWidth() - 1
            tryVerify(() => line.stretchWidth < line.width)
        }

        // A name shorter than three characters is its own floor: it never folds while it fits whole.
        function test_g_a_short_name_is_its_own_floor() {
            line.records = [{ name: "Al", email: "", face: 1 }]
            compare(line.nameFloor, line.nameNatural, "the whole name is less than three characters and a mark")
            line.width = line.naturalWidth
            compare(line.folded, false)
        }
    }
}
