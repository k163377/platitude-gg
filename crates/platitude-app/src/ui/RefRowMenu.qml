pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

// The right-click on a ref wherever it is drawn — sidebar row, graph chip, a row of a chip's stacked list — so a
// branch offers the same things everywhere. Decided as it opens and frozen while it stands (デザイン規約 §メニュー).
//
// An `Item` because a menu measures the window through the item it was declared under (`AppMenu.ownerItem`).
Item {
    id: refRowMenu

    required property RepoTab repoTab
    required property WorkTreeModel workTree
    /// Answers the `rebase` row's note off the drawn rows (`rebasePublished`).
    required property GraphModel graphModel
    /// Both read for the branch card (`branchFacts`); `switch` also asks `worktreesModel` who holds the branch.
    required property NavSectionModel branchesModel
    required property NavSectionModel worktreesModel
    /// Read for the tag card (`tagFacts`).
    required property NavSectionModel tagsModel

    /// Why every row is out while the window's write doors are held (`RepoPage.doorsHeldWhy`), handed down so this
    /// and the left pane read one answer. The rows grey (`AppMenu.heldReason`).
    property string heldReason: ""
    /// The busy count the offers are asked with — zero while held, so the rows grey instead of vanishing and the menu
    /// keeps its shape across the lock (デザイン規約 §メニュー「例外は履歴を書き換える write が走っている間」).
    readonly property int askBusy: refRowMenu.heldReason !== "" ? 0 : refRowMenu.repoTab.busyCount

    /// The row the menu stands on. `refId` is what git knows it by, which on a stash is a selector, apart from
    /// the row's message.
    property string kind: ""
    property string refId: ""
    property string refOid: ""

    property bool canSwitch: false
    /// Whether `switch` will raise a question (an operation standing, or unmerged files), worn as the row's `!`
    /// (デザイン規約 §進行中の操作から出る).
    property bool switchAsks: false
    /// The leaf of the working copy holding this row's branch, empty when none does; the switch row then opens that
    /// copy and names it (`RepoPage.switchToRef`). The leaf is how a copy is named everywhere (デザイン規約 §ref の種別).
    property string heldLeaf: ""
    property bool canBranchHere: false
    property bool canIntegrateFrom: false
    /// Whether this row is either end of the current branch's upstream comparison, where `git pull` runs
    /// (offers::ref_menu).
    property bool canPull: false
    /// Whether the sides have diverged, which greys the row (`WorkTreeModel.pullBlocked`).
    property bool pullBlocked: false
    /// The stash's drop — the only delete on this level; branch and tag deletes are their cards' (`RefBranchMenu` /
    /// `RefTagMenu`).
    property bool canDelete: false

    // ---- bringing two lines of history together --------------------
    /// Whether merge / rebase are on offer — one reading for both rows and the rebase note, so the note appears
    /// exactly where its row does. A stash is not a line of history.
    readonly property bool integrateOffered:
        (refRowMenu.kind === "branch" || refRowMenu.kind === "remote" || refRowMenu.kind === "tag")
        && refRowMenu.canIntegrateFrom
    /// Whether a rebase onto this row would rewrite a commit a remote has (`<ref>..HEAD`), read off the drawn rows
    /// (`GraphModel.rebaseRewritesPublished`): a `git rev-list` would land after the card and widen it under the hand
    /// (規約 §行が読む答えはどこから来るか).
    property bool rebasePublished: false

    /// On screen, and finished opening — asked by the folded list and the chip's stacked list. Plain properties:
    /// `visible` read from another file comes back stale (`RefusalBadge`).
    readonly property bool showing: refMenu.visible
    readonly property bool opened: refMenu.opened

    /// Automation-only handles into these rows, like `GraphPane.view`.
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
    readonly property alias pullItem: refPullItem
    readonly property alias rebaseItem: refRebaseItem
    /// The cards those rows hang behind; a run photographing one opens it first (`AppMenu.openSub`).
    readonly property alias branchCard: branchMenu
    readonly property alias tagCard: tagMenu

    /// What the page answers for: moving the working tree, the delete git may still refuse, and the stash drop that two
    /// menus share.
    signal switchRequested(string kind, string name)
    /// A new branch on this row's commit; the page opens the name box where the menu was opened.
    signal branchHereRequested(string oidHex)
    /// A tag on this row's commit — the same box, opened where the menu was.
    signal tagHereRequested(string oidHex)
    signal deleteRequested(string kind, string id, string name, string oidHex)
    signal dropStashRequested(string selector)
    /// Which remote branch a local one is measured against — the branch card's row, answered in the page's one bar
    /// (`UpstreamFlow`).
    signal upstreamRequested(string branch, string counterpart)
    /// The menu went away: the stacked list it may have stood on is the pointer's to answer for again.
    signal dismissed()

    anchors.fill: parent

    /// The BRANCH card's `facts` (`RefBranchMenu.standOn`), read here because this is where the models are. A remote
    /// row is held through the local branch of the same name.
    function branchFacts(kind, full, oidHex) {
        if (kind !== "branch" && kind !== "remote")
            return { "heldByWorktree": "", "holderLeaf": "", "remoteCounterpart": "", "remoteDrifted": false,
                     "offers": "", "open": false, "merged": "" }
        const held = kind === "branch"
            ? refRowMenu.worktreesModel.worktreeHolding(full)
            : refRowMenu.worktreesModel.worktreeHolding(refRowMenu.repoTab.localNameFor(full))
        const counterpart = kind === "branch" ? refRowMenu.branchesModel.upstreamOf(full) : ""
        const drifted = kind === "branch" && refRowMenu.branchesModel.upstreamDrifted(full)
        const open = refRowMenu.repoTab.state === "open"
        return {
            "heldByWorktree": held,
            "holderLeaf": held === "" ? "" : GitFacts.pathLeaf(held),
            "remoteCounterpart": counterpart,
            "remoteDrifted": drifted,
            "open": open,
            "merged": !open || kind !== "branch" ? "" : refRowMenu.graphModel.branchDeleteMerged(
                oidHex, refRowMenu.branchesModel.upstreamOidOf(full), refRowMenu.workTree.headOid),
            "offers": GitFacts.refMenuOffers(
                kind, full, oidHex, open, refRowMenu.askBusy,
                refRowMenu.workTree.branch, refRowMenu.workTree.detached,
                refRowMenu.workTree.opText, refRowMenu.workTree.conflictCount,
                held, counterpart, drifted, refRowMenu.repoTab.defaultRemote, "", "")
        }
    }

    /// Deletes the remote branch alone, cut against the configured remote names — a remote's name may contain `/`
    /// (`GitFacts.remoteOfRef`).
    function deleteRemoteNow(remoteRef) {
        const remote = GitFacts.remoteOfRef(remoteRef, refRowMenu.repoTab.remoteNames)
        if (remote === "")
            return
        refRowMenu.repoTab.deleteRemoteBranch(
            remote, GitFacts.branchOfRef(remoteRef, refRowMenu.repoTab.remoteNames))
    }

    /// Both halves at once, cut the same way: one write with one answer, so it is one thing to put back.
    function deleteEverywhereNow(branch, remoteRef, forced) {
        const remote = GitFacts.remoteOfRef(remoteRef, refRowMenu.repoTab.remoteNames)
        if (remote === "")
            return
        refRowMenu.repoTab.deleteBranchEverywhere(
            branch, remote, GitFacts.branchOfRef(remoteRef, refRowMenu.repoTab.remoteNames), forced)
    }

    /// The TAG card's `facts` (`RefTagMenu.standOn`), read here because this is where the models are. Drift and sides
    /// live in the TAGS section alone; the last argument is the `pull` row's upstream, which no tag carries.
    function tagFacts(kind, full, oidHex) {
        const remote = refRowMenu.repoTab.defaultRemote
        const drift = kind === "tag" ? refRowMenu.tagsModel.remoteTagDrift(full, remote) : ""
        const sides = kind === "tag" ? refRowMenu.tagsModel.tagSides(full) : ""
        return {
            "pushRemote": remote,
            "tagDriftOid": drift,
            "tagOnlyThere": sides === "remote",
            "offers": kind !== "tag" ? "" : GitFacts.refMenuOffers(
                kind, full, oidHex,
                refRowMenu.repoTab.state === "open", refRowMenu.askBusy,
                refRowMenu.workTree.branch, refRowMenu.workTree.detached,
                refRowMenu.workTree.opText, refRowMenu.workTree.conflictCount,
                "", "", drift !== "", remote, sides, "")
        }
    }

    /// Opens on that ref, deciding there and then what it offers. Says whether it opened at all: a ref with nothing to
    /// offer — the current branch met as a chip — falls back to the row's own menu.
    function offerOn(kind, name, full, oidHex) {
        refRowMenu.kind = kind
        refRowMenu.refId = full
        refRowMenu.refOid = oidHex
        branchMenu.standOn(kind, name, full, oidHex, refRowMenu.branchFacts(kind, full, oidHex))
        tagMenu.standOn(kind, full, oidHex, refRowMenu.tagFacts(kind, full, oidHex))
        // Whether another working copy has this row's branch out (through the same-named local branch for a remote
        // row): `switch` then opens that copy instead (offers::SwitchAction::OpenHolder).
        const held = kind === "branch" ? refRowMenu.worktreesModel.worktreeHolding(full)
                   : kind === "remote" ? refRowMenu.worktreesModel.worktreeHolding(
                                             refRowMenu.repoTab.localNameFor(full))
                                       : ""
        // Core's rule (offers::ref_menu), asked once so the answers stand while the menu does.
        const offers = GitFacts.refMenuOffers(
            kind, full, oidHex,
            refRowMenu.repoTab.state === "open", refRowMenu.askBusy,
            refRowMenu.workTree.branch, refRowMenu.workTree.detached,
            refRowMenu.workTree.opText, refRowMenu.workTree.conflictCount,
            // No drift: the remote rows are the branch card's (`RefBranchMenu`). The last is the tree's own upstream,
            // which the `pull` row reads.
            held, "", false, refRowMenu.repoTab.defaultRemote, "",
            refRowMenu.workTree.upstream)
        refRowMenu.canSwitch = offers.includes("switch")
        refRowMenu.switchAsks = offers.includes("asks")
        refRowMenu.heldLeaf = held === "" ? "" : GitFacts.pathLeaf(held)
        refRowMenu.canBranchHere = offers.includes("branch-here")
        refRowMenu.canIntegrateFrom = offers.includes("integrate")
        refRowMenu.canPull = offers.includes("pull")
        refRowMenu.pullBlocked = refRowMenu.canPull && refRowMenu.workTree.pullBlocked
        refRowMenu.canDelete = offers.includes("delete")
        // Only where the row that wears it is offered.
        refRowMenu.rebasePublished =
            refRowMenu.integrateOffered
            && refRowMenu.graphModel.rebaseRewritesPublished(oidHex, refRowMenu.workTree.headOid)
        return refMenu.offer()
    }

    // Closes itself (rules-refs/app-ui.md「メニューを閉じるのは自分」).
    AppMenu {
        id: refMenu
        heldReason: refRowMenu.heldReason
        // A refused delete's `-D` is left standing: the card clears it as it opens (`RefBranchMenu.standOn`), and
        // nothing reads it while the card is down.
        onClosed: refRowMenu.dismissed()
        // `Create branch here…` and `switch` are one group: it runs `switch --create` (デザイン規約 §メニュー の入れ子).
        AppMenuItem {
            id: refBranchHereItem
            text: Words.createBranchHere
            offered: refRowMenu.canBranchHere
            onTriggered: refRowMenu.branchHereRequested(refRowMenu.refOid)
        }
        AppMenuItem {
            id: refSwitchItem
            // A branch another copy holds opens that copy instead: the chip gives way to the copy's name behind its
            // tree mark (デザイン規約 §進行中の操作から出る).
            code: refRowMenu.heldLeaf === "" ? "switch" : ""
            //: The row that leads to the working copy holding this branch; the folder's name follows it.
            text: refRowMenu.heldLeaf === "" ? "" : qsTr("Open")
            nameMark: refRowMenu.heldLeaf === "" ? "" : "tree"
            nameMarkTint: Theme.success
            markName: refRowMenu.heldLeaf
            offered: refRowMenu.canSwitch
            // Never greyed: what stands in the move's way presses through to a question, marked by `asks`
            // (デザイン規約 §進行中の操作から出る). Held doors still hold it (`heldReason`) — a move mid-rewrite is
            // the exit nobody meant (§フル interactive rebase).
            blockedReason: ""
            asks: refRowMenu.switchAsks
            // Through the chips' dispatcher: a remote branch whose local one already exists cannot simply be created.
            onTriggered: refRowMenu.switchRequested(refRowMenu.kind, refRowMenu.refId)
        }
        AppMenuSeparator {}
        AppMenuItem {
            code: "merge"
            //: Follows the `merge` chip: "merge into main".
            refSentence: qsTr("into %1")
            refName: refRowMenu.workTree.branch
            offered: refRowMenu.integrateOffered
            onTriggered: refRowMenu.repoTab.merge(refRowMenu.refId, false, false, "")
        }
        AppMenuItem {
            id: refRebaseItem
            code: "rebase"
            //: Follows the `rebase` chip: "rebase main onto it".
            refSentence: qsTr("%1 onto it")
            refName: refRowMenu.workTree.branch
            // A tag stays put under a rebase onto it: git peels it to its commit, and `--update-refs` carries
            // branches only (デザイン規約 §履歴を合流させる).
            offered: refRowMenu.integrateOffered
            // Published = reachable from a remote-tracking ref, as of the last fetch.
            note: refRowMenu.rebasePublished ? Words.rewritesPushed : ""
            onTriggered: refRowMenu.repoTab.rebase(refRowMenu.refId, "", true)
        }
        AppMenuSeparator {}
        // A group of its own above the cards, and the word alone (デザイン規約 §取り込んで合流させる).
        AppMenuItem {
            id: refPullItem
            code: "pull"
            offered: refRowMenu.canPull
            // Diverged: greys and points at the remote branch's own rows (デザイン規約 §取り込んで合流させる).
            blockedReason: refRowMenu.pullBlocked ? Words.pullDiverged : ""
            // git resolves the other end itself.
            onTriggered: refRowMenu.repoTab.pull()
        }
        // A stash's drop stays on this level: it has no card to nest in.
        AppMenuItem {
            id: refStashDropItem
            code: "drop"
            offered: refRowMenu.kind === "stash" && refRowMenu.canDelete
            holdMs: Metrics.holdMs
            holdTone: Theme.danger
            onHeld: {
                refMenu.dismiss()
                refRowMenu.dropStashRequested(refRowMenu.refId)
            }
        }
        AppMenuSeparator {}
        // The same card the graph row's menu carries.
        RefBranchMenu {
            id: branchMenu
            heldReason: refRowMenu.heldReason
            // git's two answers, live: the card turns its own row on them while it stands.
            refusedDelete: refRowMenu.repoTab.branchDeleteRefused
            landedDelete: refRowMenu.repoTab.branchDeleteLanded
            checkedBranch: refRowMenu.repoTab.branchDeleteAsked
            checkedMerged: refRowMenu.repoTab.branchDeleteMerged
            onDeleteRequested: (kind, id, name, oidHex) => refRowMenu.deleteRequested(kind, id, name, oidHex)
            onUpstreamRequested: (branch, counterpart) => refRowMenu.upstreamRequested(branch, counterpart)
            onCheckDeleteRequested: branch => refRowMenu.repoTab.checkBranchDelete(branch)
            onForceDeleteRequested: branch => refRowMenu.repoTab.deleteBranch(branch, true)
            onDeleteRemoteRequested: remoteRef => refRowMenu.deleteRemoteNow(remoteRef)
            onDeleteEverywhereRequested: (branch, remoteRef, forced) =>
                refRowMenu.deleteEverywhereNow(branch, remoteRef, forced)
        }
        AppMenuSeparator {}
        // The same card the graph row's menu carries.
        RefTagMenu {
            id: tagMenu
            heldReason: refRowMenu.heldReason
            canBranchHere: refRowMenu.canBranchHere
            onTagHereRequested: oidHex => refRowMenu.tagHereRequested(oidHex)
            onPushTagRequested: (remote, tag, lease) => refRowMenu.repoTab.pushTag(remote, tag, lease)
            onDeleteTagRequested: tag => refRowMenu.repoTab.deleteTag(tag)
            onDeleteRemoteTagRequested: (remote, tag, onlyThere) =>
                refRowMenu.repoTab.deleteRemoteTag(remote, tag, onlyThere)
            onDeleteTagEverywhereRequested: (tag, remote) =>
                refRowMenu.repoTab.deleteTagEverywhere(tag, remote)
        }
    }
}
