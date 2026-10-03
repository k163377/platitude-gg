import QtQuick
import QtTest
import platitude.ui

// The operation panel's branch card, built from a listing shaped like `NavSectionModel.cardLevel`'s: a folder is a
// card opened off a row saying its name, and a branch row is pressed for its whole name. The card is built as it
// opens, one card at a time — and a card whose folder rows came out empty, or whose rows reach nothing, looks in a
// screenshot exactly like one waiting for its listing.
Item {
    id: root
    width: 800
    height: 600

    /// All the card asks of a page: the section's answer, one card of the nest at a time.
    QtObject {
        id: page
        property var rows: []
        property var asked: []
        readonly property QtObject pageBranches: QtObject {
            function cardLevel(path) {
                page.asked = page.asked.concat([path])
                return page.rows.filter(row => {
                    const cut = row.full.lastIndexOf("/")
                    return (cut < 0 ? "" : row.full.substring(0, cut)) === path
                })
            }
        }
    }

    /// A row as the section answers it. `offers` is the section's to say (a folder's: whether anything under it is on
    /// offer), so a fixture that needs it false says so.
    function row(full, depth, extra) {
        const segments = full.split("/")
        const made = {
            "name": segments[segments.length - 1], "full": full, "depth": depth, "folder": false, "head": false,
            "offers": true, "held": false, "ahead": 0, "behind": 0, "remote": false, "pr": false, "gone": false
        }
        for (const key in extra)
            made[key] = extra[key]
        return made
    }

    OpsBranchMenu {
        id: card
        page: page
        property string picked: ""
        onBranchPicked: name => card.picked = name
    }

    TestCase {
        name: "OpsBranchMenu"
        when: windowShown

        function init() {
            card.close()
            tryCompare(card, "visible", false)
            card.picked = ""
            page.asked = []
            page.rows = [
                root.row("feature", 0, { "folder": true }),
                root.row("feature/tracked", 1, { "ahead": 1, "behind": 2, "remote": true }),
                root.row("main", 0, { "head": true, "offers": false }),
                root.row("rig", 0),
                root.row("topic", 0, { "folder": true }),
                root.row("topic/deep", 1, { "folder": true }),
                root.row("topic/deep/one", 2),
                root.row("topic/two", 1),
                root.row("worktree-a", 0, { "held": true })
            ]
        }

        function rowCalled(menu, text) {
            for (let i = 0; i < menu.count; i++) {
                const item = menu.itemAt(i)
                if (item && item.text === text)
                    return item
            }
            return null
        }

        /// The label that says a row's words.
        function wordsOf(row) {
            const kids = row.contentItem.children
            for (let i = 0; i < kids.length; i++) {
                if (kids[i].text === row.text)
                    return kids[i]
            }
            return null
        }

        /// A folder's row says its name (rules-refs/app-ui.md「`AppMenu` の delegate が作る題名行」).
        function test_a_folder_is_a_row_that_says_its_name_and_opens_its_card() {
            verify(card.offerFrom(), "the card has rows to offer")
            tryCompare(card, "opened", true)
            const feature = rowCalled(card, "feature")
            verify(feature !== null, "the folder's row says the folder's name")
            verify(feature.subMenu !== null, "the folder's row opens a card")
            verify(Qt.colorEqual(wordsOf(feature).color, Theme.textPrimary),
                   "a row that opens a card to its right is not drawn darker than a branch row")
            verify(card.openFolder("feature"))
            compare(rowCalled(feature.subMenu, "tracked").full, "feature/tracked")
            verify(card.openFolder("topic/deep"))
            const deep = rowCalled(rowCalled(card, "topic").subMenu, "deep")
            verify(deep !== null && deep.subMenu !== null, "a folder inside a folder is a card inside a card")
            verify(rowCalled(deep.subMenu, "one") !== null)
        }

        function test_a_folders_rows_are_made_as_its_card_opens() {
            verify(card.offerFrom())
            tryCompare(card, "opened", true)
            compare(page.asked, [""], "the press asked for the top card alone")
            const feature = rowCalled(card, "feature").subMenu
            compare(feature.count, 0, "the folder's card is empty until it opens")
            verify(card.openFolder("feature"))
            tryCompare(feature, "opened", true)
            verify(feature.count > 0)
            feature.close()
            tryCompare(feature, "visible", false)
            verify(card.openFolder("feature"))
            compare(page.asked, ["", "feature"], "and its rows are made once")
        }

        function test_the_current_branch_is_not_offered() {
            verify(card.offerFrom())
            tryCompare(card, "opened", true)
            compare(rowCalled(card, "main").offered, false)
            // feature, rig, topic, worktree-a
            compare(card.offeredRows, 4)
        }

        /// Said by `offers` before the folder's card is made, since an empty card cannot say it.
        function test_a_folder_left_with_nothing_on_offer_takes_its_row_with_it() {
            page.rows = [
                root.row("only", 0, { "folder": true, "offers": false }),
                root.row("only/here", 1, { "head": true, "offers": false }),
                root.row("rig", 0)
            ]
            verify(card.offerFrom())
            tryCompare(card, "opened", true)
            compare(rowCalled(card, "only").offered, false)
            compare(card.offeredRows, 1)
        }

        /// The row shows only the segment under its folders.
        function test_a_row_is_pressed_for_the_whole_name() {
            verify(card.offerFrom())
            tryCompare(card, "opened", true)
            const one = card.rowFor("topic/deep/one")
            verify(one !== null, "the folders' cards are made on the way to it")
            one.triggered()
            compare(card.picked, "topic/deep/one")
        }

        function test_every_row_keeps_the_seat() {
            verify(card.offerFrom())
            tryCompare(card, "opened", true)
            const rig = rowCalled(card, "rig")
            const held = rowCalled(card, "worktree-a")
            verify(rig.seated && held.seated)
            compare(rig.leftPadding, held.leftPadding)
            compare(held.markKind, "tree")
            compare(rowCalled(card, "feature").leftPadding, rig.leftPadding)
        }

        function test_a_card_with_no_mark_holds_no_seat() {
            verify(card.offerFrom())
            tryCompare(card, "opened", true)
            const feature = rowCalled(card, "feature").subMenu
            verify(card.openFolder("feature"))
            tryCompare(feature, "opened", true)
            const tracked = rowCalled(feature, "tracked")
            verify(!feature.seatWorn, "nothing on the folder's card wears a mark")
            verify(!tracked.seated)
            verify(tracked.leftPadding < rowCalled(card, "rig").leftPadding,
                   "the words start before the seat the card above keeps")
        }

        /// No row here is held, so no card is held to a held row's floor — which would leave more air after the
        /// longest name than before the first mark.
        function test_the_cards_fit_their_rows() {
            page.rows = [root.row("f", 0, { "folder": true }), root.row("f/a", 1)]
            verify(card.offerFrom())
            tryCompare(card, "opened", true)
            verify(card.width < Metrics.menuMinW, "the card fits its one short row")
            const folder = rowCalled(card, "f").subMenu
            verify(card.openFolder("f"))
            tryCompare(folder, "opened", true)
            verify(folder.width < Metrics.menuMinW, "the folder's card fits its one short row")
        }

        /// Opened the way resting on its row opens it.
        function test_a_folder_opens_through_the_folders_above_it() {
            verify(card.offerFrom())
            tryCompare(card, "opened", true)
            verify(card.openFolder("topic/deep"))
            tryCompare(card, "folderStanding", "topic/deep")
            verify(!card.openFolder("nowhere"), "a folder the card does not hold opens nothing")
        }

        /// What the last opening built is taken down before the next one builds.
        function test_opening_again_builds_the_card_again() {
            verify(card.offerFrom())
            tryCompare(card, "opened", true)
            const rows = card.count
            card.close()
            tryCompare(card, "visible", false)
            verify(card.offerFrom())
            tryCompare(card, "opened", true)
            compare(card.count, rows)
            page.rows = [root.row("rig", 0), root.row("solo", 0)]
            card.close()
            tryCompare(card, "visible", false)
            verify(card.offerFrom())
            tryCompare(card, "opened", true)
            compare(card.count, 2)
        }
    }
}
