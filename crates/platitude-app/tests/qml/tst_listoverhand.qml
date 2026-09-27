import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The chip's list opens over the chip under a hand that has stopped there, so the row under it hears the hand leave
// the moment the card is drawn. The row answers that leave as it answers any other (`GraphRowDelegate.pointerCrossed`)
// — the list is let go and stands only for the hand inside the card — and the card keeps it because Qt hands the card
// the hand in the same delivery that tells the row it left, well inside the beat the leave starts (`HoverCardHost`).
// Measured with a real pointer: a headless run has none (rules-refs/app-ui.md, the chip list's hold).
Item {
    id: root
    width: 400
    height: 300

    /// The row asked for the list and the hand has not gone into the card — `RowHoverHost.refListWanted`.
    property bool wanted: false
    /// Whether the card had the hand by the time the row heard it leave.
    property bool cardFirst: false

    /// A record of `kind`, every field spelled out as `encode::chips_of` hands it.
    function record(kind, name) {
        return { "kind": kind, "name": name, "isHead": false, "hasRemote": false, "hasPr": false, "here": true,
                 "held": false, "locked": false, "remote": "", "key": kind + ":" + name }
    }
    /// What the row's chip column asks of the page (`RowHoverHost.openRefList`, cut down): held when already out,
    /// else measured and put on the row's right end, where a right-aligned chip stands.
    function ask() {
        root.wanted = true
        if (card.opened)
            return
        card.records = [root.record("branch", "main"), root.record("tag", "v1.0")]
        card.listRoom = root.height
        card.layOutRows()
        card.x = row.x + row.width - card.cardWidth
        card.y = row.y
        card.open()
    }

    // A graph row as far as hover goes: one area over the whole row, answering every arrival and every leave.
    MouseArea {
        id: row
        x: 20
        y: 100
        width: 300
        height: Theme.graphRowHeight
        hoverEnabled: true
        onContainsMouseChanged: {
            if (row.containsMouse) {
                root.ask()
                return
            }
            root.cardFirst = card.pointerInside
            root.wanted = false
            keep.settle()
        }
    }

    RefListPopup {
        id: card
        chipRoom: 200
        onPointerInsideChanged: {
            if (card.pointerInside)
                root.wanted = false
        }
    }
    HoverCardHost {
        id: keep
        card: card
        pointedAt: root.wanted
    }

    TestCase {
        name: "ListOverHand"
        when: windowShown

        function init() {
            // A corner nothing reaches, and the beat the card leaves on.
            mouseMove(root, root.width - 1, root.height - 1)
            root.wanted = false
            root.cardFirst = false
            card.close()
            tryVerify(() => !card.visible, undefined, "each case starts with nothing out")
        }

        function test_a_list_drawn_over_a_still_hand_keeps_it() {
            // Onto the row where the card is about to stand, and still from then on.
            mouseMove(root, row.x + row.width - Theme.spaceSm, row.y + row.height / 2)
            tryVerify(() => card.opened, undefined, "the row asked for the list")
            tryVerify(() => card.pointerInside && !row.containsMouse, undefined,
                      "the card is drawn over the hand and takes it off the row")
            verify(root.cardFirst, "the card had the hand by the time the row heard it leave")
            verify(!root.wanted, "the row let the list go on that leave")
            tryVerify(() => !keep.keep.running, undefined, "the beat the leave started ran out")
            verify(card.opened, "and the card stands under the hand, which it holds itself")
        }

        function test_b_a_hand_gone_from_the_bare_stretch_takes_it_down() {
            // The stretch of the row the card leaves bare, and from there to no row at all.
            mouseMove(root, row.x + Theme.spaceSm, row.y + row.height / 2)
            tryVerify(() => card.opened, undefined, "the row asked for the list")
            verify(!card.pointerInside, "the card stands beside the hand, not under it")
            mouseMove(root, row.x + Theme.spaceSm, Theme.spaceSm)
            tryVerify(() => !card.opened, undefined, "the leave lets it go and the beat takes it down")
        }

        function test_c_a_hand_back_from_the_card_is_held() {
            mouseMove(root, row.x + Theme.spaceSm, row.y + row.height / 2)
            tryVerify(() => card.opened, undefined, "the row asked for the list")
            // Into the card, then back out onto the bare stretch: the row hears the hand arrive and asks again.
            mouseMove(card.background, card.background.width / 2, card.background.height / 2)
            tryVerify(() => card.pointerInside, undefined, "the hand is in the card")
            mouseMove(root, row.x + Theme.spaceSm, row.y + row.height / 2)
            tryVerify(() => row.containsMouse && !card.pointerInside, undefined, "the hand is back on the row")
            verify(root.wanted, "the row asked for the list again")
            tryVerify(() => !keep.keep.running, undefined, "the beat the card's leave started ran out")
            verify(card.opened, "and the list stands for the hand on its row")
        }
    }
}
