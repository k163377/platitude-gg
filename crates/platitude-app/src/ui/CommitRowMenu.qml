pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

// The right-click on a graph row: one menu for a commit, another for a stash. A stash is a commit git keeps off to one
// side of every branch, so none of the commit menu's rows land on it and it gets its own (デザイン規約 §グラフ行の右クリック).
//
// What either of them offers is decided as it opens and handed in here — the conditions are live (a timer fetch alone
// moves `busyCount`), and a row that appears or vanishes under the pointer is a row clicked by accident (デザイン規約 §メニュー).
//
// An `Item` because a menu measures the window through the item it was declared under (`AppMenu.ownerItem`); it draws
// nothing itself.
Item {
    id: rowMenu

    required property RepoTab repoTab
    /// What the branch card needs to work its own answers out — the same models the ref menu hands it, so the two
    /// entrances to a row cannot drift (RefBranchMenu).
    required property WorkTreeModel workTree
    required property NavSectionModel branchesModel
    required property NavSectionModel worktreesModel

    /// The branch this row carries, if it carries one — what the card is about. Empty on a row nothing points at,
    /// which takes the card off the menu.
    property string rowBranchKind: ""
    property string rowBranch: ""
    /// The branch every one of these rows makes its sentence about (規約 §履歴を合流させる).
    required property string branch

    /// The commit the menu was opened on, and the stash it is ("" on an ordinary commit).
    required property string oid
    required property string stashRef
    /// Whether a remote already has that commit. Rewriting it is not asked about — nothing here leaves the machine —
    /// but デザイン規約 「push 済みの範囲は尋ねずに言う」 wants it said, so the squash row carries a tag the way the amend editor does. The
    /// answer lands a frame after the menu opens.
    required property bool published
    /// What these menus offer. `canSequence` is the pair that only add a commit, and so ask less of the repository than
    /// the rest.
    required property bool canSequence
    required property bool canIntegrate
    required property bool canEditHistory
    required property bool canMoveBranch
    required property bool canBranchHere
    required property bool stashCanWrite

    /// The rows the page owns the answer to: they rewrite history or move the selection, which is the page's to do
    /// (`RepoPage`).
    signal branchHereRequested(string oidHex)
    signal tagHereRequested(string oidHex)
    signal squashRequested(string oidHex)
    signal dropRequested(string oidHex)
    signal resetRequested(string mode)
    signal applyStashRequested(string selector)
    signal popStashRequested(string selector)
    signal dropStashRequested(string selector)
    /// The branch card's two, passed straight up: the delete git may still refuse is the page's question, and the row
    /// taken off the list ahead of the answer is the page's list (デザイン規約 §消す操作は先に画面から消す).
    signal deleteRequested(string kind, string id, string name, string oidHex)
    signal deleting(string kind, string id)

    /// Either card is on screen. A plain property rather than an alias — `visible` read from another file comes back
    /// stale (`RefusalBadge`) — and what the page reads to know that hover is behind a menu now (デザイン規約 §メニュー).
    readonly property bool showing: commitMenu.visible || stashMenu.visible

    /// The automation's handles into these rows, an automation-only exposure the same as `GraphPane.view` is
    /// (app-ui.md).
    readonly property alias menu: commitMenu
    readonly property alias branchHereItem: branchHereCommitItem
    readonly property alias tagHereItem: tagHereCommitItem
    readonly property alias dropItem: dropCommitItem
    readonly property alias stashDropItem: stashDeleteItem
    readonly property alias resetSubmenu: resetMenu
    readonly property alias hardResetRow: hardResetItem
    /// The two cards the rows above hang behind (`AppMenu.openSub`).
    readonly property alias branchCard: branchCommitMenu
    readonly property alias tagCard: tagCommitMenu

    anchors.fill: parent

    function offerStash() {
        stashMenu.offer()
    }
    function offerCommit() {
        // The branch card is asked before the menu opens, the way every other row's answer is settled first — its own
        // `applies` is what decides whether the card's row is there to count (`AppMenu.offeredRows`).
        branchCommitMenu.offerOn(rowMenu.rowBranchKind, rowMenu.rowBranch, rowMenu.rowBranch, rowMenu.oid)
        commitMenu.offer()
    }

    AppMenu {
        id: stashMenu
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
        // Held, not asked (デザイン規約 §長押し).
        AppMenuItem {
            id: stashDeleteItem
            code: "drop"
            offered: rowMenu.stashCanWrite
            holdMs: Metrics.holdMs
            onHeld: {
                stashMenu.close()
                rowMenu.dropStashRequested(rowMenu.stashRef)
            }
        }
    }

    AppMenu {
        id: commitMenu
        // **The card holds what moves the reader** (デザイン規約 §メニュー の入れ子): a branch of one's own started here,
        // the commits replayed onto where they are, this one's place in the history rewritten, the current branch
        // brought over or taken back. What is *done to* a ref goes behind a mark at the foot.
        //
        // The rules between the groups are the ones this menu has always drawn; what changed is the order. Down the
        // card, what each row acts on widens: where the reader would stand, then the rows that only add a commit,
        // then the rows that rewrite this one, then the rows that move the branch itself.
        //
        // Words rather than a chip: no one command is what this row runs. It opens a box, and what git is finally
        // spawned with depends on what is typed into it — the ellipsis is that (`Add remote…` / `Open repository…`).
        // The one command that would fit, `switch --create`, is the spelling the ref menu's own `switch` row would
        // then share, and switching is precisely what this row does not do until a name exists (2026-08-17 ユーザー判断).
        //
        // Still no row for landing on the commit itself: that leaves HEAD on no branch (デザイン規約 §ブランチ・コミット
        // への移動). This row is the whole of what the menu offers towards standing here — with a branch under it.
        AppMenuItem {
            id: branchHereCommitItem
            text: qsTr("Create branch here…")
            offered: rowMenu.canBranchHere
            onTriggered: rowMenu.branchHereRequested(rowMenu.oid)
        }
        AppMenuSeparator {}
        AppMenuItem {
            code: "cherry-pick"
            offered: rowMenu.canSequence
            onTriggered: rowMenu.repoTab.cherryPick(rowMenu.oid)
        }
        // Both cherry-pick and revert only add a commit, so neither is asked about or held.
        AppMenuItem {
            code: "revert"
            offered: rowMenu.canSequence
            onTriggered: rowMenu.repoTab.revert(rowMenu.oid)
        }
        AppMenuSeparator {}
        // No entry for editing the message: the click that opens this menu already puts the message in the details
        // pane's boxes.
        AppMenuItem {
            code: "squash"
            //: Follows the `squash` chip: "squash into parent".
            text: qsTr("into parent")
            // Said, not asked (要望: rewriting a pushed commit shows a warning): the squash goes ahead, and this tag is
            // the warning.
            note: rowMenu.published ? qsTr("already pushed") : ""
            offered: rowMenu.canEditHistory
            onTriggered: rowMenu.squashRequested(rowMenu.oid)
        }
        // Held while this branch is the only thing holding its tip; a plain click once something else does — then the
        // replaced commits stay drawn and a cherry-pick brings any of them back (デザイン規約 §長押し). The answer is a property
        // of the branch, not of the row, so it is already in hand when the menu opens: a mark appearing later would
        // re-indent every row (`AppMenu.holdIndent`) with the hand already on its way.
        AppMenuItem {
            id: dropCommitItem
            code: "drop"
            note: rowMenu.published ? qsTr("already pushed") : ""
            offered: rowMenu.canEditHistory
            holdMs: rowMenu.repoTab.headReachedElsewhere ? 0 : Metrics.holdMs
            onTriggered: rowMenu.dropRequested(rowMenu.oid)
            onHeld: {
                commitMenu.close()
                rowMenu.dropRequested(rowMenu.oid)
            }
        }
        AppMenuSeparator {}
        AppMenuItem {
            code: "merge"
            //: Follows the `merge` chip: "merge into main".
            text: qsTr("into %1").arg(rowMenu.branch)
            offered: rowMenu.canIntegrate
            onTriggered: rowMenu.repoTab.merge(rowMenu.oid, false, false, "")
        }
        AppMenuItem {
            code: "rebase"
            //: Follows the `rebase` chip: "rebase main onto it".
            text: qsTr("%1 onto it").arg(rowMenu.branch)
            note: rowMenu.published ? qsTr("rewrites pushed commits") : ""
            offered: rowMenu.canIntegrate
            onTriggered: rowMenu.repoTab.rebase(rowMenu.oid, "", true)
        }
        AppMenu {
            id: resetMenu
            titleCode: "reset"
            //: Follows the `reset` chip: "reset main here".
            title: rowMenu.branch !== "" ? qsTr("%1 here").arg(rowMenu.branch) : qsTr("the branch here")
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
            // Held, not asked (デザイン規約 §長押し).
            AppMenuItem {
                id: hardResetItem
                code: "--hard"
                text: qsTr("Discard everything after it")
                holdMs: Metrics.holdMs
                onHeld: {
                    resetMenu.close()
                    commitMenu.close()
                    rowMenu.resetRequested("hard")
                }
            }
        }
        AppMenuSeparator {}
        // **The very card the chip on this row opens** (RefBranchMenu): a row that carries a branch is a way at that
        // branch, and a right-click on the row has to offer what a right-click on its chip offers (2026-08-26 ユーザー
        // 判断). The card works its own answers out from the name the page hands it, so the two entrances cannot
        // drift; a row that carries none is handed "" and the card goes with its row.
        RefBranchMenu {
            id: branchCommitMenu
            repoTab: rowMenu.repoTab
            workTree: rowMenu.workTree
            branchesModel: rowMenu.branchesModel
            worktreesModel: rowMenu.worktreesModel
            onDeleteRequested: (kind, id, name, oidHex) => rowMenu.deleteRequested(kind, id, name, oidHex)
            onDeleting: (kind, id) => rowMenu.deleting(kind, id)
            onCloseRequested: commitMenu.close()
        }
        AppMenuSeparator {}
        // The mark left on this commit, behind the tag's own mark. The box this row opens is the same box the row at
        // the head opens, on the same commit and on the same answer; what the two do not share is what happens next —
        // a branch is somewhere to carry on from, a tag is a mark left behind.
        AppMenu {
            id: tagCommitMenu
            titleKind: "tag"
            titleTint: Theme.refTag
            title: qsTr("TAG")
            AppMenuItem {
                id: tagHereCommitItem
                text: qsTr("Create tag here…")
                offered: rowMenu.canBranchHere
                onTriggered: rowMenu.tagHereRequested(rowMenu.oid)
            }
        }
    }
}
