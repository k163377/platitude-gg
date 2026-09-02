pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

// The right-click on a ref, wherever it is drawn: the sidebar's rows, a chip on a graph row, a row of the list a chip
// had to stack. One menu for all of them, so a branch offers the same things wherever it is met.
//
// What it offers is decided as it opens and held while it stands: the conditions are live (a timer fetch alone moves
// `busyCount`), and a row that appears or vanishes under the pointer is a row clicked by accident (デザイン規約 §メニュー).
//
// An `Item` because a menu measures the window through the item it was declared under (`AppMenu.ownerItem`); it draws
// nothing itself.
Item {
    id: refRowMenu

    required property RepoTab repoTab
    required property WorkTreeModel workTree
    /// Handed to the branch card, which reads the remote a branch speaks for and the working copy that may be
    /// holding it out of them (`RefBranchMenu`). `switch` asks the second one here as well — it is the only row on
    /// this level that the answer changes.
    required property NavSectionModel branchesModel
    required property NavSectionModel worktreesModel
    /// Handed to the tag card, which reads the sides a name stands on and where a remote last had it out of this
    /// section alone (`RefTagMenu`).
    required property NavSectionModel tagsModel

    /// Why every row here is out, in one line, while the window's write doors are held — a write that replays is
    /// running behind the screen (`RepoPage.doorsHeldWhy`). Handed down rather than worked out here: the left pane
    /// holds the same doors off the same answer, and two readings of "is a rewrite under way" would be two answers.
    /// **The rows grey rather than go** — see `AppMenu.heldReason`.
    property string heldReason: ""
    /// What the offers are asked with while that is standing: **nothing running**. The hold already answers for every
    /// row here, and the busy count would answer a second time by taking rows away — leaving a menu that is a
    /// different shape on each side of a lock the reader is waiting to see come off. A row that is only ever *out* is
    /// read as one the menu does not have; a row that greys and says why is read as the wait it is
    /// (デザイン規約 §メニュー の例外: 「今できない」行は消えず無効になる).
    readonly property int askBusy: refRowMenu.heldReason !== "" ? 0 : refRowMenu.repoTab.busyCount

    /// The row the menu stands on. `refId` is what git knows it by, which on a stash is a selector rather than
    /// the message the row shows.
    property string kind: ""
    property string refId: ""
    property string refOid: ""

    /// What this menu offers, decided as it opens (see the note above).
    property bool canSwitch: false
    /// Whether that row will raise a question rather than move — an operation standing, files still waiting on a
    /// decision, or the branch out in another working copy. Worn as the `!` in the row's mark seat, so the press is
    /// read for what it is before it is made (デザイン規約 §進行中の操作から出る).
    property bool switchAsks: false
    property bool canBranchHere: false
    property bool canIntegrateFrom: false
    /// The stash's own drop — the only delete this level answers for. What a branch's name and a tag's name offer is
    /// their own cards' question, worked out inside them (`RefBranchMenu` / `RefTagMenu`).
    property bool canDelete: false

    // ---- bringing two lines of history together --------------------
    /// The commits a rebase onto this row would rewrite. Asked as the menu opens — the answer is a whole git call away,
    /// and it lands in the page's one answer slot (`RepoPage`).
    readonly property string rebaseRange:
        (refRowMenu.kind === "branch" || refRowMenu.kind === "remote")
        && refRowMenu.refId !== "" ? refRowMenu.refId + "..HEAD" : ""
    property bool rebasePublished: false

    /// Whether the card is on screen, and whether it has finished opening: the folded list asks the first, the chip's
    /// stacked list asks the second. Plain properties rather than aliases — `visible` read from another file comes back
    /// stale (`RefusalBadge`).
    readonly property bool showing: refMenu.visible
    readonly property bool opened: refMenu.opened

    /// The automation's handles into these rows, an automation-only exposure the same as `GraphPane.view` is
    /// (app-ui.md).
    readonly property alias menu: refMenu
    readonly property alias branchHereItem: refBranchHereItem
    readonly property alias tagHereItem: tagMenu.tagHereItem
    readonly property alias pushTagItem: tagMenu.pushTagItem
    readonly property alias deleteTagItem: tagMenu.deleteTagItem
    readonly property alias deleteRemoteTagItem: tagMenu.deleteRemoteTagItem
    readonly property alias deleteTagBothItem: tagMenu.deleteTagBothItem
    readonly property alias deleteItem: branchMenu.deleteItem
    readonly property alias upstreamItem: branchMenu.upstreamItem
    readonly property alias stashDropItem: refStashDropItem
    readonly property alias switchItem: refSwitchItem
    /// The two cards the rows above hang behind — a run that photographs one of those rows has to open its card
    /// first (`AppMenu.openSub`).
    readonly property alias branchCard: branchMenu
    readonly property alias tagCard: tagMenu

    /// What the page answers for: moving the working tree, the delete git may still refuse, and the stash drop that two
    /// menus share.
    signal switchRequested(string kindLetter, string name)
    /// A new branch on whatever commit this row stands on — the page owns where the box for its name opens, which is
    /// wherever the menu was opened from.
    signal branchHereRequested(string oidHex)
    /// A tag on this row's commit — the same box, opened where the menu was.
    signal tagHereRequested(string oidHex)
    signal deleteRequested(string kind, string id, string name, string oidHex)
    signal dropStashRequested(string selector)
    /// Which remote branch a local one is measured against — the branch card's row, answered in the page's one bar
    /// (`UpstreamFlow`).
    signal upstreamRequested(string branch, string counterpart)
    /// A ref this menu has just asked git to delete, so the window can show it as gone while the write is out
    /// (デザイン規約 §消す操作は先に画面から消す). One per ref — `Delete both` names two. `kind` is `branch` / `remote` /
    /// `tag`, `id` what git knows it by. Raised beside the write rather than instead of it: the rows the page owns
    /// (`deleteRequested`, `dropStashRequested`) take themselves away where they are answered.
    signal deleting(string kind, string id)
    /// The menu went away — with it goes a refused delete's offer, and the stacked list it may have been standing on is
    /// the pointer's to answer for again.
    signal dismissed()

    anchors.fill: parent

    /// Opens on that ref, deciding there and then what it offers. Says whether it opened at all: a ref with nothing to
    /// offer — the current branch met as a chip — falls back to the row's own menu.
    function offerOn(kind, name, full, oidHex) {
        refRowMenu.kind = kind
        refRowMenu.refId = full
        refRowMenu.refOid = oidHex
        refRowMenu.rebasePublished = false
        // Both cards are their own question, and each works its answers out for itself — the very cards the graph
        // row's menu carries, asked the same way (RefBranchMenu / RefTagMenu).
        branchMenu.offerOn(kind, name, full, oidHex)
        tagMenu.offerOn(kind, full, oidHex)
        // Whether another working copy has this row's branch out. **git refuses `switch` for one** (measured), and the
        // row wears the `!` for it rather than greying — this level's one use of the answer; the delete rows read
        // their own copy inside the card. A remote row lands on the local branch of the same name, so it is that one
        // another copy can be holding.
        const held = kind === "branch" ? refRowMenu.worktreesModel.worktreeHolding(full)
                   : kind === "remote" ? refRowMenu.worktreesModel.worktreeHolding(
                                             refRowMenu.repoTab.localNameFor(full))
                                       : ""
        // The lookups above are the models'; what the rows may offer on them is core's rule, with the measured
        // refusals it encodes — held elsewhere keeps the switch row (offers::ref_menu). Asked the once, so the
        // answers stand while the menu does (see the note above).
        const offers = GitFacts.refMenuOffers(
            kind, full, oidHex,
            refRowMenu.repoTab.state === "open", refRowMenu.askBusy,
            refRowMenu.workTree.branch, refRowMenu.workTree.detached,
            refRowMenu.workTree.opText, refRowMenu.workTree.conflictCount,
            held, "", refRowMenu.repoTab.defaultRemote, "").split(" ")
        refRowMenu.canSwitch = offers.includes("switch")
        refRowMenu.switchAsks = offers.includes("asks")
        refRowMenu.canBranchHere = offers.includes("branch-here")
        refRowMenu.canIntegrateFrom = offers.includes("integrate")
        refRowMenu.canDelete = offers.includes("delete")
        if (refRowMenu.repoTab.state === "open" && refRowMenu.rebaseRange !== "")
            refRowMenu.repoTab.checkPublish(refRowMenu.rebaseRange)
        return refMenu.offer()
    }

    /// A write that landed has nothing left for the menu to catch.
    function close() {
        refMenu.close()
    }

    AppMenu {
        id: refMenu
        heldReason: refRowMenu.heldReason
        // A refused delete's offer is not cleared here: the card puts it back as it opens (`RefBranchMenu.offerOn`),
        // and nothing reads it while the card is down.
        onClosed: refRowMenu.dismissed()
        // **The card holds what moves the reader; the rest is behind a mark** (デザイン規約 §メニュー の入れ子). Every row on
        // this level answers the question the reader came with — where am I, and where do I go from here — and the
        // cards at the foot hold what is *done to* a ref rather than gone from it.
        //
        // A branch of one's own, started where this row stands, is one of those moves and not a thing done to this
        // ref: it is where the reader carries on from. **Beside `switch`, in one group**: what it runs is
        // `switch --create`, so it is that row's other form — the move to a branch that does not exist yet
        // (デザイン規約 §ブランチ・コミットへの移動). Same words in the same seat as the graph row's menu
        // (§メニュー: 入口が違っても同じ操作は同じ文).
        AppMenuItem {
            id: refBranchHereItem
            text: Words.createBranchHere
            offered: refRowMenu.canBranchHere
            onTriggered: refRowMenu.branchHereRequested(refRowMenu.refOid)
        }
        AppMenuItem {
            id: refSwitchItem
            code: "switch"
            offered: refRowMenu.canSwitch
            // **Never blocked by what stands in the move's way**: a branch another copy holds and a tree with unmerged
            // files both press through to a question instead, and the mark is what says so before the press
            // (デザイン規約 §進行中の操作から出る). Greying is the delete rows' answer, not this one's — a row that cannot be
            // pressed says why only on hover, and the reader who reached for it is the one who needs to read it.
            //
            // **The doors being held is the other thing**, and this row is held with the rest of them (`heldReason`):
            // there is no question to raise there, the answer being to wait — and a move let go into the middle of a
            // rewrite is the exit nobody meant (§フル interactive rebase).
            blockedReason: ""
            asks: refRowMenu.switchAsks
            // Through the chips' dispatcher: a remote branch whose local one already exists cannot simply be created.
            onTriggered: refRowMenu.switchRequested(refRowMenu.kind === "remote" ? "R" : "L", refRowMenu.refId)
        }
        AppMenuSeparator {}
        AppMenuItem {
            code: "merge"
            //: Follows the `merge` chip: "merge into main".
            text: qsTr("into %1").arg(refRowMenu.workTree.branch)
            offered: (refRowMenu.kind === "branch"
                      || refRowMenu.kind === "remote"
                      || refRowMenu.kind === "tag")
                     && refRowMenu.canIntegrateFrom
            onTriggered: refRowMenu.repoTab.merge(refRowMenu.refId, false, false, "")
        }
        AppMenuItem {
            code: "rebase"
            //: Follows the `rebase` chip: "rebase main onto it".
            text: qsTr("%1 onto it").arg(refRowMenu.workTree.branch)
            // Deliberately not offered on a tag — that row belongs with the tag's own gestures.
            offered: (refRowMenu.kind === "branch" || refRowMenu.kind === "remote") && refRowMenu.canIntegrateFrom
            // Said, not asked (要望: rewriting a pushed commit shows a warning). Published = reachable from a
            // remote-tracking ref, only as fresh as the last fetch.
            note: refRowMenu.rebasePublished ? Words.rewritesPushed : ""
            onTriggered: refRowMenu.repoTab.rebase(refRowMenu.refId, "", true)
        }
        // A stash has one thing done to it and nothing to nest: it is not a branch and not a tag, so its drop stays on
        // the card where the reader found it.
        AppMenuItem {
            id: refStashDropItem
            code: "drop"
            offered: refRowMenu.kind === "stash" && refRowMenu.canDelete
            holdMs: Metrics.holdMs
            holdTone: Theme.danger
            onHeld: {
                refMenu.close()
                refRowMenu.dropStashRequested(refRowMenu.refId)
            }
        }
        AppMenuSeparator {}
        // The deletes a branch's name answers for — its own file, because the graph row's menu carries the very same
        // card (RefBranchMenu).
        RefBranchMenu {
            id: branchMenu
            heldReason: refRowMenu.heldReason
            repoTab: refRowMenu.repoTab
            workTree: refRowMenu.workTree
            branchesModel: refRowMenu.branchesModel
            worktreesModel: refRowMenu.worktreesModel
            onDeleteRequested: (kind, id, name, oidHex) => refRowMenu.deleteRequested(kind, id, name, oidHex)
            onDeleting: (kind, id) => refRowMenu.deleting(kind, id)
            onUpstreamRequested: (branch, counterpart) => refRowMenu.upstreamRequested(branch, counterpart)
            onCloseRequested: refMenu.close()
        }
        AppMenuSeparator {}
        // Everything a tag's name answers for, behind its own mark — the very card the graph row's menu carries
        // (RefTagMenu).
        RefTagMenu {
            id: tagMenu
            heldReason: refRowMenu.heldReason
            repoTab: refRowMenu.repoTab
            workTree: refRowMenu.workTree
            tagsModel: refRowMenu.tagsModel
            canBranchHere: refRowMenu.canBranchHere
            onTagHereRequested: oidHex => refRowMenu.tagHereRequested(oidHex)
            onDeleting: (kind, id) => refRowMenu.deleting(kind, id)
            onCloseRequested: refMenu.close()
        }
    }
}
