import QtQuick
import platitude.ui

// The delete table's first row — the branch itself, or a remote branch's reading (デザイン規約 §左メニューの所作 の
// 削除の表). A row of the BRANCH card, standing on its frozen answers (`RefBranchMenu.state`), which also turn
// this row into the held `-D` when git refuses while the card stands (`forceDeleteBranch`).
AppMenuItem {
    id: deleteRow

    required property RefBranchMenu card
    readonly property QtObject facts: deleteRow.card.state

    // The delete names its target: during the hold, what is about to go has to be readable on the row itself
    // (デザイン規約 §メニュー「例外は削除行だけ」).
    readonly property bool remoteRow: deleteRow.facts.kind === "remote"
    readonly property bool branchRow: deleteRow.facts.kind === "branch"
    // git refused `--delete` while this menu stood, or the check at open came back unmerged (off the rows, else
    // git's echo). Only `"no"` dresses the row: `"unknown"` is a read that fell over, offered plain.
    readonly property bool refusedRow:
        deleteRow.branchRow
        && (deleteRow.card.forceDeleteBranch === deleteRow.facts.refId
            || (deleteRow.facts.deleteAnswered ? !deleteRow.facts.deleteMerged
                : (deleteRow.card.checkedBranch === deleteRow.facts.refId
                   && deleteRow.card.checkedMerged === "no")))
    readonly property bool heldRow: deleteRow.remoteRow || deleteRow.refusedRow
    /// `refusedRow` as latched at the first press: on a branch row the warning's count (`AppMenuItem.armedPresses`)
    /// means refused, the plain delete's means not. Read live, git's answer to the last attempt would re-word the
    /// row between presses (デザイン規約 §回数押し).
    readonly property bool shownRefused:
        deleteRow.branchRow && deleteRow.armedPresses >= Metrics.holdPressesWarned

    code: deleteRow.shownRefused ? "branch -D"
        : deleteRow.branchRow ? "branch --delete"
        : "push --delete"
    // Data: untranslated, and it does not widen the menu past `labelColW` (`codeName`) — behind its kind's mark
    // (デザイン規約 §ref の種別「名前の印」).
    nameMark: deleteRow.remoteRow ? "remote" : "branch"
    codeName: deleteRow.facts.refId
    note: deleteRow.shownRefused ? qsTr("not merged") : ""
    // On a branch the delete forms are a fixed table whose rows grey out rather than go
    // (デザイン規約 §メニュー「例外はブランチの削除の表」). The other kinds keep the assembled rule.
    offered: deleteRow.branchRow || (deleteRow.heldRow && deleteRow.facts.canDelete)
    blockedReason: !deleteRow.branchRow || deleteRow.facts.canDelete ? ""
                 : deleteRow.facts.onCurrentBranch ? deleteRow.card.deleteBlockedOnCurrent
                 : deleteRow.card.blockedByWorktree
    // Asked of the sentence this row shows, not of the state: a row blocked for another reason names no worktree.
    reasonMarkWord: deleteRow.blockedReason === deleteRow.card.blockedByWorktree ? deleteRow.facts.holderLeaf : ""
    reasonMarkKind: deleteRow.facts.holderHome ? "home" : "tree"
    // The plain delete is a delete too: two presses, in the plain ink (デザイン規約 §回数押し). Refused, or reaching
    // past this machine, it is the warning's three.
    holdPresses: deleteRow.heldRow ? Metrics.holdPressesWarned : Metrics.holdPressesPlain
    // `warning` both ways when held: a remote's name stands back up here, and a local branch's tip and upstream
    // come back from the discard record (デザイン規約 §回数押し の色の表). Off the latched count, as the words are.
    holdTone: deleteRow.armedPresses >= Metrics.holdPressesWarned ? Theme.warning : Theme.textPrimary
    // At the last press, so `shownRefused` is the count these presses were given: an unmerged answer landing on the
    // floor turns no plain delete into `-D`.
    onHeld: {
        // A branch's plain delete keeps the menu up: git's answer lands on this row, which a refusal turns into
        // `branch -D` (`AppMenuItem.armedPresses` reopens with the warning's count).
        if (deleteRow.branchRow && !deleteRow.shownRefused) {
            deleteRow.card.deleteRequested(deleteRow.facts.kind, deleteRow.facts.refId, deleteRow.facts.refName,
                                           deleteRow.facts.refOid)
            return
        }
        deleteRow.closeMenu()
        if (deleteRow.remoteRow) {
            // Leased to the commit this row showed (`remote::delete_remote_branch`).
            deleteRow.card.deleteRemoteRequested(deleteRow.facts.refId, deleteRow.facts.refOid)
        } else {
            deleteRow.card.forceDeleteRequested(deleteRow.facts.refId)
        }
    }
}
