pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

// The find card over the graph and what a query does to the view. Rows are marked in Rust (`GraphModel.setFind`,
// matching rules in `platitude-core::find`); this end moves the viewport and reads the count back. The rules both find
// bars share (where a search starts, Enter / Shift+Enter, the count, the press away) are 規約 §コミットを探す; what
// a miss dims is `GraphRowDelegate.dimmed`.
//
// Floats at the graph's top-right, over the rows, which keep their seats (規約 §可否・警告の出し場所); a question bar
// goes first — two cards down from one edge leave neither readable, and the question is what is being waited on.
// `✕` / Escape take the dimming away; the selection stays where the last match put it.
//
// Only the commits drawn now are searched (the loaded window); past it nothing is found, and the note under the card
// says so (`unansweredNote`).
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
    /// The newest row matched, so the graph steps out from under the card by its height. Covering a row is normally
    /// fine, unless that row is what is being looked for. Back the moment it stops matching, so the eye never learns
    /// the gap as a fixed place. A WIP row never
    /// matches (`find::Query::matches`), so `firstMatched` is the whole question. Headless: `clears=`, and
    /// `find_settled shift=` on the second line only (the first is read before the animation and is always 0).
    readonly property bool findClears: find.open && find.graphModel.firstMatched
    /// The note's press, waiting for the step it asked for: the graph's finish count when it was asked, `-1` while
    /// nothing is out. Any walk that settles after it answers it — the one asked for, or a pass that overtook it
    /// (`GraphModel` lowers `growing` at either).
    property int furtherFrom: -1

    /// Brings the card up; refused while a question is standing (the header says why).
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
    // and goes to the hit (user decision — ask before changing). Never loaded by itself: a load walks the whole window
    // again, not a cost to pay per keystroke. The words are `Words.findNone*`.
    unansweredNote: find.graphModel.truncated
                    ? Words.findNoneWithMore(find.graphModel.windowStep)
                    : Words.findNone
    unansweredAction: find.graphModel.truncated && !find.graphModel.growing
                      ? Words.loadFurther(find.graphModel.windowStep) : ""
    onNoteActed: find.loadFurther()
    // Hex under the floor finds no commit by its hash, and nothing on screen says where the floor is: `731` could be a
    // missing commit or an id not read yet. A fact about ids, never an order: "at least N characters" as other
    // products say it (GitLab's `Enter at least three characters to search`), not `Type …`, which would order about
    // someone looking for a hex-looking word. The number comes from core (`find::MIN_OID_PREFIX`); written into the
    // words it would drift from the floor. Gone with any non-hex character (a space too) and at the floor, where the
    // line matches as an id. Up for hex-looking words too (`add`, `be`, digits; user decision — ask before changing):
    // the machine cannot tell them from an id's head, and a fact about ids misleads nobody — whatever the hit count.
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
