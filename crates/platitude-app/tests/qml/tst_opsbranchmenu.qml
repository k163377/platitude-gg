import QtQuick
import QtTest
import platitude.ui

// The operation panel's branch card, built from a listing the way it is built from the section's own answer
// (`NavSectionModel.cardLevel`): a folder a card of its own, opened off a row that says the folder's name, and every
// branch a row pressed for its whole name. **The card is built as it opens, a card at a time**, so what is pinned
// here is the build — a card whose folder rows came out empty, or whose rows reached nothing when pressed, frames in
// a screenshot exactly like one waiting for its listing.
Item {
    id: root
    width: 800
    height: 600

    /// The two things the card asks of a page: the section's answer, a card of the nest at a time, and nothing else.
    QtObject {
        id: page
        property var rows: []
        /// How many times the card asked, and for which folders — what the press itself costs.
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

    /// A row the way the section answers it. `offers` is the section's to say — a folder's is whether anything under
    /// it is on offer — so a fixture that needs it otherwise says so.
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

        /// A folder is a row that opens the card of what is filed under it, and **says its name** — the row Qt builds
        /// for a card is handed the title once, mid-build, and a row that wrote over it drew an empty folder.
        function test_a_folder_is_a_row_that_says_its_name_and_opens_its_card() {
            verify(card.offerFrom(), "the card has rows to offer")
            tryCompare(card, "opened", true)
            const feature = rowCalled(card, "feature")
            verify(feature !== null, "the folder's row says the folder's name")
            verify(feature.subMenu !== null, "the folder's row opens a card")
            verify(feature.folderRow, "the folder's row is drawn as a folder")
            verify(card.openFolder("feature"))
            compare(rowCalled(feature.subMenu, "tracked").full, "feature/tracked")
            verify(card.openFolder("topic/deep"))
            const deep = rowCalled(rowCalled(card, "topic").subMenu, "deep")
            verify(deep !== null && deep.subMenu !== null, "a folder inside a folder is a card inside a card")
            verify(rowCalled(deep.subMenu, "one") !== null)
        }

        /// **The press makes the top card and nothing under it**: every row is an item made on the spot, and a
        /// folder's rows are made the first time its card opens — once, however often it opens.
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

        /// The branch the window stands on is on the list and not on offer; everything else is.
        function test_the_current_branch_is_not_offered() {
            verify(card.offerFrom())
            tryCompare(card, "opened", true)
            compare(rowCalled(card, "main").offered, false)
            // feature, rig, topic, worktree-a
            compare(card.offeredRows, 4)
        }

        /// A folder holding nothing but the current branch has nothing to offer, and its row goes with it — **said
        /// before its card is made** (`offers`), since an empty card cannot say it.
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

        /// A row pressed is a branch picked **by its whole name** — the row says the segment under its folders.
        function test_a_row_is_pressed_for_the_whole_name() {
            verify(card.offerFrom())
            tryCompare(card, "opened", true)
            const one = card.rowFor("topic/deep/one")
            verify(one !== null, "the folders' cards are made on the way to it")
            one.triggered()
            compare(card.picked, "topic/deep/one")
        }

        /// Every row keeps the seat, and a row with a mark about itself wears it there, in the WORKTREES colour.
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

        /// …and a card with no mark on any of its rows holds no seat: its words start where every other card's do.
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

        /// Neither the card nor a folder's is held to the floor a held row needs: no row here is held, and a card
        /// held to it keeps more air after its longest name than before its first mark.
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

        /// A folder's card is opened the way resting on its row opens it, every folder above it on the way.
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
