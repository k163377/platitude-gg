import QtQuick
import platitude.ui

// The delete table's second row: the remote reading alone — on the current branch the only delete on offer
// (デザイン規約 §左メニューの所作 の削除の表). On the first level's last group, standing on the BRANCH card's frozen
// answers (`RefBranchMenu.state`).
AppMenuItem {
    id: remoteDeleteRow

    required property RefBranchMenu card
    readonly property QtObject facts: remoteDeleteRow.card.state

    code: "push --delete"
    nameMark: "remote"
    codeName: remoteDeleteRow.facts.remoteCounterpart
    // Seated only while a remote reading exists; grey is "not now" — the reading stands on another commit
    // (デザイン規約 §左メニューの所作 の削除の表). A drift greys this row and `Delete both` and does not take them
    // away (user decision — ask before changing): the name is over there, so a row gone would look as if the remote
    // lacked it, and the one place that reads "it is there too" would go. The `Differs from <upstream>` line is the tag
    // holder's warning. The reading itself goes from the row it stands on: its own chip on the graph (デザイン規約
    // §ref の種別), the REMOTES row on the left.
    offered: remoteDeleteRow.facts.kind === "branch" && remoteDeleteRow.facts.remoteCounterpart !== ""
    blockedReason: remoteDeleteRow.facts.canDeleteRemote ? ""
                 : Words.differsFrom(remoteDeleteRow.facts.remoteCounterpart)
    reasonMarks: Words.listMarks(remoteDeleteRow.blockedReason, remoteDeleteRow.facts.remoteCounterpart, "remote")
    holdPresses: Metrics.holdPressesWarned
    holdTone: Theme.warning
    onHeld: {
        remoteDeleteRow.closeMenu()
        remoteDeleteRow.card.deleteRemoteRequested(remoteDeleteRow.facts.remoteCounterpart,
                                                   remoteDeleteRow.facts.remoteCounterpartOid)
    }
}
