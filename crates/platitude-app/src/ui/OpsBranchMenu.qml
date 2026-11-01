pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The operation panel's branch card, filed as the left menu's BRANCHES section (user decision; the panel's cards follow
// the left menu — `TopBar`'s header), a folder being a row that opens a nested card under a hand at rest (§メニュー
// の入れ子); a folder's row says only its segment, as the left menu's, and a branch row acts on the whole name. The
// rows' ends follow the left menu's order: counts, then the cloud (remote / PR / gone badges).
//
// **Built as it opens, one card at a time**: the press makes the top card only,
// and a folder's card makes its rows the first time it opens (`onAboutToShow`; automation `fillFolder` / `rowFor`) —
// a whole tree on the press costs an item per local branch (`NavSectionModel.cardLevel`,
// ci/baseline/code-costs-windows-x64.md). Each level is the whole branch tree's, whatever the left menu folds or
// filters to. A folder whose card would hold nothing has no row (`AppMenu.delegate`), so whether a folder offers
// anything comes from Rust before its card is made (`CardRow.offers` → `AppMenu.promises`).
//
// A folder's row is in the rows' own `wordColor`: a row that opens with `>` is never dimmed, a heading neither
// (`AppMenuItem` `font.weight`; pinned by `tests/qml/tst_opsbranchmenu.qml`).
AppMenu {
    id: card

    /// The page whose branches these are (`TopBar.curPage`).
    property var page: null
    /// Automation: the path of the folder whose card is standing, empty while none is — what a cropped picture of two
    /// cards cannot say (`TopBar.standDoor`).
    property string folderStanding: ""

    /// A branch row pressed, by the branch's whole name (`RepoPage.switchToRef`).
    signal branchPicked(string name)

    keepsSeat: true
    // No row here is held, so the card fits its rows (`AppMenu.widthFloor`).
    widthFloor: 0

    /// Builds the card and opens it. Says whether it opened: with only the current branch there is nothing to offer
    /// (`AppMenu.offerHere`).
    function offerFrom() {
        card.build()
        return card.offerHere()
    }

    /// Automation: opens the folder at `path` by the road a hand's rest takes (`AppMenu.openSub`), each folder above
    /// it on the way. Says whether the last came up.
    function openFolder(path) {
        let menu = card
        const parts = path.split("/")
        for (let depth = 0; depth < parts.length; depth++) {
            const sub = card.folderIn(menu, parts.slice(0, depth + 1).join("/"))
            if (sub === null)
                return false
            card.fillFolder(sub)
            if (!menu.openSub(sub))
                return false
            menu = sub
        }
        return true
    }

    /// Automation: the row of the branch called `full`, its folders' cards made on the way without opening them —
    /// null where the card holds no such branch (`TopBar.pickBranchRow`).
    function rowFor(full) {
        let menu = card
        const parts = full.split("/")
        for (let depth = 0; depth + 1 < parts.length; depth++) {
            menu = card.folderIn(menu, parts.slice(0, depth + 1).join("/"))
            if (menu === null)
                return null
            card.fillFolder(menu)
        }
        for (let i = 0; i < menu.count; i++) {
            const row = menu.itemAt(i)
            if (row && !row.subMenu && row.full === full)
                return row
        }
        return null
    }

    /// The card of the folder at `path` filed directly in `menu`, or null.
    function folderIn(menu, path) {
        for (let i = 0; i < menu.count; i++) {
            const row = menu.itemAt(i)
            if (row && row.subMenu && row.subMenu.path === path)
                return row.subMenu
        }
        return null
    }

    /// The top card's rows, made afresh — the folders' cards make theirs as they open (`fillFolder`).
    function build() {
        card.takeDown()
        card.fill(card, "")
    }

    /// A folder's rows, made the first time its card opens and not again until the whole card is rebuilt (§メニュー).
    function fillFolder(folder) {
        if (folder.filled)
            return
        folder.filled = true
        card.fill(folder, folder.path)
    }

    /// Makes the rows filed directly under the folder at `path` into the card `into`.
    function fill(into, path) {
        if (card.page === null)
            return
        const rows = card.page.pageBranches.cardLevel(path)
        for (let i = 0; i < rows.length; i++) {
            const row = rows[i]
            if (row.folder) {
                // `promises`: the card is empty until it opens, and the row that opens it is on offer only where
                // something under it is (`AppMenu.delegate`).
                into.addMenu(folderCard.createObject(into, {
                    "title": row.name, "path": row.full, "promises": row.offers, "above": into
                }))
            } else {
                // Under the card's `contentItem`: made under the card itself, the item has no scene until the card
                // adds it, and Qt warns on every build (`Created graphical object was not placed in the graphics
                // scene`). A folder's card is made under its parent card and goes in with `addMenu`.
                into.addItem(branchRow.createObject(into.contentItem, {
                    "text": row.name,
                    "full": row.full,
                    // The current branch has no row — it is what the menu was opened from (規約 §メニュー); a folder
                    // left holding nothing else goes too (`AppMenu.delegate`).
                    "offered": row.offers,
                    // The repository's own worktree's branch wears the house (`NavSectionModel.homeBranch`).
                    "markKind": !row.held ? ""
                              : card.page !== null && card.page.pageWorktrees
                                && row.full === card.page.pageWorktrees.homeBranch ? "home"
                              : "tree",
                    "ahead": row.ahead,
                    "behind": row.behind,
                    "remoteBadge": row.remote,
                    "badgePr": row.pr,
                    "badgeGone": row.gone
                }))
            }
        }
    }

    /// What the last opening built, taken off before the next one builds. A folder's card takes its rows with it —
    /// they were made with it as their parent.
    function takeDown() {
        card.folderStanding = ""
        while (card.count > 0) {
            const row = card.itemAt(0)
            if (row && row.subMenu)
                card.removeMenu(row.subMenu)
            else
                card.removeItem(row)
        }
    }

    Component {
        id: folderCard
        AppMenu {
            id: folder
            /// What git's names under it start with — the folder's path, which is also what a run names it by.
            property string path: ""
            /// The card this one is filed in — where the one standing is, once this one shuts.
            property var above: null
            /// Whether its rows have been made (`fillFolder`).
            property bool filled: false
            keepsSeat: true
            widthFloor: 0
            // A hand resting on the row opens the card before anything else asks it for rows.
            onAboutToShow: card.fillFolder(folder)
            onOpened: card.folderStanding = folder.path
            onClosed: {
                if (card.folderStanding !== folder.path)
                    return
                const outer = folder.above
                card.folderStanding = outer !== null && outer !== card && outer.opened ? outer.path : ""
            }
        }
    }
    Component {
        id: branchRow
        AppMenuItem {
            id: leaf
            /// The branch's whole name — what the row is pressed for, where `text` is the segment it says.
            property string full: ""
            // The mark is about the row, not a heading: the WORKTREES section's own, where the left menu stands it
            // (`NavRowBody.seatMark`). The press still lands — on that worktree (`switchToRef`).
            headed: false
            markTint: Theme.success
            onTriggered: card.branchPicked(leaf.full)
        }
    }
}
