import QtQuick
import platitude.ui
import platitude.ui.words

// The delete table's third row: the branch and its remote reading together. Two commands, so words and no code
// (§git 用語のコード表記). The local half runs first; its refusal stops the pair with nothing touched, and a refused
// lease on the remote half stops it with the local one done (`RepoSession::delete_branch_everywhere`). On the first
// level's last group, standing on the BRANCH card's answers (`RefBranchMenu.state`).
AppMenuItem {
    id: bothDeleteRow

    required property RefBranchMenu card
    readonly property QtObject facts: bothDeleteRow.card.state

    /// Whether the local half goes as `-D` — the card's `refused`, which the table's first row follows too: the local
    /// half is the same delete. Latched for the presses under way (デザイン規約 §回数押し): read live, git's answer
    /// landing between presses would turn presses begun on the plain delete into a force delete. The count cannot carry
    /// it (this row takes three either way), so the `Binding` freezes it while `AppMenuItem.gesturing`.
    property bool forces: bothDeleteRow.card.refused
    readonly property Binding forcesHeld: Binding {
        target: bothDeleteRow
        property: "forces"
        value: bothDeleteRow.card.refused
        when: !bothDeleteRow.gesturing
        restoreMode: Binding.RestoreNone
    }

    text: qsTr("Delete both")
    note: bothDeleteRow.forces ? qsTr("not merged") : ""
    // Its remote half is leased as the table's second row is, and stops applying with it (`remoteMoved`).
    offered: bothDeleteRow.facts.kind === "branch" && bothDeleteRow.facts.remoteCounterpart !== ""
             && !bothDeleteRow.facts.remoteMoved
    // The local half's reasons first, since it runs first — core's order (`offers::RefMenuOffers::delete_both_out`).
    blockedReason: bothDeleteRow.card.deleteWhy(bothDeleteRow.facts.deleteBothOut)
    // As on the local row and the reading's.
    reasonMarkWord: bothDeleteRow.facts.deleteBothOut === "held" ? bothDeleteRow.facts.holderLeaf : ""
    reasonMarkKind: bothDeleteRow.facts.holderHome ? "home" : "tree"
    reasonMarks: bothDeleteRow.facts.deleteBothOut === "drifted" || bothDeleteRow.facts.deleteBothOut === "unread"
                 ? Words.listMarks(bothDeleteRow.blockedReason, bothDeleteRow.facts.remoteCounterpart, "remote") : []
    holdPresses: Metrics.holdPressesWarned
    // `warning` whether the local half runs as `-d` or `-D`: the commits only it reached come back from the discard
    // record with the name (デザイン規約 §回数押し の色の表).
    holdTone: Theme.warning
    // Both halves: the branch here and the commit it stands on, the reading over there and its lease.
    premise: [bothDeleteRow.facts.refId, bothDeleteRow.facts.refOid, bothDeleteRow.facts.remoteCounterpart,
              bothDeleteRow.facts.remoteCounterpartOid].join("\n")
    onHeld: {
        bothDeleteRow.closeMenu()
        // One write with one answer, so one thing to put back. `forces` is still the latched one.
        bothDeleteRow.card.deleteEverywhereRequested(bothDeleteRow.facts.refId, bothDeleteRow.facts.remoteCounterpart,
                                                     bothDeleteRow.forces, bothDeleteRow.facts.remoteCounterpartOid)
    }
}
