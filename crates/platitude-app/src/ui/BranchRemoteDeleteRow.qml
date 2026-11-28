import QtQuick
import platitude.ui
import platitude.ui.words

// The delete table's second row: the remote reading alone — on the current branch the only delete on offer
// (デザイン規約 §左メニューの所作 の削除の表) —, and on a remote branch's own row that branch: core's one row for a
// remote branch deleted (`menu::Row::DeleteRemoteBranch`). It stands on the BRANCH card's answers
// (`RefBranchMenu.state`). Held, with no bar over the graph: there is one target and the row names it,
// and the hold stands in for the refusal git does not make (`remote::delete_remote_branch`).
AppMenuItem {
    id: remoteDeleteRow

    required property RefBranchMenu card
    readonly property QtObject facts: remoteDeleteRow.card.state
    /// The remote branch it deletes — the row's own on a remote branch's row, else the reading the branch speaks for —
    /// and the commit over there the press is leased to, as the menu went up.
    readonly property bool ownRow: remoteDeleteRow.facts.kind === "remote"
    readonly property string target: remoteDeleteRow.ownRow ? remoteDeleteRow.facts.refId
                                                            : remoteDeleteRow.facts.remoteCounterpart
    readonly property string targetOid: remoteDeleteRow.ownRow ? remoteDeleteRow.facts.refOid
                                                               : remoteDeleteRow.facts.remoteCounterpartOid

    code: "push --delete"
    nameMark: "remote"
    codeName: remoteDeleteRow.target
    // Seated only while a remote reading exists (core's row); grey is "not now" — the reading stands on another commit
    // (デザイン規約 §左メニューの所作 の削除の表). A drift greys this row and `Delete both` and does not take them
    // away (user decision — ask before changing): the name is over there, so a row gone would look as if the remote
    // lacked it, and the one place that reads "it is there too" would go. The `Differs from <upstream>` line is the tag
    // holder's warning. The reading itself goes from the row it stands on: its own chip on the graph (デザイン規約
    // §ref の種別), the REMOTES row on the left. Leased to the commit the menu went up on: moved since, the row stops
    // applying and greys where it stands (`RefBranchMenu.state.remoteMoved`).
    word: "delete-remote-branch"
    whyNames: remoteDeleteRow.card.deleteNames
    leaseMoved: remoteDeleteRow.facts.remoteMoved
    reasonMarks: Words.listMarks(remoteDeleteRow.blockedReason, remoteDeleteRow.target, "remote")
    holdPresses: Metrics.holdPressesWarned
    holdTone: Theme.warning
    premise: remoteDeleteRow.target + "\n" + remoteDeleteRow.targetOid
    onHeld: {
        remoteDeleteRow.closeMenu()
        remoteDeleteRow.card.deleteRemoteRequested(remoteDeleteRow.target, remoteDeleteRow.targetOid)
    }
}
