pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The right-click on a graph row: one menu for a commit, another for a stash, a third for another worktree's
// uncommitted row (デザイン規約 §グラフ行の右クリック) — this worktree's own uncommitted row has none. The first level
// holds the rows used often — on the row's commit, and on the name the chip draws (or the name pressed in the chip's
// stacked list): its pull, push and delete table —; the cards under them hold what is used rarely.
//
// Nothing here is looked up and nothing is run: answers arrive as plain values, held still from the moment the menu
// opens until a command it waited for has ended (`CommitMenuState`, デザイン規約 §メニュー), and presses leave as
// signals for `RepoPage` — which is what lets `tst_commitrowmenu.qml` drive the real menu.
//
// An `Item` because a menu measures the window through the item it was declared under (`AppMenu.ownerItem`).
Item {
    id: rowMenu

    /// Why every row is out while a history-replaying write holds the doors (`RepoPage.doorsHeldWhy`). Every card
    /// takes it; the rows stay and grey (`AppMenu.heldReason`).
    property string heldReason: ""
    /// What the rows say while they wait for a git command this window started (`RepoPage.runningWhy`). Every card
    /// takes it; a card opened under one keeps its rows and greys them until it ends (`AppMenu.runningWhy`).
    property string runningWhy: ""

    /// The name this menu is aimed at: the one the chip draws, or the one pressed in the chip's stacked list. `kind`
    /// is `branch` / `remote` / `tag`, or `worktree` for a worktree's folder chip — its name then the worktree's path —
    /// and empty on a row that draws neither (デザイン規約 §グラフ行の右クリック).
    property string targetKind: ""
    property string targetName: ""
    /// What merge / rebase are handed: the name where the row draws one — a merge by name says so in the commit it
    /// writes, and one by id does not. A worktree's path is no ref, so the commit.
    readonly property string integrateRef:
        rowMenu.targetName !== "" && rowMenu.targetKind !== "worktree" ? rowMenu.targetName : rowMenu.oid
    /// The branch every one of these rows makes its sentence about (規約 §履歴を合流させる).
    required property string branch

    /// The commit the menu was opened on, and the stash it is ("" on an ordinary commit).
    required property string oid
    required property string stashRef
    /// Whether a remote already has that commit: the rewrite rows carry a tag and do not ask
    /// (デザイン規約 §回数押し「ローカルの履歴書き換えはクリック」).
    required property bool published
    /// What these menus offer. `canSequence` is cherry-pick / revert, which only add a commit and so ask less.
    required property bool canSequence
    required property bool canIntegrate
    required property bool canEditHistory
    required property bool canMoveBranch
    required property bool canBranchHere
    /// An operation stands: every row that starts or rewrites history keeps its seat, greyed — the reset card's rows
    /// through the card (`offers::CommitMenuOffers::standing`).
    required property bool opStanding
    /// What those rows say while it does — empty while they are live.
    readonly property string startWhy: rowMenu.opStanding ? Words.finishStoppedFirst : ""
    /// `published` as the rewrite rows wear it: a greyed row warns of nothing, the press it warned of being out.
    readonly property bool publishedLive: rowMenu.published && !rowMenu.opStanding
    required property bool stashCanWrite
    /// Whether anything besides this branch still reaches the tip — what decides whether the drop row is held
    /// (`CommitMenuState.menuTipHeldElsewhere`, the same answer `RebasePlanRunBar` is handed).
    required property bool tipHeldElsewhere
    /// Whether the row's name is somewhere to move to, and whether that move asks first (`offers::ref_menu`).
    required property bool canSwitch
    required property bool switchAsks
    /// …or leads to the worktree holding that name instead: the name that worktree goes by (`RefRowMenu.heldLeaf`).
    required property string heldLeaf
    /// That worktree is the repository's own: its name wears the house (`RefRowMenu.heldHome`).
    property bool heldHome: false
    /// What the `pull` row runs on that name — `here` / `aside`, "" for no row — and why it is greyed — `standing` /
    /// `diverged` / `own` / `held`, "" while live (`RefRowMenu.pullRuns` / `pullOut`).
    required property string pullRuns
    required property string pullOut
    /// The remote branch that name tracks, which the `own` reason names (`RefRowMenu.pullUpstream`).
    required property string pullUpstream
    /// How many uncommitted files `--hard` takes besides the commits (`status::Counts::hard_reset_takes`); 0 on a
    /// clean tree.
    required property int hardResetTakes
    /// Whether what `--hard` takes holds a file the discard record cannot copy (`WorktreeModel.hardResetNotCopied`),
    /// which holds the row in `danger` (デザイン規約 §回数押し の色の表).
    required property bool hardResetNotCopied
    /// git's answers about the branch's plain delete (refusal, landing, early check), live while the menu stands —
    /// passed straight through to `RefBranchMenu`, whose answers the delete rows read.
    property string refusedDelete: ""
    property string landedDelete: ""
    property string checkedBranch: ""
    property string checkedMerged: ""
    /// Another worktree's uncommitted row the third menu stands on: where that worktree is (as its row names it), the
    /// name the row gives it, and whether its apply has a row — greyed by `carriedApplyOut` where it cannot run now
    /// (`CommitMenuState.standCarried`).
    property string carriedPath: ""
    property string carriedName: ""
    /// That worktree is the repository's own, whose name wears the house (`GraphModel.carriedHome`).
    property bool carriedHome: false
    property bool carriedApplyOffered: false
    /// Where a conflict stands that greys the apply — `here` (git applies nothing over one) or `there` (no telling
    /// which side to bring) —, "" while it is live (`CommitMenuState.standCarried`).
    property string carriedApplyOut: ""

    /// Move the worktree to the row's name, by the road a double-click takes (`RepoPage.switchToRef`).
    signal switchRequested(string kind, string name)
    signal branchHereRequested(string oidHex)
    signal tagHereRequested(string oidHex)
    signal squashRequested(string oidHex)
    signal dropRequested(string oidHex)
    /// The full interactive rebase, opened as a plan over `from^..HEAD`. Nothing runs until the plan's own button, so
    /// the row is a plain click.
    signal planRequested(string oidHex)
    signal resetRequested(string mode)
    signal applyStashRequested(string selector)
    signal popStashRequested(string selector)
    signal dropStashRequested(string selector)
    /// The BRANCH card's five, passed straight up — raised by its own rows and by the push row and delete table on the
    /// first level: the delete git may still refuse, the upstream set (`UpstreamFlow`) and taken off, the push, and the
    /// first push's question (`PublishFlow`).
    signal deleteRequested(string kind, string id, string name, string oidHex)
    signal upstreamRequested(string branch, string counterpart, bool unsettable)
    signal unsetUpstreamRequested(string branch)
    signal pushBranchRequested(string branch, string force, string lease)
    signal publishRequested(string branch, string upstream)
    signal cherryPickRequested(string oidHex)
    signal revertRequested(string oidHex)
    signal mergeRequested(string ref)
    signal rebaseRequested(string ref)
    /// `branch` is the name a fast-forward moves, "" for the worktree's own `git pull` (`RepoTab.pull`).
    signal pullRequested(string branch)
    signal checkDeleteRequested(string branch)
    signal forceDeleteRequested(string branch)
    signal deleteRemoteRequested(string remoteRef, string expect)
    signal deleteEverywhereRequested(string branch, string remoteRef, bool forced, string expect)
    signal pushTagRequested(string remote, string tag, string lease)
    signal deleteTagRequested(string tag)
    signal deleteRemoteTagRequested(string remote, string tag, bool onlyThere, string expect)
    signal deleteTagEverywhereRequested(string tag, string remote, string expect)
    /// The WORKTREE card's three, passed straight up (`RefWorktreeMenu`): the box for a new branch in a worktree of
    /// its own, the row's own branch out in one, and `worktree remove`.
    signal worktreeHereRequested(string oidHex)
    signal worktreeAddRequested(string mode, string branch, string start, string path, string name)
    signal removeWorktreeRequested(string path, string name, string holds)
    /// The third menu's two: that worktree opened, by the road its row's double-click takes, and its work applied here.
    signal carriedOpenRequested(string path)
    signal carriedApplyRequested(string path)
    /// A link on its way to the clipboard (`GitHubMenu`); the page owns the clipboard.
    signal copyRequested(string text)
    /// The menu went away — and the stacked list it may have been standing on is the pointer's to answer for again.
    signal dismissed()
    /// A level of this menu waited for a git command and it has ended with the menu up: the page reads again what the
    /// rows stand on (`CommitMenuState.standAgain`).
    signal waitEnded()

    /// Either menu is on screen — what the page reads to hide hover behind it. Read this, not `visible`, which comes
    /// back stale from another file (rules-refs/app-ui.md「メニューが立っている間、裏の hover は伏せる」).
    readonly property bool showing: commitMenu.visible || stashMenu.visible || carriedMenu.visible
                                    || choiceMenu.visible
    /// Whether the one standing is the third, on another worktree's uncommitted row (`CommitMenuState.standAgain`).
    readonly property bool carriedShowing: carriedMenu.visible
    /// …or the fourth, on a choice of two commits, which stands on nothing a command can take away.
    readonly property bool choiceShowing: choiceMenu.visible
    /// Every level of this menu: the four menus, the reset card and the cards under the rows used often. The GitHub
    /// cards' rows never stand again (they start nothing), so their shape is theirs from the opening.
    readonly property var levels: [stashMenu, carriedMenu, choiceMenu, commitMenu, resetMenu, branchCommitMenu,
                                   worktreeCommitMenu, tagCommitMenu]

    /// The automation's handles into these rows, an automation-only exposure the same as `GraphPane.view` is
    /// (rules-refs/app-ui.md「製品の部品はハーネスへ答える相手を丸ごと渡す」).
    readonly property alias menu: commitMenu
    /// What a stash's row raises in its place.
    readonly property alias stashLevel: stashMenu
    readonly property alias branchHereItem: branchHereCommitItem
    readonly property alias switchItem: switchCommitItem
    readonly property alias pullItem: pullCommitItem
    readonly property alias pushBranchItem: pushCommitRow
    readonly property alias deleteItem: deleteCommitRow
    readonly property alias deleteRemoteItem: remoteDeleteCommitRow
    readonly property alias deleteBothItem: bothDeleteCommitRow
    readonly property alias tagHereItem: tagCommitMenu.tagHereItem
    readonly property alias dropItem: dropCommitItem
    readonly property alias stashDropItem: stashDeleteItem
    readonly property alias resetSubmenu: resetMenu
    readonly property alias hardResetRow: hardResetItem
    /// The three cards the rows used rarely hang behind (`AppMenu.openSub`).
    readonly property alias branchCard: branchCommitMenu
    readonly property alias worktreeCard: worktreeCommitMenu
    readonly property alias tagCard: tagCommitMenu
    readonly property alias carriedRowMenu: carriedMenu
    readonly property alias carriedOpenItem: carriedOpenRow
    readonly property alias carriedApplyItem: carriedApplyRow
    readonly property alias githubCard: githubCommitMenu
    readonly property alias choiceLevel: choiceMenu
    readonly property alias choiceGithubCard: githubChoiceMenu

    anchors.fill: parent

    /// Before the rows stand again on a newer reading with the menu up: every level keeps its shape
    /// (`AppMenu.holdShape`).
    function holdShape() {
        for (let i = 0; i < rowMenu.levels.length; i++)
            rowMenu.levels[i].holdShape()
    }
    /// Before a fresh menu stands: the shape a standing one held goes.
    function releaseShape() {
        for (let i = 0; i < rowMenu.levels.length; i++)
            rowMenu.levels[i].releaseShape()
    }

    // Each menu closes itself (a row's `dismiss()`, or the delete row when its answer lands), so nothing here
    // closes them — but for a row the command it waited for took away (`CommitMenuState.standAgain`).
    function offerStash() {
        stashMenu.offer()
    }
    function offerCarried() {
        carriedMenu.offer()
    }
    /// `facts` is what the cards stand on, read where the models are (`CommitMenuState.cardFacts`): `branch` and
    /// `tag`, each already aimed at the target or emptied because the target is the other's kind, `worktree` — the
    /// worktree the target names or the one holding its branch (undefined for none) — with `worktreeHere`, and
    /// `making`, the two rows that make a worktree (`RefWorktreeMenu.standOn`).
    function offerCommit(facts) {
        // Every card stands on its facts before `offer()`: its `applies` decides whether its row counts
        // (`AppMenu.offeredRows`).
        rowMenu.standCards(facts)
        commitMenu.offer()
    }
    /// The GITHUB card's half, before `offerCommit`: the links on the row's commit and its name
    /// (`GitHubLinks.ofRef`). Only as the menu opens — the links do not change with a command it waits for.
    function standGithub(links) {
        githubCommitMenu.standOn(links)
    }
    /// The menu on a choice of two commits: the links that take both (`GitHubLinks.ofPair`). Says whether it
    /// opened — with nothing to offer, the press falls to the one-commit menu (`RepoPage.openRowMenu`).
    function offerChoice(links) {
        githubChoiceMenu.standOn(links)
        return choiceMenu.offer()
    }
    /// The cards' half of `offerCommit`, for the menu standing again on a newer reading as well — where the BRANCH
    /// card's branch may point elsewhere by then, `branchAt` (`CommitMenuState.aimedAt`); left out, the row's commit.
    function standCards(facts, branchAt) {
        worktreeCommitMenu.standOn(facts.worktree, facts.worktreeHere, facts.making)
        const branchy = rowMenu.targetKind === "branch" || rowMenu.targetKind === "remote"
        const branchKind = branchy ? rowMenu.targetKind : ""
        const branchName = branchy ? rowMenu.targetName : ""
        branchCommitMenu.standOn(branchKind, branchName, branchName, branchAt === undefined ? rowMenu.oid : branchAt,
                                 facts.branch)
        const tagged = rowMenu.targetKind === "tag"
        tagCommitMenu.standOn(tagged ? "tag" : "", tagged ? rowMenu.targetName : "", rowMenu.oid, facts.tag)
    }
    /// The commit, the stash or the other worktree's work the menu stands on is gone with the command it waited for:
    /// nothing left to act on. The pair's menu waits for nothing, so it is never withdrawn.
    function withdraw() {
        (stashMenu.visible ? stashMenu : carriedMenu.visible ? carriedMenu : commitMenu).dismiss()
    }

    AppMenu {
        id: stashMenu
        heldReason: rowMenu.heldReason
        runningWhy: rowMenu.runningWhy
        onWaitEnded: rowMenu.waitEnded()
        AppMenuItem {
            code: "apply"
            offered: rowMenu.stashCanWrite
            onTriggered: rowMenu.applyStashRequested(rowMenu.stashRef)
        }
        AppMenuItem {
            code: "pop"
            offered: rowMenu.stashCanWrite
            onTriggered: rowMenu.popStashRequested(rowMenu.stashRef)
        }
        AppMenuSeparator {}
        // Held (デザイン規約 §回数押し).
        AppMenuItem {
            id: stashDeleteItem
            code: "drop"
            offered: rowMenu.stashCanWrite
            holdPresses: Metrics.holdPressesWarned
            onHeld: {
                stashDeleteItem.closeMenu()
                rowMenu.dropStashRequested(rowMenu.stashRef)
            }
        }
    }

    // Another worktree's uncommitted row: its door, then its work brought here (デザイン規約 §別の worktree を読む).
    AppMenu {
        id: carriedMenu
        heldReason: rowMenu.heldReason
        runningWhy: rowMenu.runningWhy
        onWaitEnded: rowMenu.waitEnded()
        onClosed: rowMenu.dismissed()
        // The row's double-click, in the words and seat a branch another worktree holds has (`switchCommitItem`).
        AppMenuItem {
            id: carriedOpenRow
            //: The row that leads to the worktree whose uncommitted changes these are; its name follows it — the folder's,
            //: or the branch the repository's own worktree has out.
            text: qsTr("Open")
            nameMark: rowMenu.carriedHome ? "home" : "tree"
            codeName: rowMenu.carriedName
            onTriggered: rowMenu.carriedOpenRequested(rowMenu.carriedPath)
        }
        AppMenuSeparator {}
        // Words, not a command: it lands as `stash apply` lands, but no one command a reader would type does this
        // (デザイン規約 §git 用語のコード表記).
        AppMenuItem {
            id: carriedApplyRow
            text: qsTr("Apply changes to this worktree")
            offered: rowMenu.carriedApplyOffered
            // Not "here": in this app that is the row right-clicked (デザイン規約 §別の worktree を読む).
            //: %1 is the other worktree's name — its folder, or the branch the repository's own worktree has out.
            blockedReason: rowMenu.carriedApplyOut === "there" ? qsTr("Settle its conflicts first — %1")
                                                                 .arg(rowMenu.carriedName)
                         : rowMenu.carriedApplyOut === "here" ? qsTr("Settle this worktree's conflicts first") : ""
            // The worktree in that reason wears its mark, as the door above does.
            reasonMarkWord: rowMenu.carriedApplyOut === "there" ? rowMenu.carriedName : ""
            reasonMarkKind: rowMenu.carriedHome ? "home" : "tree"
            onTriggered: rowMenu.carriedApplyRequested(rowMenu.carriedPath)
        }
    }

    AppMenu {
        id: commitMenu
        heldReason: rowMenu.heldReason
        runningWhy: rowMenu.runningWhy
        onWaitEnded: rowMenu.waitEnded()
        onClosed: rowMenu.dismissed()
        // The first level holds the rows used often, in order of widening reach, the named branch's delete table last;
        // what is used rarely goes in the cards under them, GITHUB at the foot (デザイン規約 §メニュー).
        //
        // Words, not a command: it opens a box, and what runs depends on what is typed. It shares a group with
        // `switch`, being its create form (デザイン規約 §ブランチ・コミットへの移動).
        AppMenuItem {
            id: branchHereCommitItem
            text: Words.createBranchHere
            offered: rowMenu.canBranchHere
            blockedReason: rowMenu.startWhy
            onTriggered: rowMenu.branchHereRequested(rowMenu.oid)
        }
        // The name is the chip's, so the row does not repeat it; landing is by name, since the commit itself would
        // leave HEAD on no branch.
        AppMenuItem {
            id: switchCommitItem
            // A branch another worktree holds leads to that worktree, in the same words and seat as the sidebar's row
            // (デザイン規約 §メニュー).
            code: rowMenu.heldLeaf === "" ? "switch" : ""
            //: The row that leads to the worktree holding this branch; its name follows it — the folder's, or the branch
            //: the repository's own worktree has out.
            text: rowMenu.heldLeaf === "" ? "" : qsTr("Open")
            nameMark: rowMenu.heldLeaf === "" ? "" : rowMenu.heldHome ? "home" : "tree"
            codeName: rowMenu.heldLeaf
            offered: rowMenu.canSwitch
            // Not greyed: where the move would ask first, `asks` marks the row before the press (as `RefRowMenu`).
            blockedReason: ""
            asks: rowMenu.switchAsks
            onTriggered: rowMenu.switchRequested(rowMenu.targetKind, rowMenu.targetName)
        }
        AppMenuSeparator {}
        AppMenuItem {
            code: "cherry-pick"
            offered: rowMenu.canSequence
            blockedReason: rowMenu.startWhy
            onTriggered: rowMenu.cherryPickRequested(rowMenu.oid)
        }
        // Both cherry-pick and revert only add a commit, so both are a plain click.
        AppMenuItem {
            code: "revert"
            offered: rowMenu.canSequence
            blockedReason: rowMenu.startWhy
            onTriggered: rowMenu.revertRequested(rowMenu.oid)
        }
        AppMenuSeparator {}
        // The message is edited in the details pane's boxes, where the click that opens this menu has already put
        // it.
        AppMenuItem {
            code: "squash"
            //: Follows the `squash` chip: "squash into parent".
            text: qsTr("into parent")
            note: rowMenu.publishedLive ? qsTr("already pushed") : ""
            offered: rowMenu.canEditHistory
            blockedReason: rowMenu.startWhy
            onTriggered: rowMenu.squashRequested(rowMenu.oid)
        }
        // Held while this branch alone holds its tip; a plain click once something else does, since the commits then
        // stay drawn and a cherry-pick brings them back (デザイン規約 §回数押し).
        AppMenuItem {
            id: dropCommitItem
            code: "drop"
            note: rowMenu.publishedLive ? qsTr("already pushed") : ""
            offered: rowMenu.canEditHistory
            blockedReason: rowMenu.startWhy
            holdPresses: rowMenu.tipHeldElsewhere ? 0 : Metrics.holdPressesWarned
            onTriggered: rowMenu.dropRequested(rowMenu.oid)
            onHeld: {
                dropCommitItem.closeMenu()
                rowMenu.dropRequested(rowMenu.oid)
            }
        }
        // The clicked commit is the oldest one included; the plan shows the base it lands on as its own last row
        // (デザイン規約 §フル interactive rebase).
        AppMenuItem {
            code: "rebase --interactive"
            //: Follows the `rebase --interactive` chip: "from here up".
            text: qsTr("from here up")
            note: rowMenu.publishedLive ? Words.rewritesPushed : ""
            offered: rowMenu.canEditHistory
            blockedReason: rowMenu.startWhy
            onTriggered: rowMenu.planRequested(rowMenu.oid)
        }
        AppMenuSeparator {}
        AppMenuItem {
            code: "merge"
            refSentence: Words.intoBranch
            refName: rowMenu.branch
            refMark: "branch"
            offered: rowMenu.canIntegrate
            blockedReason: rowMenu.startWhy
            onTriggered: rowMenu.mergeRequested(rowMenu.integrateRef)
        }
        AppMenuItem {
            code: "rebase"
            //: Follows the `rebase` chip: "rebase main onto it".
            refSentence: qsTr("%1 onto it")
            refName: rowMenu.branch
            refMark: "branch"
            note: rowMenu.publishedLive ? Words.rewritesPushed : ""
            offered: rowMenu.canIntegrate
            blockedReason: rowMenu.startWhy
            onTriggered: rowMenu.rebaseRequested(rowMenu.integrateRef)
        }
        AppMenu {
            id: resetMenu
            // A stopped operation holds every row inside as the held doors do; the title stays live to show them.
            heldReason: rowMenu.heldReason !== "" ? rowMenu.heldReason : rowMenu.startWhy
            runningWhy: rowMenu.runningWhy
            onWaitEnded: rowMenu.waitEnded()
            titleCode: "reset"
            //: Follows the `reset` chip: "reset main here".
            titleSentence: qsTr("%1 here")
            // Never shown without a branch (`canMoveBranch`, offers::commit_menu); the fallback only keeps the
            // sentence whole.
            titleRef: rowMenu.branch !== "" ? rowMenu.branch : qsTr("the branch")
            titleRefMark: rowMenu.branch !== "" ? "branch" : ""
            applies: rowMenu.canMoveBranch
            AppMenuItem {
                code: "--soft"
                text: qsTr("Keep everything, staged")
                onTriggered: rowMenu.resetRequested("soft")
            }
            AppMenuItem {
                code: "--mixed"
                text: qsTr("Keep everything, unstaged")
                onTriggered: rowMenu.resetRequested("mixed")
            }
            // Held. The sentence is about the commits; the note says the tracked uncommitted changes go too, which
            // nothing else on screen says (規約 §ブランチを過去のコミットへ戻す).
            AppMenuItem {
                id: hardResetItem
                code: "--hard"
                text: qsTr("Discard everything after it")
                // Two words and no count: `%n … file(s)` would show `(s)` verbatim, since no translation loads.
                note: rowMenu.hardResetTakes > 0 && !rowMenu.opStanding ? qsTr("uncommitted too") : ""
                holdPresses: Metrics.holdPressesWarned
                // The commits come back from the branch's reflog and the uncommitted changes from the discard
                // record's copy — unless a file in them is one the copy cannot take (デザイン規約 §回数押し の色の表).
                holdTone: rowMenu.hardResetNotCopied ? Theme.danger : Theme.warning
                onHeld: {
                    // This card and the menu it hangs off go together (`AppMenuItem.closeMenu`).
                    hardResetItem.closeMenu()
                    rowMenu.resetRequested("hard")
                }
            }
        }
        AppMenuSeparator {}
        // The sidebar's `pull` row, in the same seat: a group of its own with the push, since both act on a branch, not
        // this commit (デザイン規約 §取り込んで合流させる).
        AppMenuItem {
            id: pullCommitItem
            code: "pull"
            // The branch that moves, in its own colour: the worktree's, or the chip's where the pull runs aside.
            refSentence: Words.intoBranch
            refName: rowMenu.pullRuns === "aside" ? rowMenu.targetName : rowMenu.branch
            refTint: rowMenu.pullRuns === "aside" ? Theme.textPrimary : Theme.textLink
            refMark: "branch"
            offered: rowMenu.pullRuns !== ""
            blockedReason: Words.pullOut(rowMenu.pullOut, rowMenu.heldLeaf, rowMenu.pullUpstream)
            reasonMarkWord: rowMenu.pullOut === "held" ? rowMenu.heldLeaf : ""
            reasonMarkKind: rowMenu.heldHome ? "home" : "tree"
            reasonMarks: rowMenu.pullOut === "own"
                         ? Words.listMarks(pullCommitItem.blockedReason, rowMenu.pullUpstream, "remote") : []
            onTriggered: rowMenu.pullRequested(rowMenu.pullRuns === "aside" ? rowMenu.targetName : "")
        }
        // The sidebar's `push` row, in the same seat: the chip's branch sent, on the BRANCH card's answers.
        BranchPushRow {
            id: pushCommitRow
            card: branchCommitMenu
        }
        AppMenuSeparator {}
        // What takes the chip's name away, in the sidebar's seat — the last group above the cards: its delete table,
        // on the BRANCH card's answers. A row that draws no branch has none of the three.
        BranchDeleteRow {
            id: deleteCommitRow
            card: branchCommitMenu
        }
        BranchRemoteDeleteRow {
            id: remoteDeleteCommitRow
            card: branchCommitMenu
        }
        BranchBothDeleteRow {
            id: bothDeleteCommitRow
            card: branchCommitMenu
        }
        AppMenuSeparator {}
        // The same card the sidebar's row opens (デザイン規約 §メニュー); a row that draws no branch hands it "" and the
        // card goes with its row.
        RefBranchMenu {
            id: branchCommitMenu
            heldReason: rowMenu.heldReason
            runningWhy: rowMenu.runningWhy
            onWaitEnded: rowMenu.waitEnded()
            refusedDelete: rowMenu.refusedDelete
            landedDelete: rowMenu.landedDelete
            checkedBranch: rowMenu.checkedBranch
            checkedMerged: rowMenu.checkedMerged
            onDeleteRequested: (kind, id, name, oidHex) => rowMenu.deleteRequested(kind, id, name, oidHex)
            onUpstreamRequested: (branch, counterpart, unsettable) =>
                rowMenu.upstreamRequested(branch, counterpart, unsettable)
            onUnsetUpstreamRequested: branch => rowMenu.unsetUpstreamRequested(branch)
            onPushRequested: (branch, force, lease) => rowMenu.pushBranchRequested(branch, force, lease)
            onPublishRequested: (branch, upstream) => rowMenu.publishRequested(branch, upstream)
            onCheckDeleteRequested: branch => rowMenu.checkDeleteRequested(branch)
            onForceDeleteRequested: branch => rowMenu.forceDeleteRequested(branch)
            onDeleteRemoteRequested: (remoteRef, expect) => rowMenu.deleteRemoteRequested(remoteRef, expect)
            onDeleteEverywhereRequested: (branch, remoteRef, forced, expect) =>
                rowMenu.deleteEverywhereRequested(branch, remoteRef, forced, expect)
        }
        AppMenuSeparator {}
        // The same WORKTREE card as the sidebar's, above the TAG card: `Create worktree here…` always, the chip's
        // branch out in a worktree of its own, and the worktree the chip names or the one holding the chip's branch
        // taken away (デザイン規約 §メニュー の入れ子).
        RefWorktreeMenu {
            id: worktreeCommitMenu
            heldReason: rowMenu.heldReason
            runningWhy: rowMenu.runningWhy
            onWaitEnded: rowMenu.waitEnded()
            onWorktreeHereRequested: oidHex => rowMenu.worktreeHereRequested(oidHex)
            onWorktreeAddRequested: (mode, branch, start, path, name) =>
                rowMenu.worktreeAddRequested(mode, branch, start, path, name)
            onRemoveRequested: (path, name, holds) => rowMenu.removeWorktreeRequested(path, name, holds)
        }
        AppMenuSeparator {}
        // The same TAG card as the sidebar's: `Create tag here…` always, and the tag's own rows where the row draws
        // one (デザイン規約 §メニュー).
        RefTagMenu {
            id: tagCommitMenu
            heldReason: rowMenu.heldReason
            runningWhy: rowMenu.runningWhy
            onWaitEnded: rowMenu.waitEnded()
            canBranchHere: rowMenu.canBranchHere
            makeWhy: rowMenu.startWhy
            onTagHereRequested: oidHex => rowMenu.tagHereRequested(oidHex)
            onPushTagRequested: (remote, tag, lease) => rowMenu.pushTagRequested(remote, tag, lease)
            onDeleteTagRequested: tag => rowMenu.deleteTagRequested(tag)
            onDeleteRemoteTagRequested: (remote, tag, onlyThere, expect) =>
                rowMenu.deleteRemoteTagRequested(remote, tag, onlyThere, expect)
            onDeleteTagEverywhereRequested: (tag, remote, expect) =>
                rowMenu.deleteTagEverywhereRequested(tag, remote, expect)
        }
        AppMenuSeparator {}
        // GitHub's pages for this commit and the name the chip draws, last: they leave the window, every card above
        // acts in it (デザイン規約 §GitHub のページへのリンク).
        GitHubMenu {
            id: githubCommitMenu
            onCopyRequested: text => rowMenu.copyRequested(text)
        }
    }

    // A choice of two commits, right-clicked inside it: what takes the pair — every row above takes one commit, and a
    // right-click outside the choice takes it down to that row (デザイン規約 §複数のコミットを選ぶ). Nothing here starts
    // git, so it neither waits nor stands again.
    AppMenu {
        id: choiceMenu
        onClosed: rowMenu.dismissed()
        GitHubMenu {
            id: githubChoiceMenu
            onCopyRequested: text => rowMenu.copyRequested(text)
        }
    }
}
