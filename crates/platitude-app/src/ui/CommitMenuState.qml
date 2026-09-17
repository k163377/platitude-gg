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
    /// How many files the reset submenu's `--hard` would take with it besides the commits — read once here with every
    /// other answer, because the tag it feeds sits at the end of a row and widens the card: a count that grew while
    /// the menu stood would move the card's edge under the hand (app-ui.md §メニュー).
    property int menuHardResetTakes: 0
    /// Whether anything besides this branch still reaches the tip, which is what decides whether the drop row is
    /// held. Read here for the same reason as the count above and one more: a mark arriving later re-indents **every**
    /// row (`AppMenu.holdIndent`), so the words move under a hand already reaching for one.
    property bool menuTipHeldElsewhere: false

    /// What the `switch` row reads, asked of the name the menu is aimed at. Empty on a row that draws none, which is
    /// what takes the row off the menu.
    function askSwitch(kind, name, oidHex) {
        menuState.menuCanSwitch = false
        menuState.menuSwitchAsks = false
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
            // remote are the card's (`RefBranchMenu`).
            held, "", false, menuState.repoTab.defaultRemote, "").split(" ")
        menuState.menuCanSwitch = offers.includes("switch")
        menuState.menuSwitchAsks = offers.includes("asks")
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
            oidHex, menuState.workTree.headOid, menuState.menuStashRef).split(" ")
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
        menuState.askSwitch(menuState.menu.targetKind, menuState.menu.targetName, oidHex)
        menuState.menu.offerCommit()
    }
}
