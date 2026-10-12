pragma ComponentBehavior: Bound

import QtQuick
import platitude
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
    /// The note's press, waiting for the step it asked for: the graph's finish count when it was asked, `-1` while
    /// nothing is out. Any walk that settles after it answers it — the one asked for, or a pass that overtook it
    /// (`GraphModel` lowers `growing` at either).
    property int furtherFrom: -1

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
    /// The note's press: the next step of history, as the graph's tail loads it; the search runs again once it lands.
    function loadFurther() {
        if (!find.graphModel.truncated || find.graphModel.growing)
            return
        find.furtherFrom = find.graphModel.finishCount
        find.graphModel.growWindow()
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
    searching: find.graphModel.searching
    // Not while the history is being walked: until it settles, neither "the whole history" nor "more to load" is known
    // (`truncated` is set when the walk ends).
    unanswered: find.open && find.graphModel.searching && find.graphModel.matchCount === 0
                && !find.graphModel.loading
    // Older commits are not loaded: the note's press loads the next step as the graph's tail does, then searches again
    // (規約 §コミットを探す).
    unansweredNote: find.graphModel.truncated
                    ? Words.findNoneWithMore(find.graphModel.windowStep)
                    : Words.findNone
    unansweredAction: find.graphModel.truncated && !find.graphModel.growing
                      ? Words.findLoadFurther(find.graphModel.windowStep) : ""
    onNoteActed: find.loadFurther()
    // Hex under the floor finds no commit by its hash, and nothing on screen would say why (規約 §コミットを探す).
    hint: find.graphModel.shortOfAnOid ? qsTr("Hashes need at least %1 characters").arg(GitFacts.oidFloor()) : ""
    // A new line drops a press's wait: the landing would search for a line nobody pressed for.
    onQueryChanged: {
        find.furtherFrom = -1
        find.runFind()
    }
    onNextRequested: find.findNext()
    onPreviousRequested: find.findPrevious()

    Connections {
        target: find.graphModel
        function onStatsChanged() {
            if (find.furtherFrom < 0 || find.graphModel.growing || find.graphModel.finishCount <= find.furtherFrom)
                return
            find.furtherFrom = -1
            // Closed meanwhile: nothing to search.
            if (find.open)
                find.runFind()
        }
    }
}
