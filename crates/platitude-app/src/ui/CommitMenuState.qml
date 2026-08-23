import QtQuick
import platitude
import platitude.ui

// ---- what the commit menu is standing on ----------------------------
// The row the menu was opened on and what it may offer there. Held apart from the page because the answers are decided
// once, in the function that opens the menu, and must not move again while it stands (app-ui.md §メニュー) — one place to
// open it is also one place to look for why a row is not on offer.
//
// Nothing is drawn here, so this is a `QtObject`: the menu itself is declared on the page, where it has an item to
// measure the window through (rules-refs/structure.md §描かないホスト).
QtObject {
    id: menuState

    required property RepoTab repoTab
    required property WorkTreeModel workTree
    required property GraphModel graphModel
    /// The menu these answers are handed to, and the one this opens.
    required property CommitRowMenu menu

    property string menuOid: ""
    // The stash the menu was opened on, by the selector git answers to ("" on an ordinary commit). A stash is a commit
    // git keeps off to one side of every branch, so none of the commit menu's rows land on it and it gets its own
    // (デザイン規約 §グラフ行の右クリック).
    property string menuStashRef: ""
    // Whether a remote already has the menu's commit. Rewriting it is not asked about — nothing here leaves the machine
    // — but デザイン規約 「push 済みの範囲は尋ねずに言う」 wants it said, so the squash row carries a tag the way the amend editor does.
    // The answer lands a frame after the menu opens.
    property bool menuPublished: false
    /// What these menus offer, held still for as long as they stand. What each stands for, and the state it asks of
    /// the repository, is core's rule (offers::commit_menu — the doc there carries the measured refusals).
    property bool menuCanSequence: false
    property bool menuCanIntegrate: false
    property bool menuCanEditHistory: false
    property bool menuCanMoveBranch: false
    property bool menuCanBranchHere: false
    property bool menuStashCanWrite: false
    function openRowMenu(oidHex) {
        menuState.menuOid = oidHex
        menuState.menuStashRef = menuState.graphModel.stashRefOf(oidHex)
        const offers = GitFacts.commitMenuOffers(
            menuState.repoTab.state === "open", menuState.repoTab.busyCount,
            menuState.workTree.branch, menuState.workTree.detached, menuState.workTree.opText,
            oidHex, menuState.workTree.headOid, menuState.menuStashRef).split(" ")
        if (menuState.menuStashRef !== "") {
            menuState.menuStashCanWrite = offers.includes("stash-write")
            menuState.menu.offerStash()
            return
        }
        menuState.menuPublished = false
        menuState.menuCanSequence = offers.includes("sequence")
        menuState.menuCanIntegrate = offers.includes("integrate")
        menuState.menuCanEditHistory = offers.includes("edit-history")
        menuState.menuCanMoveBranch = offers.includes("move-branch")
        menuState.menuCanBranchHere = offers.includes("branch-here")
        if (menuState.repoTab.state === "open")
            menuState.repoTab.checkPublish(oidHex + "^!")
        menuState.menu.offerCommit()
    }
}
