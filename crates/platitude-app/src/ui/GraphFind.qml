pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The find card over the graph and what a query does to the view. Rows are marked in Rust (`GraphModel.setFind`,
// matching rules in `platitude-core::find`); this end moves the viewport and reads the count back.
FindBar {
    id: find

    required property var graphModel
    /// The list being searched: which rows are on screen, and which one is current.
    required property var view
    /// A question is standing over a row.
    required property bool asking

    /// The search came to rest on a row, so the right-hand panes follow.
    signal landed(string oidHex)

    /// A query is standing, matched or not — what lets rows dim, so a query with no answers dims them all.
    readonly property bool findOn: find.open && find.graphModel.searching
    /// The newest row matched, so the graph steps out from under the card (規約 §コミットを探す). A WIP row never
    /// matches.
    readonly property bool findClears: find.open && find.graphModel.firstMatched

    /// Brings the card up; refused while a question is standing (規約 §コミットを探す).
    function startFind() {
        if (find.asking)
            return
        find.raise()
        // The card keeps its last query, but its marks came off on close.
        find.runFind()
    }
    /// A press landed elsewhere (`RepoPage.releasePressedAway`) at `scenePos`: an empty card goes, one with a query
    /// stays (規約 §コミットを探す). "Empty" is the model's answer (`searching`), which owns the whitespace rule.
    function dropIfEmpty(scenePos) {
        if (!find.open || find.graphModel.searching || find.holds(scenePos))
            return
        find.dropAway()
        find.runFind()
    }
    /// Re-runs the search and goes to where it lands. Every keystroke: it only walks rows in memory, no git.
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
    /// Puts the search on `row`, moving the viewport only when the row is off screen. `atMatch` is not assigned here —
    /// that would break its binding, and the count would stop following a background refresh.
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
    // Bound: a background refresh re-marks the rows without anybody typing.
    matches: find.graphModel.matchCount
    atMatch: find.graphModel.matchCount > 0 ? find.graphModel.matchOrdinal(find.view.currentIndex) : 0
    refused: find.open && find.graphModel.searching && find.graphModel.matchCount === 0
    refusedTip: find.graphModel.truncated
                ? qsTr("Nothing in the loaded history matches — older commits are not loaded")
                : qsTr("Nothing in this history matches")
    onQueryChanged: find.runFind()
    onNextRequested: find.findNext()
    onPreviousRequested: find.findPrevious()
}
