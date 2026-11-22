import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// How the parent line fits the room its row leaves it (`HashPlate.parentFit`): every parent cut evenly from
// whole down to three digits and the mark, then the tail given to `+N` down to one link, then the arrow and the mark
// alone (`←…`). The room is set by hand here; where it comes from is the row's (`details-parents`).
//
// A control, to hand the plate the window's font as `Main` does: the count and the commas are priced in the face their
// labels draw in, and a default face with other figures lays the line out a pixel off its price.
Control {
    id: root
    width: 800
    height: 200
    font.family: Theme.uiFamily
    font.pixelSize: Theme.fontMd

    property string visited: ""

    readonly property var two: ["974a87e8aaaabbbbccccddddeeeeffff00002222", "1b2c3d4eaaaabbbbccccddddeeeeffff00003333"]
    readonly property var eight: {
        const hexes = []
        for (let i = 0; i < 8; i++)
            hexes.push(i + "0a1b2c3aaaabbbbccccddddeeeeffff0000444" + i)
        return hexes
    }

    HashPlate {
        id: plate
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        sha8: "164db4f7"
        fullSha: "164db4f7aaaabbbbccccddddeeeeffff00001111"
        onParentClicked: oidHex => root.visited = oidHex
    }

    TestCase {
        name: "ParentLine"
        when: windowShown

        function init() {
            root.visited = ""
            plate.unrolled = false
            plate.parents = root.two
            plate.parentRoom = root.width
        }

        /// The line laid out at the width the fit priced it at — which is also the claim that the pricing is the
        /// layout's: a line wider than its price runs past the room it was fitted to.
        function laidOut() {
            tryVerify(() => {
                const priced = plate.lineWidth(plate.parentsShown, plate.parentDigits, plate.parentsLeft)
                return Math.abs(plate.parentLineWidth - priced) < 0.5
            })
        }

        /// The links standing, and what each shows.
        function drawn() {
            const shown = []
            for (let i = 0; i < plate.parents.length; i++) {
                const link = plate.linkAt(i)
                if (link && link.visible)
                    shown.push(link.field.text)
            }
            return shown
        }

        function test_a_room_enough_shows_every_parent_whole() {
            plate.parents = root.eight
            plate.parentRoom = plate.lineWidth(8, 8, 0)
            compare(plate.parentsShown, 8)
            compare(plate.parentDigits, 8)
            compare(plate.moreStands, false, "nothing left out, nothing counted")
            laidOut()
            compare(drawn().length, 8)
            compare(drawn()[0], root.eight[0].substring(0, 8), "in git's order")
            compare(drawn()[7], root.eight[7].substring(0, 8))
            verify(!plate.linkAt(0).field.clipped)
        }

        // Short of whole, every parent loses the same digits: a line of uneven hashes reads as parents of two kinds.
        function test_b_less_room_cuts_every_hash_alike() {
            plate.parentRoom = plate.lineWidth(2, 5, 0)
            compare(plate.parentsShown, 2)
            compare(plate.parentDigits, 5)
            laidOut()
            for (let i = 0; i < 2; i++)
                verify(plate.linkAt(i).field.clipped, "link " + i + " is cut")
            compare(plate.linkAt(0).field.text, root.two[0].substring(0, 8), "the field still holds all eight")
        }

        // Past three digits the tail goes, counted: a parent cut to nothing says less than `+N` does.
        function test_c_past_three_digits_the_tail_is_counted() {
            plate.parents = root.eight
            plate.parentRoom = plate.lineWidth(5, 3, 3)
            compare(plate.parentsShown, 5)
            compare(plate.parentDigits, 3)
            compare(plate.moreStands, true)
            compare(plate.moreSaid, "+3")
            laidOut()
            compare(drawn().length, 5)
            compare(drawn()[4], root.eight[4].substring(0, 8), "the ones kept are the first ones")
        }

        // Down to one link, the rest counted; with no room for that, the arrow and the mark that stands for them all.
        function test_d_past_one_link_the_mark_stands_for_them_all() {
            plate.parentRoom = plate.lineWidth(1, 3, 1)
            compare(plate.parentsShown, 1)
            compare(plate.moreSaid, "+1")
            laidOut()
            plate.parentRoom = plate.lineWidth(1, 3, 1) - 1
            compare(plate.parentsShown, 0)
            compare(plate.moreStands, true)
            compare(plate.moreSaid, "…")
            laidOut()
            compare(drawn().length, 0, "no link stands beside the mark")
            plate.parents = root.eight
            plate.parentRoom = 0
            compare(plate.parentsShown, 0)
            compare(plate.moreSaid, "…")
        }

        // One parent gives way as the others do: cut to three digits, then to the mark.
        function test_e_one_parent_gives_way_too() {
            plate.parents = [root.two[0]]
            plate.parentRoom = plate.lineWidth(1, 8, 0)
            compare(plate.parentDigits, 8)
            compare(plate.moreStands, false)
            plate.parentRoom = plate.lineWidth(1, 4, 0)
            compare(plate.parentsShown, 1)
            compare(plate.parentDigits, 4)
            laidOut()
            verify(plate.linkAt(0).field.clipped)
            plate.parentRoom = 0
            compare(plate.parentsShown, 0)
            compare(plate.moreSaid, "…")
            laidOut()
        }

        function test_f_each_link_goes_to_its_own_parent() {
            plate.parents = root.eight
            plate.parentRoom = plate.lineWidth(3, 3, 5)
            laidOut()
            const second = plate.linkAt(1).face
            const p = second.mapToItem(root, Qt.point(second.width / 2, second.height / 2))
            mouseClick(root, p.x, p.y)
            compare(root.visited, root.eight[1], "the second link is the second parent's")
            const first = plate.linkAt(0).face
            const q = first.mapToItem(root, Qt.point(first.width / 2, first.height / 2))
            mouseClick(root, q.x, q.y)
            compare(root.visited, root.eight[0])
        }

        // `a, b, c +5`: a comma after every link but the last one standing, and none before the count.
        function test_g_links_stand_comma_after_comma() {
            plate.parents = root.eight
            plate.parentRoom = plate.lineWidth(3, 3, 5)
            laidOut()
            for (let i = 0; i < 3; i++)
                compare(plate.linkAt(i).followed, i < 2, "link " + i)
            compare(plate.linkAt(0).width, plate.linkAt(0).face.width + plate.commaWidth, "the comma has its room")
            compare(plate.linkAt(2).width, plate.linkAt(2).face.width, "and the last has none")
        }

        // The arrow is a mark: a press on it goes nowhere, and no hash's target reaches it.
        function test_h_the_arrow_takes_no_press() {
            laidOut()
            const arrow = plate.arrowBox(root)
            const p = Qt.point(arrow.x + arrow.width / 2, arrow.y + arrow.height / 2)
            verify(!plate.claims(root, p), "no control claims the arrow")
            mouseClick(root, p.x, p.y)
            compare(root.visited, "", "and a press there went nowhere")
        }

        // With the card out the line stands as one parent's would: the arrow, and the seat the card's first row takes,
        // right where a single parent's hash ends.
        function test_i_unrolled_the_line_is_one_parent_s() {
            plate.parents = root.eight
            plate.parentRoom = plate.lineWidth(3, 3, 5)
            laidOut()
            const right = plate.parentLineLeft(root) + plate.parentLineWidth
            const wide = plate.parentLineWidth
            plate.unrolled = true
            const seat = plate.unrollInset + plate.hashWidth(8)
            // The line lays itself out again a pass after it is asked to.
            tryVerify(() => {
                const at = plate.arrowBox(root)
                return right - (at.x + at.width) === seat
            })
            for (let i = 0; i < plate.parents.length; i++)
                verify(!plate.linkAt(i).visible, "link " + i + " is off the line")
            compare(plate.moreStands, false, "and so is the count")
            compare(plate.parentLineLeft(root) + plate.parentLineWidth, right, "the line still ends where it did")
            // The seat and the arrow stand at the right end; the rest of the width is held, so the plate keeps it.
            compare(plate.parentLineWidth, wide, "and is as wide as it was")
        }

        /// Whether the count's underline runs under the arrow, read off the drawn plate: the rule's ink in the column
        /// through the arrow's middle, below the arrow's own middle.
        function ruleUnderArrow() {
            const image = grabImage(plate)
            const box = plate.arrowBox(plate)
            const x = Math.round(box.x + box.width / 2)
            const rule = Theme.borderStrong
            for (let y = Math.round(box.y + box.height / 2); y < image.height; y++) {
                const c = image.pixel(x, y)
                if (Math.abs(c.r - rule.r) < 0.02 && Math.abs(c.g - rule.g) < 0.02 && Math.abs(c.b - rule.b) < 0.02)
                    return true
            }
            return false
        }

        // With no link standing the arrow and the mark are one stretch (`marked`): resting on the arrow asks for the
        // card as resting on the mark does, and a press there still goes nowhere. Beside a link the arrow is a mark
        // alone and asks for nothing. Hover reaches a handler in the move that brings the pointer, so each answer is
        // read right after its move.
        function test_k_beside_the_mark_the_arrow_is_the_mark_s() {
            plate.parentRoom = plate.lineWidth(1, 3, 1)
            laidOut()
            compare(plate.marked, false)
            let arrow = plate.arrowBox(root)
            mouseMove(root, arrow.x + arrow.width / 2, arrow.y + arrow.height / 2)
            compare(plate.morePointed, false, "beside a link the arrow asks for nothing")
            verify(!ruleUnderArrow(), "and the count's underline stops short of it")
            plate.parentRoom = plate.lineWidth(1, 3, 1) - 1
            laidOut()
            compare(plate.marked, true)
            arrow = plate.arrowBox(root)
            const p = Qt.point(arrow.x + arrow.width / 2 + 1, arrow.y + arrow.height / 2)
            mouseMove(root, p.x, p.y)
            compare(plate.morePointed, true, "beside the mark the arrow asks for the card")
            verify(ruleUnderArrow(), "the mark's underline runs under the arrow")
            verify(!plate.claims(root, p), "no control claims it")
            mouseClick(root, p.x, p.y)
            compare(root.visited, "", "and a press there went nowhere")
            mouseMove(root, 1, 1)
            compare(plate.morePointed, false, "off the line, the ask is gone")
        }

        // The row reads its details again for the same commit (a picture, the tree view) and hands over a new list of
        // the same parents: the links stand as they were, a selection in one kept.
        function test_j_the_same_parents_again_keep_their_links() {
            laidOut()
            const first = plate.linkAt(0)
            plate.selectParent()
            const held = plate.parentSelected()
            verify(held !== "", "a parent's hash is selected")
            plate.parents = root.two.slice()
            compare(plate.linkAt(0), first, "the same link")
            compare(plate.parentSelected(), held, "still selected")
        }
    }
}
