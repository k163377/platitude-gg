pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The branches this repository has, as the operation panel's branch name offers them — **filed the way the left
// menu's BRANCHES section files them** (デザイン規約 §操作パネル): a folder is a row of its own, said in that section's
// quieter ink, and the names filed under it are a card of their own, opened off that row the way every nested card
// in this app opens (§メニュー の入れ子) — which is where a hand resting on it is answered. Each row says the segment
// under its folders, as the section's row does, and **a card with a mark on any row keeps the section's seat on every
// row**: the tree mark on a branch another copy has out stands in the column the left menu stands it in, and every
// name begins on one x (`AppMenu.seatWorn` — a card with no mark on it holds no seat).
//
// **Built as it opens, from the section's own answer** (`NavSectionModel.cardLevel`): what a menu offers is decided
// as it opens and left alone (§メニュー), and the answer is the whole section whatever the left menu is folding or
// filtering to — a branch the reader put out of sight over there is still somewhere to go. **One card at a time**:
// the press makes the top card's rows, and a folder's card makes its own the first time it opens. Every row is an
// item made on the spot, so a whole tree made on the press costs the press an item per local branch
// (ci/baseline/code-costs-windows-x64.md).
AppMenu {
    id: card

    /// The page whose branches these are (`TopBar.curPage`).
    property var page: null
    /// The folder whose card is standing, by its path — empty while none is. Automation: which card is up is the
    /// one thing a picture of two cards cropped to the panel cannot say (`TopBar.standDoor`).
    property string folderStanding: ""

    /// A branch row pressed, by the branch's whole name — for the road the section's own rows take
    /// (`RepoPage.switchToRef`).
    signal branchPicked(string name)

    keepsSeat: true
    // No row here is held, so the card is not held to the floor a hold needs: it fits its rows (`AppMenu.widthFloor`).
    widthFloor: 0

    /// Builds the card from the section as it stands and opens it. Says whether it opened: a repository whose one
    /// branch is the one the window stands on has nothing to offer, and an empty card is not an answer
    /// (`AppMenu.offerHere`).
    function offerFrom() {
        card.build()
        return card.offerHere()
    }

    /// Automation: the card of the folder at `path` opened the way resting on its row opens it — hover is the one
    /// input a headless run cannot make, and `openSub` is the road a hand's rest takes (`AppMenu.openSub`). Every
    /// folder above it opens on the way. Says whether the last of them came up.
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

    /// A folder's rows, made the first time its card opens — and not again: what a card offers is decided as it
    /// opens and left alone until the whole card is built again (§メニュー).
    function fillFolder(folder) {
        if (folder.filled)
            return
        folder.filled = true
        card.fill(folder, folder.path)
    }

    /// The rows filed directly under the folder at `path`, made into `into` — a folder's own card, which the rows of
    /// the next depth are made into when it opens.
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
                // Made under the card's own content, the way Qt's own dynamic rows are: a row made under the card
                // itself is an item with no scene to stand in until the card adds it, and says so on every build.
                into.addItem(branchRow.createObject(into.contentItem, {
                    "text": row.name,
                    "full": row.full,
                    // The branch the window is on has no row of its own: a menu offers what can be chosen on the
                    // thing it was opened from (規約 §メニュー), and that is what it was opened from. A folder left
                    // holding nothing else takes its own row with it (`AppMenu.delegate`).
                    "offered": row.offers,
                    "markKind": row.held ? "tree" : "",
                    "ahead": row.ahead,
                    "behind": row.behind,
                    "remoteBadge": row.remote,
                    "badgePr": row.pr,
                    "badgeGone": row.gone
                }))
            }
        }
    }

    /// What the last opening built, taken off before the next one builds. A folder's card goes with every row filed
    /// into it — they were made with it as their parent.
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
            titleFolder: true
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
            // A mark about the row, not a heading: the WORKTREES section's own, in its own colour, standing where
            // the left menu stands it (`NavRowBody.seatMark`). The press still lands: it goes to that copy
            // (`switchToRef`).
            headed: false
            markTint: Theme.success
            onTriggered: card.branchPicked(leaf.full)
        }
    }
}
