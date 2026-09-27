import QtQuick
import platitude
import platitude.ui

// ---- what the commit menu is standing on ----------------------------
// The row the menu was opened on and what it may offer there, decided once in `openRowMenu` and held still while the
// menu stands (デザイン規約 §メニュー) — one place to look for why a row is not on offer.
//
// A `QtObject`: it holds no child (rules-refs/structure.md「切り出した非表示のホストは `Item` にする」). The menu
// itself is declared on the page, which gives it an item to measure the window through (「描かないホスト」).
QtObject {
    id: menuState

    required property RepoTab repoTab
    required property WorkTreeModel workTree
    required property GraphModel graphModel
    /// Which working copy has a branch checked out — what the `switch` row needs beyond the commit's own rules.
    required property NavSectionModel worktreesModel
    /// What the cards at the foot need worked out for them: the reading a branch carries and where a tag stands
    /// (`cardFacts`).
    required property NavSectionModel branchesModel
    required property NavSectionModel tagsModel
    /// The menu these answers are handed to, and the one this opens.
    required property CommitRowMenu menu

    property string menuOid: ""
    // The stash's selector ("" on an ordinary commit); a stash gets its own menu (デザイン規約 §グラフ行の右クリック).
    property string menuStashRef: ""
    // Whether a remote already has the menu's commit — the rewrite rows say so with a tag and do not ask
    // (デザイン規約 §長押し「ローカルの履歴書き換えはクリック」). Read off the row (`GraphModel.publishedAt`), so it is
    // in hand as the menu opens (デザイン規約 §行が読む答えはどこから来るか).
    property bool menuPublished: false
    /// What these menus offer, held still while they stand. The rules are core's (`offers::commit_menu`).
    property bool menuCanSequence: false
    property bool menuCanIntegrate: false
    property bool menuCanEditHistory: false
    property bool menuCanMoveBranch: false
    property bool menuCanBranchHere: false
    property bool menuStashCanWrite: false
    /// Whether the row's name is somewhere to move to, and whether the press asks first (`offers::ref_menu`).
    property bool menuCanSwitch: false
    property bool menuSwitchAsks: false
    /// …or leads to another working copy instead: the folder of the one holding that name, empty when none does
    /// (`RefRowMenu.heldLeaf`).
    property string menuHeldLeaf: ""
    /// Whether that name has a far side a `git pull` would go to (the same `offers::ref_menu` ask).
    property bool menuCanPull: false
    /// And whether git would turn that press down for want of orders, which greys the row
    /// (`WorkTreeModel.pullBlocked`).
    property bool menuPullBlocked: false
    /// How many files the `--hard` row would take besides the commits — held still with the rest, since its tag
    /// widens the card (デザイン規約 §メニュー).
    property int menuHardResetTakes: 0
    /// Whether anything besides this branch still reaches the tip, which decides whether the drop row is held. Held
    /// still too: a hold mark arriving later re-indents every row (`AppMenu.holdIndent`).
    property bool menuTipHeldElsewhere: false

    /// What the `switch` and `pull` rows read, asked of the name the menu is aimed at. Empty on a row that draws none,
    /// which is what takes both rows off the menu.
    function askRefRows(kind, name, oidHex) {
        menuState.menuCanSwitch = false
        menuState.menuSwitchAsks = false
        menuState.menuHeldLeaf = ""
        menuState.menuCanPull = false
        menuState.menuPullBlocked = false
        if (kind !== "branch" && kind !== "remote" && kind !== "worktree")
            return
        const held = menuState.copyLeadingFrom(kind, name)
        const offers = GitFacts.refMenuOffers(
            kind, name, oidHex,
            menuState.repoTab.state === "open",
            menuState.menu.heldReason !== "" ? 0 : menuState.repoTab.busyCount,
            menuState.workTree.branch, menuState.workTree.detached,
            menuState.workTree.opText, menuState.workTree.conflictCount,
            // No drift: the rows that reach a remote are the card's (`RefBranchMenu`). The last is the working
            // tree's own upstream, which the `pull` row reads.
            held, "", false, menuState.repoTab.defaultRemote, "",
            menuState.workTree.upstream)
        menuState.menuCanSwitch = offers.includes("switch")
        menuState.menuSwitchAsks = offers.includes("asks")
        menuState.menuHeldLeaf = held === "" ? "" : GitFacts.pathLeaf(held)
        menuState.menuCanPull = offers.includes("pull")
        menuState.menuPullBlocked = menuState.menuCanPull && menuState.workTree.pullBlocked
    }

    /// The other working copy the menu's name leads to: the one holding the branch (a remote row through the local
    /// branch of the same name), or the copy a folder's chip names — unless this tab stands in it. Empty for none.
    function copyLeadingFrom(kind, name) {
        if (kind === "branch")
            return menuState.worktreesModel.worktreeHolding(name)
        if (kind === "remote")
            return menuState.worktreesModel.worktreeHolding(menuState.repoTab.localNameFor(name))
        if (kind === "worktree" && !GitFacts.samePath(name, menuState.repoTab.repoPath))
            return name
        return ""
    }

    /// What the cards at the foot stand on, aimed at the menu's name — each emptied where that name is another's
    /// kind, so that card holds only what can still be made here. The WORKTREE card stands on the copy a folder's
    /// chip names, or on the one holding the chip's local branch — not a remote chip's, which only leads there
    /// (`RefRowMenu.offerOn`). Read here, where the models are.
    function cardFacts(oidHex) {
        const kind = menuState.menu.targetKind
        const branchy = kind === "branch" || kind === "remote"
        const name = menuState.menu.targetName
        const copy = kind === "worktree" ? name : kind === "branch" ? menuState.copyLeadingFrom(kind, name) : ""
        return {
            "branch": menuState.branchFacts(branchy ? kind : "", branchy ? name : "", oidHex),
            "tag": menuState.tagFacts(kind === "tag" ? "tag" : "", kind === "tag" ? name : "", oidHex),
            "copy": copy === "" ? undefined : menuState.worktreesModel.copyFacts(copy),
            "copyHere": GitFacts.samePath(copy, menuState.repoTab.repoPath),
            "busy": menuState.menu.heldReason !== "" ? 0 : menuState.repoTab.busyCount
        }
    }

    /// What the BRANCH card stands on — the same answer `RefRowMenu.branchFacts` gives at the other entrance.
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

    /// What the TAG card stands on — the same answer `RefRowMenu.tagFacts` gives at the other entrance. A chip drawing
    /// one remote's reading names that remote on its own, so the card acts on it (`NavSectionModel.tagAimAt`).
    function tagFacts(kind, full, oidHex) {
        if (kind !== "tag")
            return { "pushRemote": menuState.repoTab.defaultRemote, "offers": [] }
        const menu = menuState.tagsModel.tagMenu(full, menuState.repoTab.defaultRemote,
                                                 menuState.tagsModel.tagAimAt(full, oidHex))
        const sides = menuState.tagsModel.tagSides(full)
        return {
            "pushRemote": menu.pushRemote,
            "tagDriftOid": menu.lease,
            "tagReach": menu.reach,
            "tagHeldBack": menu.heldBack,
            "tagCarriers": menu.carriers,
            "tagOnlyThere": menu.rowGoes,
            "tagHere": sides === "here" || sides === "both",
            "offers": GitFacts.refMenuOffers(
                kind, full, oidHex,
                menuState.repoTab.state === "open",
                menuState.menu.heldReason !== "" ? 0 : menuState.repoTab.busyCount,
                menuState.workTree.branch, menuState.workTree.detached,
                menuState.workTree.opText, menuState.workTree.conflictCount,
                "", "", menu.heldBack !== "", menu.pushRemote, sides, "")
        }
    }

    /// Delete the remote branch alone, or both at once — the ref split against the tab's configured remote names
    /// (`RefRowMenu.deleteRemoteNow`).
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
            // Nothing running while the doors are held, so the menu keeps its shape
            // (rules-refs/app-ui.md「ロック中のメニューは `busyCount` を 0 として offers を訊く」).
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
