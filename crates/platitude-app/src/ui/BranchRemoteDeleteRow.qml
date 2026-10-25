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
    // (デザイン規約 §左メニューの所作 の削除の表).
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
