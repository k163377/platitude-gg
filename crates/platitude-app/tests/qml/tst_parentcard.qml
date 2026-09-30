import QtQuick
import QtTest
import platitude.ui

// The parents' card under a real pointer (デザイン規約 §右のペインの親): a press let go where it landed goes to that row's
// parent and takes the card down, one that travels selects the hash and goes nowhere, and a row under the hand wears
// the wash. The headless verbs enter the hands' own functions, so whether Qt hands the press to them is asked here.
Item {
    id: root
    width: 400
    height: 300

    property string picked: ""
    property int picks: 0

    ParentListCard {
        id: card
        parent: root
        x: 40
        y: 20
        listRoom: root.height - card.y
        parents: ["974a87e8aaaabbbbccccddddeeeeffff00002222", "1b2c3d4eaaaabbbbccccddddeeeeffff00003333",
                  "5f6a7b8caaaabbbbccccddddeeeeffff00004444"]
        onPicked: oidHex => { root.picks++; root.picked = oidHex }
    }
    // The line the card opens over, for the seat the two must agree on.
    HashPlate {
        id: seatPlate
        visible: false
    }

    TestCase {
        name: "ParentCard"
        when: windowShown

        function init() {
            root.picked = ""
            root.picks = 0
            card.open()
            tryVerify(() => card.opened && card.rowAt(2) !== null)
        }
        function cleanup() {
            card.close()
        }

        /// A point of row `i`'s hash, in the root's coordinates.
        function hashPoint(i, fx) {
            const f = card.rowAt(i).field
            return f.mapToItem(root, f.width * fx, f.height / 2)
        }

        function test_a_a_click_goes_to_that_row_and_shuts_the_card() {
            const p = hashPoint(1, 0.5)
            mouseClick(root, p.x, p.y)
            compare(root.picks, 1)
            compare(root.picked, card.parents[1], "the second row is the second parent's")
            tryVerify(() => !card.opened)
        }

        // The row is the target, air included: a press beside the hash is still that row's.
        function test_b_a_click_beside_the_hash_is_the_row_s() {
            const row = card.rowAt(2)
            const p = row.mapToItem(root, row.width - 2, row.height / 2)
            mouseClick(root, p.x, p.y)
            compare(root.picked, card.parents[2])
        }

        function test_c_a_drag_selects_the_hash_and_goes_nowhere() {
            const from = hashPoint(0, 0)
            const to = hashPoint(0, 1)
            mousePress(root, from.x, from.y)
            mouseMove(root, (from.x + to.x) / 2, from.y)
            mouseMove(root, to.x, to.y)
            mouseRelease(root, to.x, to.y)
            compare(card.rowAt(0).field.selected, card.parents[0].substring(0, 8))
            compare(root.picks, 0, "a drag is not a click")
            verify(card.opened, "and the card stays")
        }

        function test_d_the_row_under_the_hand_is_lit() {
            const p = hashPoint(1, 0.5)
            mouseMove(root, p.x, p.y)
            tryVerify(() => card.rowAt(1).lit)
            verify(!card.rowAt(0).lit && !card.rowAt(2).lit, "and only that one")
        }

        // One hash wide, on the seat the line keeps for it: every hash whole, and no more air than a row's
        // (デザイン規約 §右のペインの親). The card and the line build the seat apart, so the two are held together here.
        function test_e_the_card_is_one_hash_wide_on_the_line_s_seat() {
            compare(card.hashInset, seatPlate.unrollInset, "the card's air is the air the line keeps")
            compare(card.hashWidth, seatPlate.hashWidth(seatPlate.wholeDigits), "a whole hash, priced as the line's")
            for (let i = 0; i < card.parents.length; i++)
                compare(card.rowAt(i).field.clipped, false, "row " + i + " whole")
        }
    }
}
