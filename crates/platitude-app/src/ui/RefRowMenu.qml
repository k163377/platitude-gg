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
    /// Which remote reading a branch speaks for (`upstreamOf`).
    required property NavSectionModel branchesModel
    /// Which other working copy has a branch checked out (`worktreeHolding`) — the list lives in this section alone.
    required property NavSectionModel worktreesModel

    /// The row the menu stands on. `refName` is what the row shows, `refId` what git knows it by (they differ for a
    /// stash: a message and a selector).
    property string kind: ""
    property string refName: ""
    property string refId: ""
    property string refOid: ""
    /// The branch git has just refused to delete, while the menu that asked is still standing.
    property string forceDeleteBranch: ""

    /// What this menu offers, decided as it opens (see the note above).
    property bool canSwitch: false
    /// Whether that row will raise a question rather than move — an operation standing, files still waiting on a
    /// decision, or the branch out in another working copy. Worn as the `!` in the row's mark seat, so the press is
    /// read for what it is before it is made (デザイン規約 §進行中の操作から出る).
    property bool switchAsks: false
    property bool canBranchHere: false
    property bool canIntegrateFrom: false
    property bool canDelete: false
    /// The remote reading a branch row can also shed (`origin/main`), empty where it has none. What the remote-side
    /// delete rows name.
    property string remoteCounterpart: ""
    property bool canDeleteRemote: false
    /// The other working copy holding this row's local branch, empty when none does. **git refuses both `switch` and
    /// `branch --delete` for a branch another worktree has out** (実測), so this is read and not the lock — a lock
    /// stops `worktree remove` and `worktree move`, which is a different question the WORKTREES row answers. The
    /// delete rows go out on it; `switch` presses through to the question that opens that copy instead
    /// (`RepoPage.askOpenHolder`).
    property string heldByWorktree: ""
    /// The folder that copy is listed under in WORKTREES. The whole path is what git answers with and is the only
    /// unambiguous form, but nobody reads a tooltip that wide — and the list the reader goes to next shows the leaf.
    readonly property string heldByWorktreeName: {
        const at = refRowMenu.heldByWorktree.replace(/\\/g, "/").lastIndexOf("/")
        return at < 0 ? refRowMenu.heldByWorktree : refRowMenu.heldByWorktree.substring(at + 1)
    }
    /// Why the delete table's rows are out — decided as the menu opens, worn as the rows' `blockedReason` (デザイン規約 §無効).
    property bool onCurrentBranch: false
    readonly property string deleteBlockedOnCurrent:
        qsTr("Switch away first — this is the branch you are on")
    readonly property string deleteBlockedWhileBusy:
        qsTr("Another git command is still running")
    // Why git keeps a branch to one working copy is the causal half, and the tooltip rule drops it (デザイン規約 §hover の
    // ツールチップ) — what is left is the state that blocks the row and the one thing the menu cannot show: where.
    //: %1 is the folder of the other working copy that has this branch checked out.
    readonly property string blockedByWorktree:
        qsTr("Checked out in another working copy — %1").arg(refRowMenu.heldByWorktreeName)
    // **The delete rows are the only ones this blocks now.** `switch` used to be greyed here too — for a branch
    // another copy holds and for a tree with unmerged files — and both of those press through to a question instead
    // (デザイン規約 §進行中の操作から出る): a row that cannot be pressed says why only on hover, and the reader who
    // reached for it is the one who needs to read it (2026-08-22 ユーザー判断).

    // ---- bringing two lines of history together --------------------
    /// The live condition the row above is read off as the menu opens.
    readonly property bool integrateAllowed:
        refRowMenu.repoTab.state === "open" && refRowMenu.repoTab.busyCount === 0
        && !refRowMenu.workTree.detached && refRowMenu.workTree.branch !== ""
        && refRowMenu.workTree.opText === ""
        && refRowMenu.refId !== ""
        && refRowMenu.refId !== refRowMenu.workTree.branch
    /// The commits a rebase onto this row would rewrite. Asked as the menu opens — the answer is a whole git call away,
    /// and it lands in the page's one answer slot (`RepoPage`).
    readonly property string rebaseRange:
        (refRowMenu.kind === "branch" || refRowMenu.kind === "remote")
        && refRowMenu.refId !== "" ? refRowMenu.refId + "..HEAD" : ""
    property bool rebasePublished: false

    /// Whether the card is on screen, and whether it has finished opening: the folded list asks the first, the chip's
    /// stacked list asks the second. Plain properties rather than aliases — `visible` read from another file comes back
    /// stale (`RefusalBadge`).
    readonly property bool showing: refMenu.visible
    readonly property bool opened: refMenu.opened

    /// The automation's handles into these rows, an automation-only exposure the same as `GraphPane.view` is
    /// (app-ui.md).
    readonly property alias menu: refMenu
    readonly property alias branchHereItem: refBranchHereItem
    readonly property alias deleteItem: refDeleteItem
    readonly property alias switchItem: refSwitchItem

    /// What the page answers for: moving the working tree, the delete git may still refuse, and the stash drop that two
    /// menus share.
    signal switchRequested(string kindLetter, string name)
    /// A new branch on whatever commit this row stands on — the page owns where the box for its name opens, which is
    /// wherever the menu was opened from.
    signal branchHereRequested(string oidHex)
    signal deleteRequested(string kind, string id, string name, string oidHex)
    signal dropStashRequested(string selector)
    /// A ref this menu has just asked git to delete, so the window can show it as gone while the write is out
    /// (デザイン規約 §消す操作は先に画面から消す). One per ref — `Delete both` names two. `kind` is `branch` / `remote` /
    /// `tag`, `id` what git knows it by. Raised beside the write rather than instead of it: the rows the page owns
    /// (`deleteRequested`, `dropStashRequested`) take themselves away where they are answered.
    signal deleting(string kind, string id)
    /// The menu went away — with it goes a refused delete's offer, and the stacked list it may have been standing on is
    /// the pointer's to answer for again.
    signal dismissed()

    anchors.fill: parent

    /// Opens on that ref, deciding there and then what it offers. Says whether it opened at all: a ref with nothing to
    /// offer — the current branch met as a chip — falls back to the row's own menu.
    function offerOn(kind, name, full, oidHex) {
        refRowMenu.kind = kind
        refRowMenu.refName = name
        refRowMenu.refId = full
        refRowMenu.refOid = oidHex
        refRowMenu.forceDeleteBranch = ""
        refRowMenu.rebasePublished = false
        // A remote row lands on the local branch of the same name, so it is that one another copy can be holding.
        refRowMenu.heldByWorktree =
            kind === "branch" ? refRowMenu.worktreesModel.worktreeHolding(full)
            : kind === "remote" ? refRowMenu.worktreesModel.worktreeHolding(
                                      refRowMenu.repoTab.localNameFor(full))
                                : ""
        // **Held elsewhere keeps its row.** Everywhere else in this menu a row that cannot be chosen is left away
        // (§メニュー), and the exception is the same one the delete table gets: the row is the only place the reason can
        // be read, and a `switch` that quietly stopped being offered reads as a menu that lost it (2026-08-21 ユーザー判断).
        refRowMenu.canSwitch = (kind === "branch" || kind === "remote")
            && full !== refRowMenu.workTree.branch
        refRowMenu.switchAsks =
            refRowMenu.heldByWorktree !== ""
            || refRowMenu.workTree.opText !== ""
            || refRowMenu.workTree.conflictCount > 0
        // Every ref that names a commit can have a new branch started on it — the current branch included, which is
        // where one is most often started. A stash is the exception: it is nobody's history to carry on from
        // (デザイン規約 §グラフ行の右クリック).
        refRowMenu.canBranchHere =
            kind !== "stash" && oidHex !== "" && refRowMenu.repoTab.busyCount === 0
            && refRowMenu.workTree.opText === ""
        refRowMenu.canIntegrateFrom = refRowMenu.integrateAllowed
        // git refuses to delete the branch the working tree is on — or the one any other working copy is on; its
        // remote reading can still be deleted in either case.
        refRowMenu.canDelete =
            refRowMenu.repoTab.busyCount === 0
            && !(kind === "branch"
                 && (full === refRowMenu.workTree.branch || refRowMenu.heldByWorktree !== ""))
        refRowMenu.remoteCounterpart = kind === "branch" ? refRowMenu.branchesModel.upstreamOf(full) : ""
        refRowMenu.canDeleteRemote = refRowMenu.repoTab.busyCount === 0 && refRowMenu.remoteCounterpart !== ""
        refRowMenu.onCurrentBranch = kind === "branch" && full === refRowMenu.workTree.branch
        if (refRowMenu.repoTab.state === "open" && refRowMenu.rebaseRange !== "")
            refRowMenu.repoTab.checkPublish(refRowMenu.rebaseRange)
        // Whether the everyday delete would be refused, asked as the menu opens: the unmerged answer usually lands
        // before the pointer does, and the delete row wears `-D` from the start instead of only after a refused click
        // (§左メニューの所作). The chip column is settled at open, so the swap moves no other row.
        if (refRowMenu.repoTab.state === "open" && kind === "branch" && refRowMenu.canDelete)
            refRowMenu.repoTab.checkBranchDelete(full)
        return refMenu.offer()
    }

    /// A write that landed has nothing left for the menu to catch.
    function close() {
        refMenu.close()
    }

    /// Held, not asked: git refuses nothing here — the branch is on the far side, so no `-d` can weigh what it holds —
    /// and the hold stands in for that refusal (デザイン規約 §リモートブランチを消す).
    function deleteRemoteNow(remoteRef) {
        const cut = remoteRef.indexOf("/")
        if (cut < 0)
            return
        refRowMenu.deleting("remote", remoteRef)
        refRowMenu.repoTab.deleteRemoteBranch(remoteRef.substring(0, cut), remoteRef.substring(cut + 1))
    }

    AppMenu {
        id: refMenu
        // Walking away from a refused delete takes the offer with it.
        onClosed: {
            refRowMenu.forceDeleteBranch = ""
            refRowMenu.dismissed()
        }
        // A branch of one's own, started where this row stands. Ahead of everything and behind a rule of its own —
        // deliberately not beside `switch`, which is where it would read as a variant of moving onto what is already
        // there; nothing here moves anywhere until a name has been typed (2026-08-17 ユーザー判断). Same seat, same words
        // as the commit menu's row (デザイン規約 §メニュー: 入口が違っても同じ操作は同じ文).
        AppMenuItem {
            id: refBranchHereItem
            text: qsTr("Create branch here…")
            offered: refRowMenu.canBranchHere
            onTriggered: refRowMenu.branchHereRequested(refRowMenu.refOid)
        }
        AppMenuSeparator {}
        AppMenuItem {
            id: refSwitchItem
            code: "switch"
            offered: refRowMenu.canSwitch
            // Never blocked: everything that stands in a move's way is answered by the question the press raises — and
            // the mark says that a question is what this press raises.
            blockedReason: ""
            asks: refRowMenu.switchAsks
            // Through the chips' dispatcher: a remote branch whose local one already exists cannot simply be created.
            onTriggered: refRowMenu.switchRequested(refRowMenu.kind === "remote" ? "R" : "L", refRowMenu.refId)
        }
        AppMenuItem {
            code: "merge"
            //: Follows the `merge` chip: "merge into main".
            text: qsTr("into %1").arg(refRowMenu.workTree.branch)
            offered: (refRowMenu.kind === "branch"
                      || refRowMenu.kind === "remote"
                      || refRowMenu.kind === "tag")
                     && refRowMenu.canIntegrateFrom
            onTriggered: refRowMenu.repoTab.merge(refRowMenu.refId, false, false, "")
        }
        AppMenuItem {
            code: "rebase"
            //: Follows the `rebase` chip: "rebase main onto it".
            text: qsTr("%1 onto it").arg(refRowMenu.workTree.branch)
            // Deliberately not offered on a tag — that row belongs with the tag's own gestures.
            offered: (refRowMenu.kind === "branch" || refRowMenu.kind === "remote") && refRowMenu.canIntegrateFrom
            // Said, not asked (要望: rewriting a pushed commit shows a warning). Published = reachable from a
            // remote-tracking ref, only as fresh as the last fetch.
            note: refRowMenu.rebasePublished ? qsTr("rewrites pushed commits") : ""
            onTriggered: refRowMenu.repoTab.rebase(refRowMenu.refId, "", true)
        }
        AppMenuSeparator {}
        AppMenuItem {
            id: refDeleteItem
            // Alone in this menu the delete re-states its target: during the hold the name of what is about to go has
            // to be readable on the row itself (デザイン規約 §メニュー 言い直さない、の例外).
            readonly property bool stashRow: refRowMenu.kind === "stash"
            readonly property bool remoteRow: refRowMenu.kind === "remote"
            readonly property bool tagRow: refRowMenu.kind === "tag"
            readonly property bool branchRow: refRowMenu.kind === "branch"
            // git already refused `--delete` while this menu stood — or the check run at open came back unmerged, the
            // same answer a click ahead of time (§左メニューの所作).
            readonly property bool refusedRow:
                branchRow
                && (refRowMenu.forceDeleteBranch === refRowMenu.refId
                    || (refRowMenu.repoTab.branchDeleteAsked === refRowMenu.refId
                        && !refRowMenu.repoTab.branchDeleteMerged))
            readonly property bool heldRow: stashRow || remoteRow || tagRow || refusedRow
            code: refusedRow ? "branch -D"
                : branchRow ? "branch --delete"
                : tagRow ? "tag --delete"
                : remoteRow ? "push --delete"
                : "drop"
            // The name is data, not sentence: never translated, and it does not bid for the menu's width
            // (`growsForText`).
            text: stashRow ? "" : refRowMenu.refId
            growsForText: false
            note: refusedRow ? qsTr("not merged") : ""
            // On a branch the three delete forms are a fixed table — rows that cannot be chosen stay and grey out, the
            // app-menu rule rather than the assembled-menu one (デザイン規約 §メニュー、2026-08-11 ユーザー判断): the current branch
            // keeps its rows, saying why nothing here answers. The other kinds keep the assembled rule.
            offered: branchRow || (heldRow && refRowMenu.canDelete)
            blockedReason: !branchRow || refRowMenu.canDelete ? ""
                         : refRowMenu.onCurrentBranch
                           ? refRowMenu.deleteBlockedOnCurrent
                           : refRowMenu.heldByWorktree !== ""
                             ? refRowMenu.blockedByWorktree
                             : refRowMenu.deleteBlockedWhileBusy
            holdMs: heldRow ? Metrics.holdMs : 0
            // A branch's plain delete keeps the menu up: git's answer has nowhere to land otherwise, and this row is
            // where it lands.
            staysOpen: branchRow
            // Reaching past this machine is the warning tone; throwing away what is in hand is danger (デザイン規約 §状態).
            holdTone: remoteRow ? Theme.warning : Theme.danger
            onPicked: refRowMenu.deleteRequested(refRowMenu.kind,
                                                 refRowMenu.refId,
                                                 refRowMenu.refName,
                                                 refRowMenu.refOid)
            onHeld: {
                refMenu.close()
                if (stashRow)
                    refRowMenu.dropStashRequested(refRowMenu.refId)
                else if (remoteRow)
                    refRowMenu.deleteRemoteNow(refRowMenu.refId)
                else if (tagRow) {
                    refRowMenu.deleting("tag", refRowMenu.refId)
                    refRowMenu.repoTab.deleteTag(refRowMenu.refId)
                } else {
                    refRowMenu.deleting("branch", refRowMenu.refId)
                    refRowMenu.repoTab.deleteBranch(refRowMenu.refId, true)
                }
            }
        }
        // The branch's remote reading, deleted without touching the local one — on the current branch the one delete on
        // offer at all (デザイン規約 §左メニューの所作).
        AppMenuItem {
            id: refRemoteDeleteItem
            code: "push --delete"
            text: refRowMenu.remoteCounterpart
            growsForText: false
            // In the table only while the branch has a remote reading at all: a row for a target that does not exist
            // keeps no seat (2026-08-11 ユーザー判断). Grey is for "not now" — busy — not for "no such thing".
            offered: refRowMenu.kind === "branch"
                     && refRowMenu.remoteCounterpart !== ""
            blockedReason: refRowMenu.canDeleteRemote ? "" : refRowMenu.deleteBlockedWhileBusy
            holdMs: Metrics.holdMs
            holdTone: Theme.warning
            onHeld: {
                refMenu.close()
                refRowMenu.deleteRemoteNow(refRowMenu.remoteCounterpart)
            }
        }
        // A composite of two commands is no one command, so words rather than a chip (§git 用語のコード表記 の 1:1 規則). The
        // local half runs first and a refusal stops the pair with nothing touched.
        AppMenuItem {
            id: refBothDeleteItem
            text: qsTr("Delete both")
            note: refDeleteItem.refusedRow ? qsTr("not merged") : ""
            offered: refRowMenu.kind === "branch"
                     && refRowMenu.remoteCounterpart !== ""
            blockedReason: refRowMenu.canDelete && refRowMenu.canDeleteRemote
                           ? ""
                           : refRowMenu.onCurrentBranch
                             ? refRowMenu.deleteBlockedOnCurrent
                             : refRowMenu.heldByWorktree !== ""
                               ? refRowMenu.blockedByWorktree
                               : refRowMenu.deleteBlockedWhileBusy
            holdMs: Metrics.holdMs
            // The colour of the half that decides: reaching past this machine is warning, but once the local half runs
            // as `-D` this row throws away commits that live nowhere else, and that is danger (デザイン規約 §状態).
            holdTone: refDeleteItem.refusedRow ? Theme.danger : Theme.warning
            onHeld: {
                refMenu.close()
                const c = refRowMenu.remoteCounterpart
                const cut = c.indexOf("/")
                if (cut < 0)
                    return
                // Both halves go at once: the pair is one write with one answer, so it is one thing to put back.
                refRowMenu.deleting("branch", refRowMenu.refId)
                refRowMenu.deleting("remote", c)
                refRowMenu.repoTab.deleteBranchEverywhere(
                    refRowMenu.refId, c.substring(0, cut), c.substring(cut + 1),
                    refDeleteItem.refusedRow)
            }
        }
    }
}
