pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The find card over the graph, and what a query does to the view.
//
// The rows are marked in Rust (`GraphModel.setFind`), which is also where the rule about what a typed line matches
// lives (`platitude-core::find`). This end moves the viewport and reads
// the count back.
FindBar {
    id: find

    required property var graphModel
    /// The list being searched. Whether a row is already on screen is its answer, and so is which row the view is on.
    required property var view
    /// A question is standing over a row.
    required property bool asking

    /// The search came to rest on a row — the page is told, so the panes on the right follow.
    signal landed(string oidHex)

    /// Something is being looked for — which is not the same as something being found. The one thing that lets a row
    /// dim, and it takes the query: with a query and no answers, every row really is "not one of
    /// them", so the whole graph goes down.
    readonly property bool findOn: find.open && find.graphModel.searching
    /// The newest row is one of the answers, so the graph steps out from under the card (規約 §コミットを探す). The
    /// working-tree row is not a commit and never matches, so a dirty tree answers
    /// false here by itself.
    readonly property bool findClears: find.open && find.graphModel.firstMatched

    /// Brings the card down over the list. Refused while a question is standing: two bars from one edge would leave
    /// neither readable, and the question is the one with something waiting on it.
    function startFind() {
        if (find.asking)
            return
        find.raise()
        // The card comes back up holding what was last typed into it, and the marks came off when it went away — so the
        // search is run again.
        find.runFind()
    }
    /// A press landed somewhere else (`RepoPage.releasePressedAway`), at `scenePos`. A card with nothing in it goes
    /// away with the press — it was an offer nobody took, and the press says where the reading went on — while a card
    /// with a query in it stays: the query is the only thing there would be to lose, and `✕` and Escape are both one
    /// gesture away (規約 §コミットを探す).
    ///
    /// "Nothing was typed" is the model's answer: whitespace alone is not a
    /// query, and which rule that is belongs in one place (`platitude-core::find`).
    function dropIfEmpty(scenePos) {
        if (!find.open || find.graphModel.searching || find.holds(scenePos))
            return
        find.dropAway()
        find.runFind()
    }
    /// Re-runs the search and goes to where it lands. Called on every keystroke: the marking is a walk over rows
    /// already in memory, and no git runs for any of it.
    function runFind() {
        find.graphModel.setFind(find.open ? find.query : "")
        find.goToMatch(find.graphModel.matchFrom(find.view.firstVisibleRow()))
    }
    function findNext() {
        find.goToMatch(find.graphModel.matchAfter(find.view.currentIndex))
    }
    function findPrevious() {
        find.goToMatch(find.graphModel.matchBefore(find.view.currentIndex))
    }
    /// Puts the search on `row`: the owner is told so the right-hand panes follow, and the viewport moves only when the
    /// row is not already on screen — a match in sight is not worth taking the reader's place for. Which match it is
    /// comes out of this card's own binding, so nothing here assigns it (that would break the binding, and the count
    /// would then stop following a background refresh).
    function goToMatch(row) {
        if (row < 0)
            return
        if (!find.view.rowOnScreen(row)) {
            // A wheel notch still in flight was aimed elsewhere (`WheelGlide.halt`).
            find.view.haltGlide()
            find.view.positionViewAtIndex(row, ListView.Center)
        }
        find.landed(find.graphModel.oidAt(row))
    }

    loaded: find.graphModel.rowTotal
    // Bound: a background refresh re-marks the rows without anybody typing
    // (app-ui.md §QML バインディングはプロパティにしか反応しない), and the count has to be the rows'
    // count.
    matches: find.graphModel.matchCount
    atMatch: find.graphModel.matchCount > 0 ? find.graphModel.matchOrdinal(find.view.currentIndex) : 0
    // "There is a query" is the model's answer: what counts as a query at all
    // (whitespace does not) is `platitude-core::find`'s rule and belongs in one place.
    refused: find.open && find.graphModel.searching && find.graphModel.matchCount === 0
    refusedTip: find.graphModel.truncated
                ? qsTr("Nothing in the loaded history matches — older commits are not loaded")
                : qsTr("Nothing in this history matches")
    onQueryChanged: find.runFind()
    onNextRequested: find.findNext()
    onPreviousRequested: find.findPrevious()
}
