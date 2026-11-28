import QtQuick
import platitude.ui

// The delete table's first row — the branch itself (デザイン規約 §左メニューの所作 の削除の表; a remote branch's own
// delete is the second row's, `BranchRemoteDeleteRow`). It stands on the BRANCH card's answers
// (`RefBranchMenu.state`), which also turn this row into the held `-D` when git refuses while the menu stands
// (`RefBranchMenu.refused`).
AppMenuItem {
    id: deleteRow

    required property RefBranchMenu card
    /// The card's `state`, by another name: this row is an Item, whose own `state` is QML's states.
    readonly property QtObject facts: deleteRow.card.state

    // The delete names its target: during the hold, what is about to go has to be readable on the row itself
    // (デザイン規約 §メニュー「例外は削除行だけ」).
    /// `card.refused` as latched at the first press: the warning's count (`AppMenuItem.armedPresses`) means refused, the
    /// plain delete's means not. Read live, git's answer to the last attempt would re-word the row between presses
    /// (デザイン規約 §回数押し).
    readonly property bool shownRefused: deleteRow.armedPresses >= Metrics.holdPressesWarned

    // `-D` quotes git's own refusal hint. The row says only what git said: a refusal does not mean the commits would be
    // reachable from nowhere — git measures against the upstream where there is one, so a branch merged into HEAD but
    // not pushed is refused, and one unmerged into HEAD but pushed goes through. Hence `not merged` and `-D`, no more.
    code: deleteRow.shownRefused ? "branch -D" : "branch --delete"
    // Data: untranslated, and it does not widen the menu past `labelColW` (`codeName`) — behind its kind's mark
    // (デザイン規約 §ref の種別「名前の印」).
    nameMark: "branch"
    codeName: deleteRow.facts.refId
    note: deleteRow.shownRefused ? qsTr("not merged") : ""
    // The delete forms are a fixed table whose rows grey out rather than go (`RefBranchMenu`).
    word: "delete-branch"
    whyNames: deleteRow.card.deleteNames
    // Asked of the reason this row shows: a row blocked for another reason names no worktree.
    reasonMarkWord: deleteRow.why === "held" ? deleteRow.facts.holderLeaf : ""
    reasonMarkKind: deleteRow.facts.holderHome ? "home" : "tree"
    // The plain delete is a delete too: two presses, in the plain ink (デザイン規約 §回数押し). `--delete` loses nothing
    // — the second press only rules out a mistaken first —; something is lost only where git refuses, so only there is
    // it the warning's three.
    holdPresses: deleteRow.card.refused ? Metrics.holdPressesWarned : Metrics.holdPressesPlain
    // The name it deletes and the commit that stands on: a branch here moved part way would take other commits.
    premise: deleteRow.facts.kind + "\n" + deleteRow.facts.refId + "\n" + deleteRow.facts.refOid
    // `warning` when held: the branch's tip and upstream come back from the discard record (デザイン規約 §回数押し の色
    // の表). Off the latched count, as the words are.
    holdTone: deleteRow.armedPresses >= Metrics.holdPressesWarned ? Theme.warning : Theme.textPrimary
    // At the last press, so `shownRefused` is the count these presses were given: an unmerged answer landing on the
    // floor turns no plain delete into `-D`.
    onHeld: {
        // A branch's plain delete keeps the menu up: git's answer, which comes back later, lands on this row, where it
        // was asked, and a refusal turns it into `branch -D` (`AppMenuItem.armedPresses` reopens with the warning's
        // count). A bar over the graph would make the hand close the menu and go elsewhere.
        if (!deleteRow.shownRefused) {
            deleteRow.card.deleteRequested(deleteRow.facts.kind, deleteRow.facts.refId, deleteRow.facts.refName,
                                           deleteRow.facts.refOid)
            return
        }
        deleteRow.closeMenu()
        deleteRow.card.forceDeleteRequested(deleteRow.facts.refId)
    }

    /// The plain delete landed while the menu stood: through the row, so a git that answers inside the floor still
    /// leaves the full fill its time. A property's value, not a bare child, as `forcesHeld` is on the pair's row.
    /// No picture tells it (a menu closed and one never opened look alike): the verbs `delete-branch-chip` (graph
    /// row) / `delete-branch` (left menu) say it, and what Qt does is `tests/qml/tst_menudismiss.qml`'s.
    readonly property Connections landing: Connections {
        target: deleteRow.card
        function onLandedDeleteChanged() {
            if (deleteRow.card.answersHere(deleteRow.card.landedDelete))
                deleteRow.closeMenu()
        }
    }
}
