import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The card a chip unfolds into is measured before it opens (`RowHoverHost.openRefList`), while nothing inside a closed
// popup is visible. A row's room read off `visible` came to nothing there, and the room it took back once the card was
// up came out of the chip — cutting the line under a tag that names whose reading it is. Only the real card, measured
// and then opened, can answer it.
Item {
    id: root
    width: 400
    height: 300

    /// A tag only mirror has on this commit, as `encode::chips_of` hands it (every field spelled out).
    readonly property var mirrors: ({ "kind": "tag", "name": "v3.2-split", "isHead": false, "hasRemote": false,
                                      "hasPr": false, "here": false, "held": false, "locked": false,
                                      "remote": "mirror", "key": "tag:v3.2-split" })

    RefListPopup {
        id: card
        chipRoom: 600
    }

    TestCase {
        name: "RefListRoom"
        when: windowShown

        function test_a_row_naming_whose_reading_it_is_keeps_its_room_through_the_opening() {
            card.records = [root.mirrors]
            // A line under the name, as a tag standing apart opens on (`NavFacts.apartLine`).
            card.mates = [{ "mark": "remote", "markTint": Theme.warning, "text": "Remotes disagree",
                            "tone": Theme.warning, "ahead": 0, "behind": 0, "note": "", "to": null }]
            card.listRoom = root.height
            card.layOutRows()
            card.open()
            tryCompare(card, "opened", true)
            const row = card.rowAt(0)
            verify(row !== null)
            verify(row.whoseRoom > 0, "the reading's remote stands at the row's end")
            verify(row.chip.width >= Math.ceil(row.chip.mateWidth) + 2 * Theme.spaceXs,
                   "the line under the name fits its frame: " + row.chip.width + " for " + row.chip.mateWidth)
            card.close()
        }
    }
}
