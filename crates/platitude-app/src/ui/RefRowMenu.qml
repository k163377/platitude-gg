pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

// The right-click on a ref wherever it is drawn — sidebar row, graph chip, a row of a chip's stacked list — so a
// branch offers the same things everywhere. Decided as it opens and frozen while it stands, but for standing again
// when a command it waited for has ended (`standAgain`, デザイン規約 §メニュー).
//
// An `Item` because a menu measures the window through the item it was declared under (`AppMenu.ownerItem`).
Item {
    id: refRowMenu

    required property RepoTab repoTab
    required property WorktreeModel worktree
    /// The worktree as it stood when the command the menus wait for was asked (`RepoPage.treeAtAsk`); null while
    /// nothing is out.
    property var treeAtAsk: null
    /// What the rows are read off, set by the door that stands them: the worktree — or, for a menu raised under a
    /// command, `treeAtAsk`.
    property var tree: refRowMenu.worktree
    /// Answers the `rebase` row's note off the drawn rows (`rebasePublished`).
    required property GraphModel graphModel
    /// Both read for the branch card (`branchFacts`); `switch` also asks `worktreesModel` who holds the branch.
    required property NavSectionModel branchesModel
    required property NavSectionModel worktreesModel
    /// Read for the tag card (`tagFacts`).
    required property NavSectionModel tagsModel
    /// Where a remote branch points now and which selector a stash answers to now, for the menu standing again
    /// (`standAgain`) — the stash off the listing, which holds every entry the graph's walk may not reach.
    required property NavSectionModel remotesModel
    required property NavSectionModel stashesModel

    /// Why every row is out while the window's write doors are held (`RepoPage.doorsHeldWhy`), handed down so this
    /// and the left pane read one answer. The rows grey (`AppMenu.heldReason`).
    property string heldReason: ""
    /// What the rows say while they wait for a git command this window started (`RepoPage.runningWhy`). No running
    /// command is asked about: a menu opened under one keeps its rows and greys them until it ends
    /// (`AppMenu.runningWhy`), so it has one shape whenever it is opened.
    property string runningWhy: ""

    /// The row the menu stands on. `refId` is what git knows it by, which on a stash is a selector, apart from
    /// the row's message, and on a worktree with no branch out its path.
    property string kind: ""
    property string refId: ""
    property string refOid: ""
    /// What else it was opened on — the name the row shows, the worktree the WORKTREE card stands on, a remote named on
    /// its own (`offerOn`) — for the menu standing again on a newer reading (`standAgain`).
    property string refName: ""
    property var openedFor: undefined
    property string openedAim: ""
    /// What merge / rebase are handed: the name, or the commit where the row is a worktree — a path is no ref.
    readonly property string integrateRef: refRowMenu.kind === "worktree" ? refRowMenu.refOid : refRowMenu.refId

    property bool canSwitch: false
    /// Whether `switch` will raise a question (an operation standing, or unmerged files), worn as the row's `!`
    /// (デザイン規約 §進行中の操作から出る).
    property bool switchAsks: false
    /// The name the worktree holding this row's branch goes by, empty when none does; the switch row then opens that
    /// worktree and names it (`RepoPage.switchToRef`). Its WORKTREES row's name — the folder, or the branch the
    /// repository's own has out (`NavSectionModel.worktreeCalled`, デザイン規約 §ref の種別).
    property string heldLeaf: ""
    /// That worktree is the repository's own: its name wears the house, not the tree (デザイン規約 §ref の種別).
    property bool heldHome: false
    property bool canBranchHere: false
    property bool canIntegrateFrom: false
    /// What the `pull` row runs (`offers::pull_row`): `here` — `git pull`, on either end of the current branch's
    /// upstream comparison —, `aside` — this row's own branch fast-forwarded, the worktree being on another —, or ""
    /// for no row.
    property string pullRuns: ""
    /// Why that row is greyed — `diverged` / `own` / `held` (`Words.pullOut`) — or "" while it is live.
    property string pullOut: ""
    /// The remote branch this row's branch tracks, which the `own` reason names.
    property string pullUpstream: ""
    /// The stash's drop — the only delete on this level; branch and tag deletes are their cards' (`RefBranchMenu` /
    /// `RefTagMenu`).
    property bool canDelete: false

    // ---- bringing two lines of history together --------------------
    /// Whether merge / rebase are on offer — one reading for both rows and the rebase note, so the note appears
    /// exactly where its row does. A stash is not a line of history.
    readonly property bool integrateOffered:
        (refRowMenu.kind === "branch" || refRowMenu.kind === "remote" || refRowMenu.kind === "tag"
         || refRowMenu.kind === "worktree")
        && refRowMenu.canIntegrateFrom
    /// Whether a rebase onto this row would rewrite a commit a remote has (`<ref>..HEAD`), read off the drawn rows
    /// (`GraphModel.rebaseRewritesPublished`): a `git rev-list` would land after the card and widen it under the hand
    /// (規約 §行が読む答えはどこから来るか).
    property bool rebasePublished: false

    /// On screen, and finished opening — asked by the folded list and the chip's stacked list. Plain properties:
    /// `visible` read from another file comes back stale (`RefusalBadge`).
    readonly property bool showing: refMenu.visible
    readonly property bool opened: refMenu.opened
    /// Every level of this menu: the menu and its three cards.
    readonly property var levels: [refMenu, branchMenu, worktreeMenu, tagMenu]

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
    readonly property alias pushBranchItem: branchMenu.pushItem
    readonly property alias removeWorktreeItem: worktreeMenu.removeWorktreeItem
    readonly property alias stashDropItem: refStashDropItem
    readonly property alias switchItem: refSwitchItem
    readonly property alias pullItem: refPullItem
    readonly property alias rebaseItem: refRebaseItem
    /// The cards those rows hang behind; a run photographing one opens it first (`AppMenu.openSub`).
    readonly property alias branchCard: branchMenu
    readonly property alias worktreeCard: worktreeMenu
    readonly property alias tagCard: tagMenu

    /// What the page answers for: moving the worktree, the delete git may still refuse, and the stash drop that two
    /// menus share.
    signal switchRequested(string kind, string name)
    /// A new branch on this row's commit; the page opens the name box where the menu was opened.
    signal branchHereRequested(string oidHex)
    /// A tag on this row's commit — the same box, opened where the menu was.
    signal tagHereRequested(string oidHex)
    /// The WORKTREE card's two making rows (`RefWorktreeMenu`): a new branch on this row's commit in a worktree of
    /// its own — the same box again, opened where the menu was — and the row's own branch out in one.
    signal worktreeHereRequested(string oidHex)
    signal worktreeAddRequested(string mode, string branch, string start, string path, string name)
    signal deleteRequested(string kind, string id, string name, string oidHex)
    signal dropStashRequested(string selector)
    /// The WORKTREE card's `worktree remove`, which takes the worktree's row off the screen at the press (`RepoTab`).
    signal removeWorktreeRequested(string path, string name, string holds)
    /// Which remote branch a local one is measured against — the branch card's row, answered in the page's one bar
    /// (`UpstreamFlow`).
    signal upstreamRequested(string branch, string counterpart)
    /// Where a branch's first push goes — the branch card's push row, asked in the page's one bar (`PublishFlow`).
    signal publishRequested(string branch, string upstream)

    anchors.fill: parent

    /// The BRANCH card's `facts` (`RefBranchMenu.standOn`), read here because this is where the models are. A remote
    /// row is held through the local branch of the same name.
    function branchFacts(kind, full, oidHex) {
        if (kind !== "branch" && kind !== "remote")
            return { "heldByWorktree": "", "holderLeaf": "", "holderHome": false,
                     "remoteCounterpart": "", "remoteCounterpartOid": "",
                     "remoteDrifted": false, "offers": "", "open": false, "merged": "" }
        const held = kind === "branch"
            ? refRowMenu.worktreesModel.worktreeHolding(full)
            : refRowMenu.worktreesModel.worktreeHolding(refRowMenu.repoTab.localNameFor(full))
        const counterpart = kind === "branch" ? refRowMenu.branchesModel.upstreamOf(full) : ""
        const drifted = kind === "branch" && refRowMenu.branchesModel.upstreamDrifted(full)
        const open = refRowMenu.repoTab.state === "open"
        return {
            "heldByWorktree": held,
            "holderLeaf": held === "" ? "" : refRowMenu.worktreesModel.worktreeCalled(held),
            "holderHome": held !== "" && refRowMenu.worktreesModel.worktreeHome(held),
            "remoteCounterpart": counterpart,
            "remoteCounterpartOid": counterpart === "" ? "" : refRowMenu.branchesModel.upstreamOidOf(full),
            "remoteDrifted": drifted,
            "open": open,
            "push": kind === "branch"
                    ? refRowMenu.branchesModel.branchPush(full, refRowMenu.repoTab.defaultRemote) : undefined,
            "merged": !open || kind !== "branch" ? "" : refRowMenu.graphModel.branchDeleteMerged(
                oidHex, refRowMenu.branchesModel.upstreamOidOf(full), refRowMenu.tree.headOid),
            "offers": GitFacts.refMenuOffers(
                kind, full, oidHex, open,
                refRowMenu.tree.branch, refRowMenu.tree.detached,
                refRowMenu.tree.opText, refRowMenu.tree.conflictCount,
                held, counterpart, drifted, refRowMenu.repoTab.defaultRemote, "")
        }
    }

    /// Deletes the remote branch alone, cut against the configured remote names — a remote's name may contain `/`
    /// (`GitFacts.remoteOfRef`). `expect` is the commit the card showed it on, the delete's lease.
    function deleteRemoteNow(remoteRef, expect) {
        const remote = GitFacts.remoteOfRef(remoteRef, refRowMenu.repoTab.remoteNames)
        if (remote === "")
            return
        refRowMenu.repoTab.deleteRemoteBranch(
            remote, GitFacts.branchOfRef(remoteRef, refRowMenu.repoTab.remoteNames), expect)
    }

    /// Both halves at once, cut the same way: one write with one answer, so it is one thing to put back.
    function deleteEverywhereNow(branch, remoteRef, forced, expect) {
        const remote = GitFacts.remoteOfRef(remoteRef, refRowMenu.repoTab.remoteNames)
        if (remote === "")
            return
        refRowMenu.repoTab.deleteBranchEverywhere(
            branch, remote, GitFacts.branchOfRef(remoteRef, refRowMenu.repoTab.remoteNames), forced, expect)
    }

    /// The TAG card's `facts` (`RefTagMenu.standOn`), read here because this is where the models are. What the card
    /// reaches and why its rows are held back live in the TAGS section alone (`NavSectionModel.tagMenu`); `aim` is a
    /// remote the reader named on its own.
    function tagFacts(kind, full, oidHex, aim) {
        if (kind !== "tag")
            return { "pushRemote": refRowMenu.repoTab.defaultRemote, "offers": "" }
        const menu = refRowMenu.tagsModel.tagMenu(full, refRowMenu.repoTab.defaultRemote, aim)
        const sides = refRowMenu.tagsModel.tagSides(full)
        return {
            "pushRemote": menu.pushRemote,
            "tagDriftOid": menu.lease,
            "tagReach": menu.reach,
            "tagHeldBack": menu.heldBack,
            "tagCarriers": menu.carriers,
            "tagOnlyThere": menu.rowGoes,
            "tagHere": sides === "here" || sides === "both",
            // Held back reads as drift to core: the rows reaching over there stay out (offers::ref_menu).
            "offers": GitFacts.refMenuOffers(
                kind, full, oidHex,
                refRowMenu.repoTab.state === "open",
                refRowMenu.tree.branch, refRowMenu.tree.detached,
                refRowMenu.tree.opText, refRowMenu.tree.conflictCount,
                "", "", menu.heldBack !== "", menu.pushRemote, sides)
        }
    }

    /// What the WORKTREE card's two making rows stand on (`RefWorktreeMenu.standOn`'s `making`), asked of the row
    /// (`offers::worktree_rows`) — `held` the worktree holding its branch, as `offerOn` found it. A free branch, or a
    /// remote one with no local branch, goes out as it is to the folder `newWorktreeFor` names; the folder is looked at
    /// here, as the menu opens. The same answer `CommitMenuState.askWorktreeRows` gives the graph row's card.
    function askWorktreeRows(kind, full, oidHex, held) {
        const local = kind === "remote" ? refRowMenu.repoTab.localNameFor(full) : full
        const offers = GitFacts.worktreeOffers(
            kind, full, oidHex, refRowMenu.repoTab.state === "open",
            refRowMenu.tree.branch, kind === "branch" || kind === "remote" ? held : "",
            kind === "remote" && refRowMenu.branchesModel.oidOfName(local) !== "")
        const checkout = offers.includes("checkout-branch") ? "branch"
                       : offers.includes("checkout-track") ? "track" : ""
        const place = checkout === "" ? undefined : refRowMenu.worktreesModel.newWorktreeFor(local)
        return {
            "oid": oidHex,
            // Only where worktrees have a place to go (`CommitMenuState.askWorktreeRows`).
            "here": offers.includes("here") && refRowMenu.worktreesModel.worktreesPlaced(),
            "checkout": place === undefined ? "" : checkout,
            "branch": place === undefined ? "" : local,
            "start": checkout === "track" ? full : "",
            "path": place === undefined ? "" : place.path,
            "place": place === undefined ? "" : place.name,
            "taken": place === undefined ? "" : place.taken
        }
    }

    /// Opens on that ref, deciding there and then what it offers. Says whether it opened at all: a ref with nothing to
    /// offer — the current branch met as a chip — falls back to the row's own menu.
    ///
    /// `worktreePath` is the worktree the WORKTREE card stands on — the WORKTREES row's own; left out, the worktree
    /// holding this row's branch, if another does. `aim` is a remote the reader named on its own
    /// (`RepoPage.openRefMenu`), which the TAG card acts on.
    function offerOn(kind, name, full, oidHex, worktreePath, aim) {
        // A worktree with a branch out is that branch's menu, carrying the worktree's card: the worktree is where
        // the branch is, and the row says both (デザイン規約 §左メニューの所作).
        if (kind === "worktree") {
            const worktree = refRowMenu.worktreesModel.worktreeFacts(full)
            if (worktree !== undefined && worktree.branch !== "")
                return refRowMenu.offerOn("branch", worktree.branch, worktree.branch, oidHex, full)
        }
        for (let i = 0; i < refRowMenu.levels.length; i++)
            refRowMenu.levels[i].releaseShape()
        refRowMenu.tree = refRowMenu.treeAtAsk !== null ? refRowMenu.treeAtAsk : refRowMenu.worktree
        refRowMenu.standOn(kind, name, full, oidHex, worktreePath, aim)
        return refMenu.offer()
    }

    /// The command the menu waited for has ended with the menu up: what it left is read again and the rows stand on
    /// it, the menu keeping its shape (`AppMenu.holdShape`) — on the commit the ref points at now, a stash's row on the
    /// selector it now answers to. A ref the command took away leaves the menu nothing to act on.
    function standAgain() {
        if (!refRowMenu.showing)
            return
        // A stash answers to its place in the list, which the command may have moved; the rest to their names.
        const full = refRowMenu.kind === "stash" ? refRowMenu.stashesModel.fullOfOid(refRowMenu.refOid)
                                                 : refRowMenu.refId
        const oidHex = full === "" ? "" : refRowMenu.oidNow(refRowMenu.kind, full)
        if (oidHex === "") {
            refMenu.dismiss()
            return
        }
        for (let i = 0; i < refRowMenu.levels.length; i++)
            refRowMenu.levels[i].holdShape()
        refRowMenu.tree = refRowMenu.worktree
        refRowMenu.standOn(refRowMenu.kind, refRowMenu.refName, full, oidHex, refRowMenu.openedFor,
                           refRowMenu.openedAim)
    }
    /// The commit the ref `full` of `kind` points at now; empty where it is gone. A stash's commit does not move. A
    /// remote branch that moved counts as gone: its `push --delete` is leased to where it stood as the menu went up,
    /// and stood again on the new tip it would take commits the menu never showed.
    function oidNow(kind, full) {
        if (kind === "branch")
            return refRowMenu.branchesModel.oidOfName(full)
        if (kind === "remote")
            return refRowMenu.remotesModel.oidOfName(full) === refRowMenu.refOid ? refRowMenu.refOid : ""
        if (kind === "tag")
            return refRowMenu.tagsModel.oidOfName(full)
        if (kind === "worktree") {
            const worktree = refRowMenu.worktreesModel.worktreeFacts(full)
            return worktree === undefined ? "" : worktree.head
        }
        return refRowMenu.refOid
    }

    /// Reads and freezes everything the rows stand on: `offerOn` without raising the menu.
    function standOn(kind, name, full, oidHex, worktreePath, aim) {
        refRowMenu.kind = kind
        refRowMenu.refName = name
        refRowMenu.refId = full
        refRowMenu.refOid = oidHex
        refRowMenu.openedFor = worktreePath
        refRowMenu.openedAim = aim === undefined ? "" : aim
        branchMenu.standOn(kind, name, full, oidHex, refRowMenu.branchFacts(kind, full, oidHex))
        tagMenu.standOn(kind, full, oidHex, refRowMenu.tagFacts(kind, full, oidHex, aim === undefined ? "" : aim))
        // Whether another worktree has this row's branch out (through the same-named local branch for a remote
        // row): `switch` then opens that worktree instead (offers::SwitchAction::OpenHolder). A worktree with no
        // branch leads to itself, unless this tab already stands in it.
        const held = kind === "branch" ? refRowMenu.worktreesModel.worktreeHolding(full)
                   : kind === "remote" ? refRowMenu.worktreesModel.worktreeHolding(
                                             refRowMenu.repoTab.localNameFor(full))
                   : kind === "worktree" && !GitFacts.samePath(full, refRowMenu.repoTab.repoPath) ? full
                                       : ""
        // The card's `worktree remove` stands on a worktree the row itself names or whose branch it is: a remote row
        // leads to the holder of the same-named local branch, but the worktree is not what it names
        // (デザイン規約 §メニュー の入れ子).
        const worktreeAt = worktreePath !== undefined ? worktreePath : kind === "remote" ? "" : held
        worktreeMenu.standOn(worktreeAt === "" ? undefined : refRowMenu.worktreesModel.worktreeFacts(worktreeAt),
                         GitFacts.samePath(worktreeAt, refRowMenu.repoTab.repoPath),
                         refRowMenu.askWorktreeRows(kind, full, oidHex, held))
        // Core's rule (offers::ref_menu), asked once so the answers stand while the menu does.
        const offers = GitFacts.refMenuOffers(
            kind, full, oidHex,
            refRowMenu.repoTab.state === "open",
            refRowMenu.tree.branch, refRowMenu.tree.detached,
            refRowMenu.tree.opText, refRowMenu.tree.conflictCount,
            // No drift: the remote rows are the branch card's (`RefBranchMenu`).
            held, "", false, refRowMenu.repoTab.defaultRemote, "")
        refRowMenu.canSwitch = offers.includes("switch")
        refRowMenu.switchAsks = offers.includes("asks")
        // As its WORKTREES row names it (`NavSectionModel.worktreeCalled`).
        refRowMenu.heldLeaf = held === "" ? "" : refRowMenu.worktreesModel.worktreeCalled(held)
        refRowMenu.heldHome = held !== "" && refRowMenu.worktreesModel.worktreeHome(held)
        refRowMenu.canBranchHere = offers.includes("branch-here")
        refRowMenu.canIntegrateFrom = offers.includes("integrate")
        // The tree's own comparison for `git pull`, the row's branch's for its fast-forward (offers::pull_row).
        refRowMenu.pullUpstream = refRowMenu.branchesModel.upstreamOf(full)
        const pull = GitFacts.pullRow(
            kind, full, refRowMenu.repoTab.state === "open",
            refRowMenu.tree.branch, refRowMenu.tree.detached, refRowMenu.tree.opText,
            refRowMenu.tree.upstream, refRowMenu.tree.pullBlocked,
            held, refRowMenu.pullUpstream, refRowMenu.branchesModel.aheadOf(full))
        refRowMenu.pullRuns = pull.length > 0 ? pull[0] : ""
        refRowMenu.pullOut = pull.length > 1 ? pull[1].replace("out-", "") : ""
        refRowMenu.canDelete = offers.includes("delete")
        // Only where the row that wears it is offered.
        refRowMenu.rebasePublished =
            refRowMenu.integrateOffered
            && refRowMenu.graphModel.rebaseRewritesPublished(oidHex, refRowMenu.tree.headOid)
    }

    // Closes itself (rules-refs/app-ui.md「メニューを閉じるのは自分」).
    AppMenu {
        id: refMenu
        heldReason: refRowMenu.heldReason
        runningWhy: refRowMenu.runningWhy
        onWaitEnded: refRowMenu.standAgain()
        // Nothing to clear at the close: a refused delete's `-D` is left standing, the card clears it as it opens
        // (`RefBranchMenu.standOn`), and nothing reads it while the card is down.

        // `Create branch here…` and `switch` are one group: it runs `switch --create` (デザイン規約 §メニュー の入れ子).
        AppMenuItem {
            id: refBranchHereItem
            text: Words.createBranchHere
            offered: refRowMenu.canBranchHere
            onTriggered: refRowMenu.branchHereRequested(refRowMenu.refOid)
        }
        AppMenuItem {
            id: refSwitchItem
            // A branch another worktree holds opens that worktree instead: the chip gives way to the worktree's name
            // behind its tree mark (デザイン規約 §進行中の操作から出る).
            code: refRowMenu.heldLeaf === "" ? "switch" : ""
            //: The row that leads to the worktree holding this branch; its name follows it — the folder's, or the branch
            //: the repository's own worktree has out.
            text: refRowMenu.heldLeaf === "" ? "" : qsTr("Open")
            nameMark: refRowMenu.heldLeaf === "" ? "" : refRowMenu.heldHome ? "home" : "tree"
            nameMarkTint: Theme.success
            markName: refRowMenu.heldLeaf
            offered: refRowMenu.canSwitch
            // Never greyed for its own reason: what stands in the move's way presses through to a question, marked by
            // `asks` (デザイン規約 §進行中の操作から出る). Held doors still hold it (`heldReason`) — a move mid-rewrite
            // is the exit nobody meant (§フル interactive rebase) — and so does a command the menu waits for
            // (`runningWhy`).
            blockedReason: ""
            asks: refRowMenu.switchAsks
            // Through the chips' dispatcher: a remote branch whose local one already exists cannot simply be created.
            onTriggered: refRowMenu.switchRequested(refRowMenu.kind, refRowMenu.refId)
        }
        AppMenuSeparator {}
        AppMenuItem {
            code: "merge"
            refSentence: Words.intoBranch
            refName: refRowMenu.worktree.branch
            offered: refRowMenu.integrateOffered
            onTriggered: refRowMenu.repoTab.merge(refRowMenu.integrateRef, false, false, "")
        }
        AppMenuItem {
            id: refRebaseItem
            code: "rebase"
            //: Follows the `rebase` chip: "rebase main onto it".
            refSentence: qsTr("%1 onto it")
            refName: refRowMenu.worktree.branch
            // A tag stays put under a rebase onto it: git peels it to its commit, and `--update-refs` carries
            // branches only (デザイン規約 §履歴を合流させる).
            offered: refRowMenu.integrateOffered
            // Published = reachable from a remote-tracking ref, as of the last fetch.
            note: refRowMenu.rebasePublished ? Words.rewritesPushed : ""
            onTriggered: refRowMenu.repoTab.rebase(refRowMenu.integrateRef, "", true)
        }
        AppMenuSeparator {}
        // A group of its own above the cards, naming the branch that moves in its own colour — the worktree's, which
        // the row may not name (its upstream's), or this row's own (デザイン規約 §取り込んで合流させる).
        AppMenuItem {
            id: refPullItem
            code: "pull"
            refSentence: Words.intoBranch
            refName: refRowMenu.pullRuns === "aside" ? refRowMenu.refId : refRowMenu.worktree.branch
            refTint: refRowMenu.pullRuns === "aside" ? Theme.textPrimary : Theme.textLink
            offered: refRowMenu.pullRuns !== ""
            // Greyed where git would turn the press down, saying where to go instead (デザイン規約 §取り込んで合流させる).
            blockedReason: Words.pullOut(refRowMenu.pullOut, refRowMenu.heldLeaf, refRowMenu.pullUpstream)
            reasonMarkWord: refRowMenu.pullOut === "held" ? refRowMenu.heldLeaf : ""
            reasonMarkKind: refRowMenu.heldHome ? "home" : "tree"
            // `git pull` resolves the other end itself; the fast-forward is told the branch.
            onTriggered: refRowMenu.repoTab.pull(refRowMenu.pullRuns === "aside" ? refRowMenu.refId : "")
        }
        // A stash's drop stays on this level: it has no card to nest in.
        AppMenuItem {
            id: refStashDropItem
            code: "drop"
            offered: refRowMenu.kind === "stash" && refRowMenu.canDelete
            holdPresses: Metrics.holdPressesWarned
            onHeld: {
                refStashDropItem.closeMenu()
                refRowMenu.dropStashRequested(refRowMenu.refId)
            }
        }
        AppMenuSeparator {}
        // The same card the graph row's menu carries.
        RefBranchMenu {
            id: branchMenu
            heldReason: refRowMenu.heldReason
            runningWhy: refRowMenu.runningWhy
            onWaitEnded: refRowMenu.standAgain()
            // git's two answers, live: the card turns its own row on them while it stands.
            refusedDelete: refRowMenu.repoTab.branchDeleteRefused
            landedDelete: refRowMenu.repoTab.branchDeleteLanded
            checkedBranch: refRowMenu.repoTab.branchDeleteAsked
            checkedMerged: refRowMenu.repoTab.branchDeleteMerged
            onDeleteRequested: (kind, id, name, oidHex) => refRowMenu.deleteRequested(kind, id, name, oidHex)
            onUpstreamRequested: (branch, counterpart) => refRowMenu.upstreamRequested(branch, counterpart)
            onPushRequested: (branch, force, lease) => refRowMenu.repoTab.pushNamed(branch, force, lease)
            onPublishRequested: (branch, upstream) => refRowMenu.publishRequested(branch, upstream)
            onCheckDeleteRequested: branch => refRowMenu.repoTab.checkBranchDelete(branch)
            onForceDeleteRequested: branch => refRowMenu.repoTab.deleteBranch(branch, true)
            onDeleteRemoteRequested: (remoteRef, expect) => refRowMenu.deleteRemoteNow(remoteRef, expect)
            onDeleteEverywhereRequested: (branch, remoteRef, forced, expect) =>
                refRowMenu.deleteEverywhereNow(branch, remoteRef, forced, expect)
        }
        AppMenuSeparator {}
        // A worktree made off this row, and the one this row names or leads to; above the TAG card
        // (デザイン規約 §メニュー の入れ子). The same card the graph row's menu carries.
        RefWorktreeMenu {
            id: worktreeMenu
            heldReason: refRowMenu.heldReason
            runningWhy: refRowMenu.runningWhy
            onWaitEnded: refRowMenu.standAgain()
            onWorktreeHereRequested: oidHex => refRowMenu.worktreeHereRequested(oidHex)
            onWorktreeAddRequested: (mode, branch, start, path, name) =>
                refRowMenu.worktreeAddRequested(mode, branch, start, path, name)
            onRemoveRequested: (path, name, holds) => refRowMenu.removeWorktreeRequested(path, name, holds)
        }
        AppMenuSeparator {}
        // The same card the graph row's menu carries.
        RefTagMenu {
            id: tagMenu
            heldReason: refRowMenu.heldReason
            runningWhy: refRowMenu.runningWhy
            onWaitEnded: refRowMenu.standAgain()
            canBranchHere: refRowMenu.canBranchHere
            onTagHereRequested: oidHex => refRowMenu.tagHereRequested(oidHex)
            onPushTagRequested: (remote, tag, lease) => refRowMenu.repoTab.pushTag(remote, tag, lease)
            onDeleteTagRequested: tag => refRowMenu.repoTab.deleteTag(tag)
            onDeleteRemoteTagRequested: (remote, tag, onlyThere, expect) =>
                refRowMenu.repoTab.deleteRemoteTag(remote, tag, onlyThere, expect)
            onDeleteTagEverywhereRequested: (tag, remote, expect) =>
                refRowMenu.repoTab.deleteTagEverywhere(tag, remote, expect)
        }
    }
}
