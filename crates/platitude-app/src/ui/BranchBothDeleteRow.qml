import QtQuick
import platitude.ui

// The delete table's third row: the branch and its remote reading together. Two commands, so words and no code
// (§git 用語のコード表記). The local half runs first; its refusal stops the pair with nothing touched, and a refused
// lease on the remote half stops it with the local one done (`session::delete_branch_everywhere`). A row of the
// BRANCH card, standing on its frozen answers (`RefBranchMenu.state`).
AppMenuItem {
    id: bothDeleteRow

    required property RefBranchMenu card
    /// The table's first row, whose refusal this one follows: the local half is the same delete.
    required property BranchDeleteRow localRow
    readonly property QtObject facts: bothDeleteRow.card.state

    /// Whether the local half goes as `-D`, latched for the presses under way (デザイン規約 §回数押し): read live, git's
    /// answer landing between presses would turn presses begun on the plain delete into a force delete. The count
    /// cannot carry it (this row takes three either way), so the `Binding` freezes it while
    /// `AppMenuItem.gesturing`.
    property bool forces: bothDeleteRow.localRow.refusedRow
    readonly property Binding forcesHeld: Binding {
        target: bothDeleteRow
        property: "forces"
        value: bothDeleteRow.localRow.refusedRow
        when: !bothDeleteRow.gesturing
        restoreMode: Binding.RestoreNone
    }

    text: qsTr("Delete both")
    note: bothDeleteRow.forces ? qsTr("not merged") : ""
    offered: bothDeleteRow.facts.kind === "branch" && bothDeleteRow.facts.remoteCounterpart !== ""
    // The local half's reasons first: it runs first.
    blockedReason: bothDeleteRow.facts.canDelete && bothDeleteRow.facts.canDeleteRemote ? ""
                 : bothDeleteRow.facts.onCurrentBranch ? bothDeleteRow.card.deleteBlockedOnCurrent
                 : bothDeleteRow.facts.heldByWorktree !== "" ? bothDeleteRow.card.blockedByWorktree
                 : Words.differsFrom(bothDeleteRow.facts.remoteCounterpart)
    // As on the local row.
    reasonMarkWord: bothDeleteRow.blockedReason === bothDeleteRow.card.blockedByWorktree
                    ? bothDeleteRow.facts.holderLeaf : ""
    reasonMarkKind: bothDeleteRow.facts.holderHome ? "home" : "tree"
    reasonMarks: bothDeleteRow.blockedReason === Words.differsFrom(bothDeleteRow.facts.remoteCounterpart)
                 ? Words.listMarks(bothDeleteRow.blockedReason, bothDeleteRow.facts.remoteCounterpart, "remote") : []
    holdPresses: Metrics.holdPressesWarned
    // `warning` whether the local half runs as `-d` or `-D`: the commits only it reached come back from the discard
    // record with the name (デザイン規約 §回数押し の色の表).
    holdTone: Theme.warning
    onHeld: {
        bothDeleteRow.closeMenu()
        // One write with one answer, so one thing to put back. `forces` is still the latched one.
        bothDeleteRow.card.deleteEverywhereRequested(bothDeleteRow.facts.refId, bothDeleteRow.facts.remoteCounterpart,
                                                     bothDeleteRow.forces, bothDeleteRow.facts.remoteCounterpartOid)
    }
}
