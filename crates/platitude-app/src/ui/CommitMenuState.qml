import QtQuick
import platitude
import platitude.ui

// ---- what the commit menu is standing on ----------------------------
// The row the menu was opened on and what it may offer there. Held apart from the page because the answers are decided
// once, in the function that opens the menu, and hold still while it stands (app-ui.md §メニュー) — one place to
// open it is also one place to look for why a row is not on offer.
//
// Nothing is drawn here, so this is a `QtObject`: the menu itself is declared on the page, where it has an item to
// measure the window through (rules-refs/structure.md §描かないホスト).
QtObject {
    id: menuState

    required property RepoTab repoTab
    required property WorkTreeModel workTree
    required property GraphModel graphModel
    /// Which working copy has a branch checked out — the one answer the `switch` row reads that the commit's own
    /// rules cannot give (`RefRowMenu` asks it the same way at the other entrance).
    required property NavSectionModel worktreesModel
    /// What the two cards at the foot need worked out for them: the reading a branch carries and where a tag stands
    /// (`cardFacts`).
    required property NavSectionModel branchesModel
    required property NavSectionModel tagsModel
    /// The menu these answers are handed to, and the one this opens.
    required property CommitRowMenu menu

    property string menuOid: ""
    // The stash the menu was opened on, by the selector git answers to ("" on an ordinary commit). A stash is a commit
    // git keeps off to one side of every branch, so none of the commit menu's rows land on it and it gets its own
    // (デザイン規約 §グラフ行の右クリック).
    property string menuStashRef: ""
    // Whether a remote already has the menu's commit. Rewriting it goes ahead unasked — nothing here leaves the machine
    // — but デザイン規約 「push 済みの範囲は言うだけ」 wants it said, so the squash row carries a tag the way the amend editor does.
    //
    // **Read off the row, in hand as the menu opens** (`GraphModel.publishedAt`). The walk that drew the row already
    // marked it, so nothing here waits on git: a note arriving a frame later would grow the widest row and take the
    // card's right edge — and the `▸` on it — out from under the hand that is already reaching for a row
    // (app-ui.md §メニュー).
    property bool menuPublished: false
    /// What these menus offer, held still for as long as they stand. What each stands for, and the state it asks of
    /// the repository, is core's rule (offers::commit_menu — the doc there carries the measured refusals).
    property bool menuCanSequence: false
    property bool menuCanIntegrate: false
    property bool menuCanEditHistory: false
    property bool menuCanMoveBranch: false
    property bool menuCanBranchHere: false
    property bool menuStashCanWrite: false
    /// Whether the name this row draws is somewhere to move to, and whether the press raises a question first. Off
    /// the ref rules, where the destination is the name
    /// (offers::ref_menu).
    property bool menuCanSwitch: false
    property bool menuSwitchAsks: false
    /// Whether that name has a far side a `git pull` would go to, off the same rules and in the same one ask
    /// (offers::ref_menu) — the chip's entrance to the row the sidebar's own draws.
    property bool menuCanPull: false
    /// And whether git would turn that press down for want of orders, which greys the row
    /// (`WorkTreeModel.pullBlocked`).
    property bool menuPullBlocked: false
    /// How many files the reset submenu's `--hard` would take with it besides the commits — read once here with every
    /// other answer, because the tag it feeds sits at the end of a row and widens the card: a count that grew while
    /// the menu stood would move the card's edge under the hand (app-ui.md §メニュー).
    property int menuHardResetTakes: 0
    /// Whether anything besides this branch still reaches the tip, which is what decides whether the drop row is
    /// held. Read here for the same reason as the count above and one more: a mark arriving later re-indents **every**
    /// row (`AppMenu.holdIndent`), so the words move under a hand already reaching for one.
    property bool menuTipHeldElsewhere: false

    /// What the `switch` and `pull` rows read, asked of the name the menu is aimed at. Empty on a row that draws none,
    /// which is what takes both rows off the menu.
    function askRefRows(kind, name, oidHex) {
        menuState.menuCanSwitch = false
        menuState.menuSwitchAsks = false
        menuState.menuCanPull = false
        menuState.menuPullBlocked = false
        if (kind !== "branch" && kind !== "remote")
            return
        // A remote row lands on the local branch of the same name, so it is that one another copy can be holding.
        const held = kind === "branch"
            ? menuState.worktreesModel.worktreeHolding(name)
            : menuState.worktreesModel.worktreeHolding(menuState.repoTab.localNameFor(name))
        const offers = GitFacts.refMenuOffers(
            kind, name, oidHex,
            menuState.repoTab.state === "open",
            menuState.menu.heldReason !== "" ? 0 : menuState.repoTab.busyCount,
            menuState.workTree.branch, menuState.workTree.detached,
            menuState.workTree.opText, menuState.workTree.conflictCount,
            // The drift is the card's: what this level reads is where a move lands, and the rows that reach a
            // remote are the card's (`RefBranchMenu`). The last one is the working tree's own upstream, the same
            // value the sidebar's entrance hands in — what the `pull` row reads (`RefRowMenu.offerOn`).
            held, "", false, menuState.repoTab.defaultRemote, "",
            menuState.workTree.upstream)
        menuState.menuCanSwitch = offers.includes("switch")
        menuState.menuSwitchAsks = offers.includes("asks")
        menuState.menuCanPull = offers.includes("pull")
        menuState.menuPullBlocked = menuState.menuCanPull && menuState.workTree.pullBlocked
    }

    /// What the two cards at the foot stand on, both aimed at the name the menu is aimed at — and each emptied where
    /// that name is the other one's kind, so the card that cannot name it comes up holding only what can still be made
    /// here. Read here rather than in the menu for the reason every other answer is: the models are this side of the
    /// door, and a lookup arriving a frame later moves the card's edge out from under the hand (app-ui.md §メニュー).
    function cardFacts(oidHex) {
        const kind = menuState.menu.targetKind
        const branchy = kind === "branch" || kind === "remote"
        const name = menuState.menu.targetName
        return {
            "branch": menuState.branchFacts(branchy ? kind : "", branchy ? name : "", oidHex),
            "tag": menuState.tagFacts(kind === "tag" ? "tag" : "", kind === "tag" ? name : "", oidHex)
        }
    }

    /// What the BRANCH card stands on — the other entrance to it (`RefRowMenu.branchFacts` says why it is read here
    /// and not in the card).
    function branchFacts(kind, full, oidHex) {
        if (kind !== "branch" && kind !== "remote")
            return { "heldByWorktree": "", "holderLeaf": "", "remoteCounterpart": "", "remoteDrifted": false,
                     "offers": [], "open": false, "merged": "" }
        const held = kind === "branch"
            ? menuState.worktreesModel.worktreeHolding(full)
            : menuState.worktreesModel.worktreeHolding(menuState.repoTab.localNameFor(full))
        const counterpart = kind === "branch" ? menuState.branchesModel.upstreamOf(full) : ""
        const drifted = kind === "branch" && menuState.branchesModel.upstreamDrifted(full)
        const open = menuState.repoTab.state === "open"
        return {
            "heldByWorktree": held,
            "holderLeaf": held === "" ? "" : GitFacts.pathLeaf(held),
            "remoteCounterpart": counterpart,
            "remoteDrifted": drifted,
            "open": open,
            "merged": !open || kind !== "branch" ? "" : menuState.graphModel.branchDeleteMerged(
                oidHex, menuState.branchesModel.upstreamOidOf(full), menuState.workTree.headOid),
            "offers": GitFacts.refMenuOffers(
                kind, full, oidHex, open,
                menuState.menu.heldReason !== "" ? 0 : menuState.repoTab.busyCount,
                menuState.workTree.branch, menuState.workTree.detached,
                menuState.workTree.opText, menuState.workTree.conflictCount,
                held, counterpart, drifted, menuState.repoTab.defaultRemote, "", "")
        }
    }

    /// What the TAG card stands on — the other entrance to it, reading the same section and asking core the same
    /// question (`RefRowMenu.tagFacts` says why it is read here and not in the card).
    function tagFacts(kind, full, oidHex) {
        const remote = menuState.repoTab.defaultRemote
        const drift = kind === "tag" ? menuState.tagsModel.remoteTagDrift(full, remote) : ""
        const sides = kind === "tag" ? menuState.tagsModel.tagSides(full) : ""
        return {
            "pushRemote": remote,
            "tagDriftOid": drift,
            "tagOnlyThere": sides === "remote",
            "offers": kind !== "tag" ? [] : GitFacts.refMenuOffers(
                kind, full, oidHex,
                menuState.repoTab.state === "open",
                menuState.menu.heldReason !== "" ? 0 : menuState.repoTab.busyCount,
                menuState.workTree.branch, menuState.workTree.detached,
                menuState.workTree.opText, menuState.workTree.conflictCount,
                "", "", drift !== "", remote, sides, "")
        }
    }

    /// The reading over there gone on its own, and the pair at once — cut against the configured remote names, which
    /// are the tab's (`RefRowMenu.deleteRemoteNow`).
    function deleteRemoteNow(remoteRef) {
        const remote = GitFacts.remoteOfRef(remoteRef, menuState.repoTab.remoteNames)
        if (remote === "")
            return
        menuState.repoTab.deleteRemoteBranch(
            remote, GitFacts.branchOfRef(remoteRef, menuState.repoTab.remoteNames))
    }

    function deleteEverywhereNow(branch, remoteRef, forced) {
        const remote = GitFacts.remoteOfRef(remoteRef, menuState.repoTab.remoteNames)
        if (remote === "")
            return
        menuState.repoTab.deleteBranchEverywhere(
            branch, remote, GitFacts.branchOfRef(remoteRef, menuState.repoTab.remoteNames), forced)
    }

    function openRowMenu(oidHex) {
        menuState.menuOid = oidHex
        menuState.menuStashRef = menuState.graphModel.stashRefOf(oidHex)
        const offers = GitFacts.commitMenuOffers(
            // Nothing running, while the doors are held: the hold already answers for every row, and a row the busy
            // count took away would leave the reader watching the menu change shape around a lock they are waiting to
            // see lifted (`RefRowMenu.askBusy` — the same rule at the other entrance).
            menuState.repoTab.state === "open",
            menuState.menu.heldReason !== "" ? 0 : menuState.repoTab.busyCount,
            menuState.workTree.branch, menuState.workTree.detached, menuState.workTree.opText,
            oidHex, menuState.workTree.headOid, menuState.menuStashRef)
        if (menuState.menuStashRef !== "") {
            menuState.menuStashCanWrite = offers.includes("stash-write")
            menuState.menu.offerStash()
            return
        }
        menuState.menuPublished = menuState.graphModel.publishedAt(oidHex)
        menuState.menuHardResetTakes = menuState.workTree.hardResetTakes
        menuState.menuTipHeldElsewhere = menuState.workTree.headReachedElsewhere
        menuState.menuCanSequence = offers.includes("sequence")
        menuState.menuCanIntegrate = offers.includes("integrate")
        menuState.menuCanEditHistory = offers.includes("edit-history")
        menuState.menuCanMoveBranch = offers.includes("move-branch")
        menuState.menuCanBranchHere = offers.includes("branch-here")
        menuState.askRefRows(menuState.menu.targetKind, menuState.menu.targetName, oidHex)
        menuState.menu.offerCommit(menuState.cardFacts(oidHex))
    }
}
