import QtQuick
import platitude
import platitude.ui

// ---- what the commit menu is standing on ----------------------------
// The row the menu was opened on and what it may offer there, decided in `openRowMenu` and stood again on the tree as
// it is while the menu stands — when a command it waited for has ended (`standAgain`) and when what it reads moves
// (`follow`), its rows kept (デザイン規約 §メニュー). One place to look for why a row is not on offer.
//
// A `QtObject`: it holds no child (rules-refs/structure.md「切り出した非表示のホストは `Item` にする」). The menu
// itself is declared on the page, which gives it an item to measure the window through (「描かないホスト」).
QtObject {
    id: menuState

    required property RepoTab repoTab
    required property WorktreeModel worktree
    /// The worktree as it stood when the command the menus wait for was asked (`RepoPage.treeAtAsk`); null while
    /// nothing is out. Back to null with the menu up, the rows stand on what the command left (`follow`) — a menu up as
    /// it ran, waiting for it or not.
    property var treeAtAsk: null
    onTreeAtAskChanged: if (menuState.treeAtAsk === null) menuState.follow()
    /// What the rows are read off, set by the door that stands them: the worktree's `reading` — or, for a menu raised
    /// under a command, `treeAtAsk`. Handed to `GitFacts` as it is, never built here. Null before the first door, not
    /// bound: nothing reads it outside a door, and a binding would build the record on every status drain.
    property var tree: null
    /// Moves each time what the rows stand on has been read again and moved (`RepoPage.menuReadRevision`): a menu
    /// standing follows it (`follow`).
    property int readRevision: 0
    onReadRevisionChanged: followLater.restart()
    readonly property TurnLater followLater: TurnLater {
        id: followLater
        onTriggered: menuState.follow()
    }
    required property GraphModel graphModel
    /// What the rows on the name the menu is aimed at and the cards at the foot stand on, read through `menuFacts`;
    /// where the name points now (`aimedAt`) and what it was renamed to (`standAgain`) off the sections themselves.
    required property NavSectionModel worktreesModel
    required property NavSectionModel branchesModel
    required property NavSectionModel tagsModel
    required property NavSectionModel remotesModel
    /// What the GITHUB card at the foot stands on.
    required property GitHubLinks githubLinks
    /// The menu these answers are handed to, and the one this opens.
    required property CommitRowMenu menu
    /// What the rows on the menu's name and every card stand on, read in one place for this entrance and the
    /// sidebar's (`RefRowMenu.menuFacts`).
    readonly property RefMenuFacts menuFacts: RefMenuFacts {
        id: menuFacts
        repoTab: menuState.repoTab
        graphModel: menuState.graphModel
        worktreesModel: menuState.worktreesModel
        branchesModel: menuState.branchesModel
        remotesModel: menuState.remotesModel
        tagsModel: menuState.tagsModel
    }

    property string menuOid: ""
    // The stash's selector ("" on an ordinary commit); a stash gets its own menu (デザイン規約 §グラフ行の右クリック).
    property string menuStashRef: ""
    // Whether a remote already has the menu's commit — the rewrite rows say so with a tag and do not ask
    // (デザイン規約 §回数押し「ローカルの履歴書き換えはクリック」). Read off the row (`GraphModel.publishedAt`), so it is
    // in hand as the menu opens (デザイン規約 §行が読む答えはどこから来るか).
    property bool menuPublished: false
    /// Whether a rebase onto the menu's commit would rewrite a commit a remote has (`<ref>..HEAD`), off the same rows
    /// (`GraphModel.rebaseRewritesPublished`) — the `rebase` row's note, as `RefRowMenu.rebasePublished`.
    property bool menuRebasePublished: false
    /// What these menus offer on the reading they stand on (`standRow`). The rules are core's (`offers::commit_menu`).
    property bool menuCanCherryPick: false
    property bool menuCanRevert: false
    property bool menuCanIntegrate: false
    property bool menuCanEditHistory: false
    property bool menuCanMoveBranch: false
    property bool menuCanBranchHere: false
    /// An operation stands: every row that starts or rewrites history is there but greyed.
    property bool menuOpStanding: false
    property bool menuStashCanWrite: false
    /// Whether the row's name is somewhere to move to, and whether the press asks first (`offers::ref_menu`).
    property bool menuCanSwitch: false
    property bool menuSwitchAsks: false
    /// …or leads to another worktree instead: the name the one holding that name goes by, empty when none does
    /// (`RefRowMenu.heldLeaf`).
    property string menuHeldLeaf: ""
    /// That worktree is the repository's own (`RefRowMenu.heldHome`).
    property bool menuHeldHome: false
    /// What the `pull` row runs on that name and why it is greyed — the same `offers::pull_row` ask as
    /// `RefRowMenu.pullRuns` / `pullOut`.
    property string menuPullRuns: ""
    property string menuPullOut: ""
    property string menuPullUpstream: ""
    /// How many files the `--hard` row would take besides the commits, off the reading the rows stand on — read with
    /// them, not bound to the model, so its note comes and goes as the menu stands again (デザイン規約 §メニュー).
    property int menuHardResetTakes: 0
    /// Whether those files hold one the discard record cannot copy, off the same reading: it dresses the row.
    property bool menuHardResetNotCopied: false
    /// Whether anything besides this branch still reaches the tip, which decides whether the drop row is held — off
    /// the same reading. A hold mark coming re-indents every row (`AppMenu.holdIndent`), which only the menu standing
    /// again brings: the press count the commit now needs wins over the words' x.
    property bool menuTipHeldElsewhere: false

    /// The menu on another worktree's uncommitted row (デザイン規約 §別の worktree を読む), at `path` as the row names
    /// it — read off the tree as `openRowMenu` reads it, and greyed while a command is out as every menu is.
    function openCarriedMenu(path, name) {
        const menu = menuState.menu
        menu.releaseShape()
        menuState.tree = menuState.treeAtAsk !== null ? menuState.treeAtAsk : menuState.worktree.reading
        menu.carriedPath = path
        menu.carriedName = name
        const carried = menuState.graphModel.carriedAt(menuState.graphModel.carriedRowOf(path))
        menu.carriedHome = carried !== undefined && carried.home
        menuState.standCarried()
        menu.offerCarried()
    }
    /// Whether that worktree's work has an apply row — false where the row is gone — and what greys it: a conflict
    /// standing on either side (デザイン規約 §メニュー「「今できない」行は無効で残す」; user decision — ask before changing).
    /// There: which of its sides to carry is not decided. Here: git applies nothing over an unmerged index — so with
    /// both, the reason says here. That worktree's count is the row's, as old as its last read (`session::pace`); the
    /// write reads the tree afresh when its turn comes (`RepoSession::apply_carried`).
    function standCarried() {
        const menu = menuState.menu
        const carried = menuState.graphModel.carriedAt(menuState.graphModel.carriedRowOf(menu.carriedPath))
        const tally = carried !== undefined ? carried.tally : undefined
        menu.carriedApplyOffered = menuState.repoTab.state === "open" && tally !== undefined
        menu.carriedApplyOut = !menu.carriedApplyOffered ? ""
                             : menuState.tree.conflictCount > 0 ? "here"
                             : tally.conflicted > 0 ? "there" : ""
    }

    function openRowMenu(oidHex) {
        menuState.menu.releaseShape()
        menuState.tree = menuState.treeAtAsk !== null ? menuState.treeAtAsk : menuState.worktree.reading
        const facts = menuState.standRow(oidHex, oidHex)
        if (facts === null) {
            menuState.menu.offerStash()
            return
        }
        menuState.menu.standGithub(
            menuState.githubLinks.ofRef(oidHex, menuState.menu.targetKind, menuState.menu.targetName, facts.card))
        menuState.menu.offerCommit(facts)
    }

    /// What the rows stand on moved with the menu up — outside while nothing runs, or with a command that has just
    /// ended: the rows stand again on the tree as it is, as when a wait ends (`standAgain`; デザイン規約 §メニュー
    /// 「今の内容」). Not while a command is out: the menu stands on the tree as that was asked until it ends
    /// (`treeAtAsk`), whatever the write.
    function follow() {
        if (menuState.treeAtAsk !== null)
            return
        menuState.standAgain("follow")
    }

    /// The command the menu waited for has ended with the menu up: what it left is read again and the rows stand on
    /// it, the menu keeping its shape (`AppMenu.holdShape`) — the stash's selector the row now answers to, the notes
    /// and the presses off where HEAD now is, the BRANCH card off where its branch now points, the rows an operation it
    /// stopped in takes away greyed. A row the command took off the graph (a stash popped, a commit dropped), or the
    /// name the menu was aimed at taken away, leaves the menu nothing to act on. `again`: `wait` for that, `follow`
    /// for what moved with the menu up (`follow`) — the cards' rows that reach over there keep their leases either way,
    /// and git's answers about the branch only while following (`RefBranchMenu.standOn`). The pair's menu reads
    /// nothing of the tree: its links name the two commits, and it starts no git.
    function standAgain(again) {
        const menu = menuState.menu
        if (menu === null || !menu.showing || menu.choiceShowing)
            return
        // Another worktree's row: gone where that worktree went clean, its apply read again otherwise.
        // No test reaches this branch: `tst_commitrowmenu` covers the menu's side (the wait, the shape,
        // `withdraw`), and choosing between withdrawing and reading again needs a graph model with the row.
        if (menu.carriedShowing) {
            if (menuState.graphModel.carriedRowOf(menu.carriedPath) < 0) {
                menu.withdraw()
                return
            }
            menu.holdShape()
            menuState.tree = menuState.worktree.reading
            menuState.standCarried()
            return
        }
        const wasStash = menuState.menuStashRef !== ""
        // The branch the menu was aimed at renamed outside with the menu up answers to its new name, as on the left
        // menu (`RefRowMenu.standAgain`): the chip on the row says it now.
        if (menu.targetKind === "branch" && menuState.branchesModel.oidOfName(menu.targetName) === "") {
            const to = menuState.branchesModel.renamedTo(menu.targetName)
            if (to !== "") {
                menu.branchCard.renamed(menu.targetName, to)
                menu.targetName = to
            }
        }
        const aimAt = menuState.aimedAt()
        if (menuState.graphModel.rowOf(menuState.menuOid) < 0 || aimAt === ""
                || (menuState.graphModel.stashRefOf(menuState.menuOid) !== "") !== wasStash) {
            menu.withdraw()
            return
        }
        menu.holdShape()
        menuState.tree = menuState.worktree.reading
        const facts = menuState.standRow(menuState.menuOid, aimAt)
        if (facts !== null)
            menu.standCards(facts, aimAt, again)
    }
    /// Where the name the menu was aimed at points now, empty where it is gone: a local branch may have moved off the
    /// row (a commit takes the current branch with it) and its card goes with it. So does a remote branch that moved —
    /// a fetch outside, or the one the menu waited for (デザイン規約 §hover「外乱で閉じない」): its `push --delete`
    /// stays leased to where it stood as the menu went up and greys, since on the new tip it would take commits the
    /// menu never showed (`RefBranchMenu.standOn`). The rest stand on the row's commit — a tag's chip may be one
    /// remote's reading, and the WORKTREE card's rows act on the commit.
    function aimedAt() {
        const kind = menuState.menu.targetKind
        const name = menuState.menu.targetName
        if (kind === "branch")
            return menuState.branchesModel.oidOfName(name)
        if (kind === "remote")
            return menuState.remotesModel.oidOfName(name)
        const there = kind === "tag" ? menuState.tagsModel.oidOfName(name) !== ""
                    : kind !== "worktree" || menuState.worktreesModel.worktreeFacts(name) !== undefined
        return there ? menuState.menuOid : ""
    }

    /// Reads everything the rows stand on off `tree`, for the row `oidHex` with its name at `aimAt`, and stands them
    /// on it: what the cards stand on, or null for a stash's row, which has no cards.
    function standRow(oidHex, aimAt) {
        menuState.menuOid = oidHex
        menuState.menuStashRef = menuState.graphModel.stashRefOf(oidHex)
        // No running command is asked about: the menu waits for one with its rows greyed (`AppMenu.runningWhy`), so it
        // has one shape whenever it is opened.
        const offers = GitFacts.commitMenu(
            menuState.repoTab.state === "open", menuState.tree, oidHex, menuState.menuStashRef !== "",
            menuState.graphModel.provisionalAt(menuState.graphModel.rowOf(oidHex)))
        if (menuState.menuStashRef !== "") {
            menuState.menuStashCanWrite = offers.stashWrite
            return null
        }
        menuState.menuPublished = menuState.graphModel.publishedAt(oidHex)
        menuState.menuRebasePublished = menuState.graphModel.rebaseRewritesPublished(oidHex, menuState.tree.headOid)
        menuState.menuHardResetTakes = menuState.tree.hardResetTakes
        menuState.menuHardResetNotCopied = menuState.tree.hardResetNotCopied
        menuState.menuTipHeldElsewhere = menuState.tree.headReachedElsewhere
        menuState.menuCanCherryPick = offers.cherryPick
        menuState.menuCanRevert = offers.revert
        menuState.menuCanIntegrate = offers.integrate
        menuState.menuCanEditHistory = offers.editHistory
        menuState.menuCanMoveBranch = offers.moveBranch
        menuState.menuCanBranchHere = offers.branchHere
        menuState.menuOpStanding = offers.standing
        // The name the menu is aimed at: its `switch` and `pull` rows and every card at the foot, the BRANCH card where
        // its branch points (`aimAt`). A chip drawing one remote's reading of a tag names that remote on its own, so
        // the TAG card's `push` and deletes act on it (user decision — ask before changing;
        // `NavSectionModel.tagAimAt`); the copy here and a reading several remotes share on this commit name none, and
        // the card's default stands.
        const kind = menuState.menu.targetKind
        const name = menuState.menu.targetName
        const facts = menuFacts.stand(kind, name, oidHex, menuState.tree,
                                      kind === "tag" ? menuState.tagsModel.tagAimAt(name, oidHex) : "", aimAt)
        const menu = facts.menu
        menuState.menuCanSwitch = menu.switch
        menuState.menuSwitchAsks = menu.switchAsks
        menuState.menuHeldLeaf = facts.lead.called
        menuState.menuHeldHome = facts.lead.home
        menuState.menuPullUpstream = kind === "branch" ? facts.card.remoteCounterpart : ""
        menuState.menuPullRuns = menu.pullRuns
        menuState.menuPullOut = menu.pullOut
        return facts
    }
}
