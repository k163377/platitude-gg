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
    /// Where the `rebase` row's note comes from: the drawn rows carry the walk's own marks, and the range this menu
    /// would rewrite is a question about them (`rebasePublished`).
    required property GraphModel graphModel
    /// Handed to the branch card, which reads the remote a branch speaks for and the working copy that may be
    /// holding it out of them (`RefBranchMenu`). `switch` asks the second one here as well — it is the only row on
    /// this level that the answer changes.
    required property NavSectionModel branchesModel
    required property NavSectionModel worktreesModel
    /// Handed to the tag card, which reads the sides a name stands on and where a remote last had it out of this
    /// section alone (`RefTagMenu`).
    required property NavSectionModel tagsModel

    /// Why every row here is out, in one line, while the window's write doors are held — a write that replays is
    /// running behind the screen (`RepoPage.doorsHeldWhy`). Handed down: the left pane holds the same doors off the
    /// same answer, and two readings of "is a rewrite under way" would be two answers. **The rows grey** — see
    /// `AppMenu.heldReason`.
    property string heldReason: ""
    /// What the offers are asked with while that is standing: **nothing running**. The hold already answers for every
    /// row here, and the busy count would answer a second time by taking rows away — leaving a menu that is a
    /// different shape on each side of a lock the reader is waiting to see come off. A row that is only ever *out* is
    /// read as one the menu does not have; a row that greys and says why is read as the wait it is
    /// (デザイン規約 §メニュー の例外: 「今できない」行は無効で残る).
    readonly property int askBusy: refRowMenu.heldReason !== "" ? 0 : refRowMenu.repoTab.busyCount

    /// The row the menu stands on. `refId` is what git knows it by, which on a stash is a selector, apart from
    /// the row's message.
    property string kind: ""
    property string refId: ""
    property string refOid: ""

    /// What this menu offers, decided as it opens (see the note above).
    property bool canSwitch: false
    /// Whether that row will raise a question — an operation standing, or files still waiting on a decision. Worn as
    /// the `!` in the row's mark seat, so the press is read for what it is before it is made
    /// (デザイン規約 §進行中の操作から出る).
    property bool switchAsks: false
    /// The folder of the other working copy holding this row's branch, empty when none does — and the whole of what
    /// makes that row lead somewhere else: the press stands the tab in that copy, so the row names it
    /// (`RepoPage.switchToRef`, offers::SwitchAction::OpenHolder). The leaf, because that is what every other place
    /// a copy is named calls it — the WORKTREES rows, the tab, the graph's marker (デザイン規約 §ref の種別).
    property string heldLeaf: ""
    property bool canBranchHere: false
    property bool canIntegrateFrom: false
    /// Whether this row has a far side a `git pull` would go to: the branch the working tree is on, or the upstream
    /// it is measured against — the two ends of one comparison (offers::ref_menu).
    property bool canPull: false
    /// And whether the sides have grown apart, which is what greys the row: a divergence is brought together by
    /// choosing, and the rows that choose are the remote branch's own (`WorkTreeModel.pullBlocked`). Frozen as the
    /// menu opens with everything else it shows.
    property bool pullBlocked: false
    /// The stash's own drop — the only delete this level answers for. What a branch's name and a tag's name offer is
    /// their own cards' question, worked out inside them (`RefBranchMenu` / `RefTagMenu`).
    property bool canDelete: false

    // ---- bringing two lines of history together --------------------
    /// Whether the merge / rebase pair is on offer. **One reading for the two rows and for the note one of them
    /// wears** — three spellings of the same condition is the one that drifts, and the note has to appear exactly
    /// where the row it hangs on does. A stash is the kind left out: it is not a line of history to bring in.
    readonly property bool integrateOffered:
        (refRowMenu.kind === "branch" || refRowMenu.kind === "remote" || refRowMenu.kind === "tag")
        && refRowMenu.canIntegrateFrom
    /// Whether a rebase onto this row would rewrite a commit a remote already has — the commits at stake are
    /// `<ref>..HEAD`, and it is the graph that answers for them (`GraphModel.rebaseRewritesPublished`).
    ///
    /// **Read off the rows, in hand as the menu opens.** A range is not one commit, so the row's own mark cannot say
    /// it — but the drawn rows are the walk's order and carry the walk's marks, which is all the question needs.
    /// A `git rev-list` would land after the card was on screen and grow this very row, taking the card's right edge
    /// and the `▸` on it out from under the hand (規約 §行が読む答えはどこから来るか).
    property bool rebasePublished: false

    /// Whether the card is on screen, and whether it has finished opening: the folded list asks the first, the chip's
    /// stacked list asks the second. Plain properties — `visible` read from another file comes back stale
    /// (`RefusalBadge`).
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
    readonly property alias pullItem: refPullItem
    readonly property alias rebaseItem: refRebaseItem
    /// The two cards the rows above hang behind — a run that photographs one of those rows has to open its card
    /// first (`AppMenu.openSub`).
    readonly property alias branchCard: branchMenu
    readonly property alias tagCard: tagMenu

    /// What the page answers for: moving the working tree, the delete git may still refuse, and the stash drop that two
    /// menus share.
    signal switchRequested(string kind, string name)
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
    /// The menu went away — with it goes a refused delete's offer, and the stacked list it may have been standing on is
    /// the pointer's to answer for again.
    signal dismissed()

    anchors.fill: parent

    /// What the BRANCH card stands on: the readings its rows are told apart by, and core's answer over them
    /// (`RefBranchMenu.standOn`). **Read here because this is where the models are** — a remote row lands on the
    /// local branch of the same name, so it is that one another copy can be holding, and the reading a local branch
    /// speaks for is the branches' section's. The `merged` the drawn rows can answer is the graph's
    /// (規約 §行が読む答えはどこから来るか); empty is a tip or a reference older than the window, which only git
    /// answers for.
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

    /// The reading over there gone without touching the local branch. The configured names say where the cut is (a
    /// remote's own name may contain `/`); an unconfigured remote still cuts at the first slash, so the press acts
    /// and git answers (`GitFacts.remoteOfRef`).
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

    /// What the TAG card stands on: the readings its rows are told apart by, and core's answer over them
    /// (`RefTagMenu.standOn`). **Read here because this is where the models are** — the card is handed values and
    /// hands requests back, so a menu carrying it is the one boundary the typed models cross.
    ///
    /// `remoteTagDrift` and `tagSides` live in the TAGS section alone. The drifted reading is what takes the two
    /// rows that reach the remote out — the same answer that shapes the push row, read the other way
    /// (デザイン規約 §左メニューの所作 の削除の表) — and the upstream the last argument names belongs to the `pull`
    /// row, which no tag carries.
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
        // Both cards are their own question, and each works its answers out for itself — the very cards the graph
        // row's menu carries, asked the same way (RefBranchMenu / RefTagMenu).
        branchMenu.standOn(kind, name, full, oidHex, refRowMenu.branchFacts(kind, full, oidHex))
        tagMenu.standOn(kind, full, oidHex, refRowMenu.tagFacts(kind, full, oidHex))
        // Whether another working copy has this row's branch out. **git refuses `switch` for one** (measured), and
        // the row wears the `!` for it — this level's one use of the answer; the delete rows read their own copy
        // inside the card. A remote row lands on the local branch of the same name, so it is that one another copy
        // can be holding.
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
            // No reading to have drifted: the rows that reach a remote are the branch card's, and it asks for itself
            // (`RefBranchMenu`). The last one is the working tree's own upstream, which is what the `pull` row reads
            // — the row it stands on is one of that comparison's two ends (offers::ref_menu).
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
        // Asked only where the row that wears it is offered, and asked of the graph — the answer is already there,
        // and the card is about to be measured with it (see `rebasePublished`).
        refRowMenu.rebasePublished =
            refRowMenu.integrateOffered
            && refRowMenu.graphModel.rebaseRewritesPublished(oidHex, refRowMenu.workTree.headOid)
        return refMenu.offer()
    }

    // The menu closes itself: a row that runs something takes it down with `dismiss()`, and the branch card
    // standing for a delete's answer goes by itself when that answer lands (`RefBranchMenu`).
    AppMenu {
        id: refMenu
        heldReason: refRowMenu.heldReason
        // A refused delete's offer is left standing: the card puts it back as it opens (`RefBranchMenu.offerOn`),
        // and nothing reads it while the card is down.
        onClosed: refRowMenu.dismissed()
        // **The card holds what moves the reader; the rest is behind a mark** (デザイン規約 §メニュー の入れ子). Every row on
        // this level answers the question the reader came with — where am I, and where do I go from here — and the
        // cards at the foot hold what is *done to* a ref.
        //
        // A branch of one's own, started where this row stands, is one of those moves: it is where the reader
        // carries on from. **Beside `switch`, in one group**: what it runs is `switch --create`, so it is that
        // row's other form — the move to a branch that does not exist yet
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
            // **A branch another working copy holds is not a move at all** — git keeps a branch to one copy, so the
            // press goes to that copy instead (offers::SwitchAction::OpenHolder). The row says so: the command it
            // cannot run gives its chip up, and what takes its place is where the press lands — the copy's own name,
            // behind the mark it is read by everywhere else (デザイン規約 §進行中の操作から出る).
            code: refRowMenu.heldLeaf === "" ? "switch" : ""
            //: The row that leads to the working copy holding this branch; the folder's name follows it.
            text: refRowMenu.heldLeaf === "" ? "" : qsTr("Open")
            nameMark: refRowMenu.heldLeaf === "" ? "" : "tree"
            nameMarkTint: Theme.success
            markName: refRowMenu.heldLeaf
            offered: refRowMenu.canSwitch
            // **What stands in the move's way presses through to a question**: an operation standing and a tree with
            // unmerged files both do, and the mark is what says so before the press
            // (デザイン規約 §進行中の操作から出る). Greying is the delete rows' answer — a row that cannot be
            // pressed says why only on hover, and the reader who reached for it is the one who needs to read it.
            //
            // **The doors being held is the other thing**, and this row is held with the rest of them (`heldReason`):
            // there is no question to raise there, the answer being to wait — and a move let go into the middle of a
            // rewrite is the exit nobody meant (§フル interactive rebase).
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
            // A tag is on offer here for the reason it is on merge: it is a fixed point either way, and the one this
            // row lands on stays where it is — git peels an annotated tag to its commit, and `--update-refs` carries
            // branches only (measured).
            offered: refRowMenu.integrateOffered
            // Said (要望: rewriting a pushed commit shows a warning). Published = reachable from a
            // remote-tracking ref, only as fresh as the last fetch.
            note: refRowMenu.rebasePublished ? Words.rewritesPushed : ""
            onTriggered: refRowMenu.repoTab.rebase(refRowMenu.refId, "", true)
        }
        AppMenuSeparator {}
        // **A group of its own, right above the cards.** The rows above bring *this row* in;
        // a pull brings in the branch the working tree is on, whichever of the comparison's two ends the menu was
        // opened from — a different subject, and the one seat it can keep in every menu that carries it. The graph
        // row's menu offers more above it and none of that moves this row (デザイン規約 §取り込んで合流させる).
        //
        // **The word is the whole row.** Both rows it stands on are ends of the same comparison — the branch the tree
        // is on and the upstream it is measured against — so the same `git pull` runs from either, and a sentence
        // naming the other end would be the row saying what the row it was opened on already is (§メニュー).
        AppMenuItem {
            id: refPullItem
            code: "pull"
            offered: refRowMenu.canPull
            // Greyed where both sides have moved: bringing those together is a choice, and the rows that make it are
            // the remote branch's own — so this row points at them instead of running
            // (規約 §メニュー の例外「今できない」行は無効で残す).
            blockedReason: refRowMenu.pullBlocked ? Words.pullDiverged : ""
            // Nothing to hand over: whichever end of the comparison this row is, git resolves the other.
            onTriggered: refRowMenu.repoTab.pull()
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
                refMenu.dismiss()
                refRowMenu.dropStashRequested(refRowMenu.refId)
            }
        }
        AppMenuSeparator {}
        // The deletes a branch's name answers for — its own file, because the graph row's menu carries the very same
        // card (RefBranchMenu).
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
        // Everything a tag's name answers for, behind its own mark — the very card the graph row's menu carries
        // (RefTagMenu).
        RefTagMenu {
            id: tagMenu
            heldReason: refRowMenu.heldReason
            canBranchHere: refRowMenu.canBranchHere
            onTagHereRequested: oidHex => refRowMenu.tagHereRequested(oidHex)
            // The card says what was pressed; the tab this menu holds is what runs it.
            onPushTagRequested: (remote, tag, lease) => refRowMenu.repoTab.pushTag(remote, tag, lease)
            onDeleteTagRequested: tag => refRowMenu.repoTab.deleteTag(tag)
            onDeleteRemoteTagRequested: (remote, tag, onlyThere) =>
                refRowMenu.repoTab.deleteRemoteTag(remote, tag, onlyThere)
            onDeleteTagEverywhereRequested: (tag, remote) =>
                refRowMenu.repoTab.deleteTagEverywhere(tag, remote)
        }
    }
}
