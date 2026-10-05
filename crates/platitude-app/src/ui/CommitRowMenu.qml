pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The right-click on a graph row: one menu for a commit, another for a stash (デザイン規約 §グラフ行の右クリック). The
// first level acts on the row's commit; the cards at the foot act on the name the chip draws, or the name pressed in
// the chip's stacked list.
//
// Nothing here is looked up and nothing is run: answers arrive as plain values, held still from the moment the menu
// opens (`CommitMenuState`, デザイン規約 §メニュー), and presses leave as signals for `RepoPage` — which is what lets
// `tst_commitrowmenu.qml` drive the real menu.
//
// An `Item` because a menu measures the window through the item it was declared under (`AppMenu.ownerItem`).
Item {
    id: rowMenu

    /// Why every row is out while a history-replaying write holds the doors (`RepoPage.doorsHeldWhy`). Every card
    /// takes it; the rows stay and grey (`AppMenu.heldReason`).
    property string heldReason: ""

    /// The name this menu is aimed at: the one the chip draws, or the one pressed in the chip's stacked list. `kind`
    /// is `branch` / `remote` / `tag`, or `worktree` for a working copy's folder chip — its name then the copy's path —
    /// and empty on a row that draws neither (デザイン規約 §グラフ行の右クリック).
    property string targetKind: ""
    property string targetName: ""
    /// What merge / rebase are handed: the name where the row draws one — a merge by name says so in the commit it
    /// writes, and one by id does not. A copy's path is no ref, so the commit.
    readonly property string integrateRef:
        rowMenu.targetName !== "" && rowMenu.targetKind !== "worktree" ? rowMenu.targetName : rowMenu.oid
    /// The branch every one of these rows makes its sentence about (規約 §履歴を合流させる).
    required property string branch

    /// The commit the menu was opened on, and the stash it is ("" on an ordinary commit).
    required property string oid
    required property string stashRef
    /// Whether a remote already has that commit: the rewrite rows carry a tag and do not ask
    /// (デザイン規約 §長押し「ローカルの履歴書き換えはクリック」).
    required property bool published
    /// What these menus offer. `canSequence` is cherry-pick / revert, which only add a commit and so ask less.
    required property bool canSequence
    required property bool canIntegrate
    required property bool canEditHistory
    required property bool canMoveBranch
    required property bool canBranchHere
    required property bool stashCanWrite
    /// Whether anything besides this branch still reaches the tip — what decides whether the drop row is held
    /// (`CommitMenuState.menuTipHeldElsewhere`, the same answer `RebasePlanRunBar` is handed).
    required property bool tipHeldElsewhere
    /// Whether the row's name is somewhere to move to, and whether that move asks first (`offers::ref_menu`).
    required property bool canSwitch
    required property bool switchAsks
    /// …or leads to the working copy holding that name instead: that copy's folder (`RefRowMenu.heldLeaf`).
    required property string heldLeaf
    /// And whether that name has a far side to pull from — the third answer off those same rules.
    required property bool canPull
    /// Whether that press would be turned down before git did anything, which greys the row
    /// (`RefRowMenu.pullBlocked`).
    required property bool pullBlocked
    /// How many uncommitted files `--hard` takes besides the commits (`status::Counts::hard_reset_takes`); 0 on a
    /// clean tree.
    required property int hardResetTakes
    /// Whether what `--hard` takes holds a file the discard record cannot copy (`WorkTreeModel.hardResetNotCopied`),
    /// which holds the row in `danger` (デザイン規約 §長押し の色の表).
    required property bool hardResetNotCopied
    /// git's answers about the branch card's delete (refusal, landing, early check), live while the card stands —
    /// passed straight through to `RefBranchMenu`.
    property string refusedDelete: ""
    property string landedDelete: ""
    property string checkedBranch: ""
    property string checkedMerged: ""

    /// Move the working tree to the row's name, by the road a double-click takes (`RepoPage.switchToRef`).
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
    /// The branch card's two, passed straight up: the delete git may still refuse, and the upstream (`UpstreamFlow`).
    signal deleteRequested(string kind, string id, string name, string oidHex)
    signal upstreamRequested(string branch, string counterpart)
    signal cherryPickRequested(string oidHex)
    signal revertRequested(string oidHex)
    signal mergeRequested(string ref)
    signal rebaseRequested(string ref)
    signal pullRequested()
    signal checkDeleteRequested(string branch)
    signal forceDeleteRequested(string branch)
    signal deleteRemoteRequested(string remoteRef, string expect)
    signal deleteEverywhereRequested(string branch, string remoteRef, bool forced, string expect)
    signal pushTagRequested(string remote, string tag, string lease)
    signal deleteTagRequested(string tag)
    signal deleteRemoteTagRequested(string remote, string tag, bool onlyThere, string expect)
    signal deleteTagEverywhereRequested(string tag, string remote, string expect)
    /// The WORKTREE card's three, passed straight up (`RefWorktreeMenu`): the box for a new branch in a copy of its
    /// own, the row's own branch out in one, and `worktree remove`.
    signal copyHereRequested(string oidHex)
    signal copyAddRequested(string mode, string branch, string start, string path, string name)
    signal removeCopyRequested(string path, string name)
    /// The menu went away — and the stacked list it may have been standing on is the pointer's to answer for again.
    signal dismissed()

    /// Either menu is on screen — what the page reads to hide hover behind it. Read this, not `visible`, which comes
    /// back stale from another file (rules-refs/app-ui.md「メニューが立っている間、裏の hover は伏せる」).
    readonly property bool showing: commitMenu.visible || stashMenu.visible

    /// The automation's handles into these rows, an automation-only exposure the same as `GraphPane.view` is
    /// (rules-refs/app-ui.md「製品の部品はハーネスへ答える相手を丸ごと渡す」).
    readonly property alias menu: commitMenu
    readonly property alias branchHereItem: branchHereCommitItem
    readonly property alias switchItem: switchCommitItem
    readonly property alias pullItem: pullCommitItem
    readonly property alias tagHereItem: tagCommitMenu.tagHereItem
    readonly property alias dropItem: dropCommitItem
    readonly property alias stashDropItem: stashDeleteItem
    readonly property alias resetSubmenu: resetMenu
    readonly property alias hardResetRow: hardResetItem
    /// The three cards the rows above hang behind (`AppMenu.openSub`).
    readonly property alias branchCard: branchCommitMenu
    readonly property alias copyCard: copyCommitMenu
    readonly property alias tagCard: tagCommitMenu

    anchors.fill: parent

    // Each menu closes itself (a row's `dismiss()`, or `RefBranchMenu` when a delete's answer lands), so nothing here
    // closes them.
    function offerStash() {
        stashMenu.offer()
    }
    /// `facts` is what the cards stand on, read where the models are (`CommitMenuState.cardFacts`): `branch` and
    /// `tag`, each already aimed at the target or emptied because the target is the other's kind, `copy` — the
    /// working copy the target names or the one holding its branch (undefined for none) — with `copyHere` / `busy`,
    /// and `making`, the two rows that make a copy (`RefWorktreeMenu.standOn`).
    function offerCommit(facts) {
        copyCommitMenu.standOn(facts.copy, facts.copyHere, facts.busy, facts.making)
        // Every card stands on its facts before `offer()`: its `applies` decides whether its row counts
        // (`AppMenu.offeredRows`).
        const branchy = rowMenu.targetKind === "branch" || rowMenu.targetKind === "remote"
        const branchKind = branchy ? rowMenu.targetKind : ""
        const branchName = branchy ? rowMenu.targetName : ""
        branchCommitMenu.standOn(branchKind, branchName, branchName, rowMenu.oid, facts.branch)
        const tagged = rowMenu.targetKind === "tag"
        tagCommitMenu.standOn(tagged ? "tag" : "", tagged ? rowMenu.targetName : "", rowMenu.oid, facts.tag)
        commitMenu.offer()
    }

    AppMenu {
        id: stashMenu
        heldReason: rowMenu.heldReason
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
        // Held (デザイン規約 §長押し).
        AppMenuItem {
            id: stashDeleteItem
            code: "drop"
            offered: rowMenu.stashCanWrite
            holdMs: Metrics.holdMs
            onHeld: {
                stashMenu.dismiss()
                rowMenu.dropStashRequested(rowMenu.stashRef)
            }
        }
    }

    AppMenu {
        id: commitMenu
        heldReason: rowMenu.heldReason
        onClosed: rowMenu.dismissed()
        // The first level holds the rows that move the reader, in order of widening reach; what is done to a ref goes
        // in the cards at the foot (デザイン規約 §メニュー).
        //
        // Words, not a command: it opens a box, and what runs depends on what is typed. It shares a group with
        // `switch`, being its create form (デザイン規約 §ブランチ・コミットへの移動).
        AppMenuItem {
            id: branchHereCommitItem
            text: Words.createBranchHere
            offered: rowMenu.canBranchHere
            onTriggered: rowMenu.branchHereRequested(rowMenu.oid)
        }
        // The name is the chip's, so the row does not repeat it; landing is by name, since the commit itself would
        // leave HEAD on no branch.
        AppMenuItem {
            id: switchCommitItem
            // A branch another working copy holds leads to that copy, in the same words and seat as the sidebar's row
            // (デザイン規約 §メニュー).
            code: rowMenu.heldLeaf === "" ? "switch" : ""
            //: The row that leads to the working copy holding this branch; the folder's name follows it.
            text: rowMenu.heldLeaf === "" ? "" : qsTr("Open")
            nameMark: rowMenu.heldLeaf === "" ? "" : "tree"
            nameMarkTint: Theme.success
            markName: rowMenu.heldLeaf
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
            onTriggered: rowMenu.cherryPickRequested(rowMenu.oid)
        }
        // Both cherry-pick and revert only add a commit, so both are a plain click.
        AppMenuItem {
            code: "revert"
            offered: rowMenu.canSequence
            onTriggered: rowMenu.revertRequested(rowMenu.oid)
        }
        AppMenuSeparator {}
        // The message is edited in the details pane's boxes, where the click that opens this menu has already put
        // it.
        AppMenuItem {
            code: "squash"
            //: Follows the `squash` chip: "squash into parent".
            text: qsTr("into parent")
            note: rowMenu.published ? qsTr("already pushed") : ""
            offered: rowMenu.canEditHistory
            onTriggered: rowMenu.squashRequested(rowMenu.oid)
        }
        // Held while this branch alone holds its tip; a plain click once something else does, since the commits then
        // stay drawn and a cherry-pick brings them back (デザイン規約 §長押し).
        AppMenuItem {
            id: dropCommitItem
            code: "drop"
            note: rowMenu.published ? qsTr("already pushed") : ""
            offered: rowMenu.canEditHistory
            holdMs: rowMenu.tipHeldElsewhere ? 0 : Metrics.holdMs
            onTriggered: rowMenu.dropRequested(rowMenu.oid)
            onHeld: {
                commitMenu.dismiss()
                rowMenu.dropRequested(rowMenu.oid)
            }
        }
        // The clicked commit is the oldest one included; the plan shows the base it lands on as its own last row
        // (デザイン規約 §フル interactive rebase).
        AppMenuItem {
            code: "rebase --interactive"
            //: Follows the `rebase --interactive` chip: "from here up".
            text: qsTr("from here up")
            note: rowMenu.published ? Words.rewritesPushed : ""
            offered: rowMenu.canEditHistory
            onTriggered: rowMenu.planRequested(rowMenu.oid)
        }
        AppMenuSeparator {}
        AppMenuItem {
            code: "merge"
            //: Follows the `merge` chip: "merge into main".
            refSentence: qsTr("into %1")
            refName: rowMenu.branch
            offered: rowMenu.canIntegrate
            onTriggered: rowMenu.mergeRequested(rowMenu.integrateRef)
        }
        AppMenuItem {
            code: "rebase"
            //: Follows the `rebase` chip: "rebase main onto it".
            refSentence: qsTr("%1 onto it")
            refName: rowMenu.branch
            note: rowMenu.published ? Words.rewritesPushed : ""
            offered: rowMenu.canIntegrate
            onTriggered: rowMenu.rebaseRequested(rowMenu.integrateRef)
        }
        AppMenu {
            id: resetMenu
            heldReason: rowMenu.heldReason
            titleCode: "reset"
            //: Follows the `reset` chip: "reset main here".
            titleSentence: qsTr("%1 here")
            // Never shown without a branch (`canMoveBranch`, offers::commit_menu); the fallback only keeps the
            // sentence whole.
            titleRef: rowMenu.branch !== "" ? rowMenu.branch : qsTr("the branch")
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
                note: rowMenu.hardResetTakes > 0 ? qsTr("uncommitted too") : ""
                holdMs: Metrics.holdMs
                // The commits come back from the branch's reflog and the uncommitted changes from the discard
                // record's copy — unless a file in them is one the copy cannot take (デザイン規約 §長押し の色の表).
                holdTone: rowMenu.hardResetNotCopied ? Theme.danger : Theme.warning
                onHeld: {
                    // Qt's `dismiss()`: this card and the menu it hangs off go together.
                    resetMenu.dismiss()
                    rowMenu.resetRequested("hard")
                }
            }
        }
        AppMenuSeparator {}
        // The sidebar's `pull` row, in the same seat: a group of its own above the cards, since a pull acts on the
        // working tree's branch, not this commit (デザイン規約 §取り込んで合流させる).
        AppMenuItem {
            id: pullCommitItem
            code: "pull"
            offered: rowMenu.canPull
            blockedReason: rowMenu.pullBlocked ? Words.pullDiverged : ""
            onTriggered: rowMenu.pullRequested()
        }
        AppMenuSeparator {}
        // The same card the sidebar's row opens (デザイン規約 §メニュー); a row that draws no branch hands it "" and the
        // card goes with its row.
        RefBranchMenu {
            id: branchCommitMenu
            heldReason: rowMenu.heldReason
            refusedDelete: rowMenu.refusedDelete
            landedDelete: rowMenu.landedDelete
            checkedBranch: rowMenu.checkedBranch
            checkedMerged: rowMenu.checkedMerged
            onDeleteRequested: (kind, id, name, oidHex) => rowMenu.deleteRequested(kind, id, name, oidHex)
            onUpstreamRequested: (branch, counterpart) => rowMenu.upstreamRequested(branch, counterpart)
            onCheckDeleteRequested: branch => rowMenu.checkDeleteRequested(branch)
            onForceDeleteRequested: branch => rowMenu.forceDeleteRequested(branch)
            onDeleteRemoteRequested: (remoteRef, expect) => rowMenu.deleteRemoteRequested(remoteRef, expect)
            onDeleteEverywhereRequested: (branch, remoteRef, forced, expect) =>
                rowMenu.deleteEverywhereRequested(branch, remoteRef, forced, expect)
        }
        AppMenuSeparator {}
        // The same WORKTREE card as the sidebar's, above the TAG card: `Create worktree here…` always, the chip's
        // branch out in a copy of its own, and the copy the chip names or the one holding the chip's branch taken away
        // (デザイン規約 §メニュー の入れ子).
        RefWorktreeMenu {
            id: copyCommitMenu
            heldReason: rowMenu.heldReason
            onCopyHereRequested: oidHex => rowMenu.copyHereRequested(oidHex)
            onCopyAddRequested: (mode, branch, start, path, name) =>
                rowMenu.copyAddRequested(mode, branch, start, path, name)
            onRemoveRequested: (path, name) => rowMenu.removeCopyRequested(path, name)
        }
        AppMenuSeparator {}
        // The same TAG card as the sidebar's: `Create tag here…` always, and the tag's own rows where the row draws
        // one (デザイン規約 §メニュー).
        RefTagMenu {
            id: tagCommitMenu
            heldReason: rowMenu.heldReason
            canBranchHere: rowMenu.canBranchHere
            onTagHereRequested: oidHex => rowMenu.tagHereRequested(oidHex)
            onPushTagRequested: (remote, tag, lease) => rowMenu.pushTagRequested(remote, tag, lease)
            onDeleteTagRequested: tag => rowMenu.deleteTagRequested(tag)
            onDeleteRemoteTagRequested: (remote, tag, onlyThere, expect) =>
                rowMenu.deleteRemoteTagRequested(remote, tag, onlyThere, expect)
            onDeleteTagEverywhereRequested: (tag, remote, expect) =>
                rowMenu.deleteTagEverywhereRequested(tag, remote, expect)
        }
    }
}
