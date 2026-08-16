pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The two things a graph row puts under a resting pointer — the card of
// its own message, and the refs one chip had to stack — and the beat that
// decides when either of them goes away.
//
// Held here rather than in the rows: delegates are recycled out from under
// an open popup (each of the two says so for itself). Held together
// because only one of them is ever out, and that rule has to be decided
// somewhere both can be seen (デザイン規約 §hover のツールチップ).
//
// Sized to the page it covers: the popups are placed in this item's
// coordinates, and a row hands over a point in the scene's.
Item {
    id: host

    required property GraphPane graphPane
    /// The branch the working tree is on; that one leads nowhere.
    required property string currentBranch
    /// A ref menu is standing on one of the list's rows — see the settle
    /// timer below on why that keeps the list up.
    required property bool menuStanding

    /// The pointer is on the row, or on the chip whose list is up (or is
    /// about to be). The graph writes the first as the pointer comes and
    /// goes; both are what the settle beats read.
    property bool rowCardWanted: false
    property bool refListWanted: false
    /// The chip the open list hangs off (null when none). The graph's rows
    /// read it back through `GraphPane.chipListAnchor`, so a hand that
    /// walked down into the list and comes back to that chip re-holds it
    /// instead of sitting out the opening rest again.
    property var refListAnchor: null
    /// The chip's list is up, or is about to be. Only one of the two is
    /// ever out, and the chip's is the more particular
    /// (デザイン規約 §hover のツールチップ).
    readonly property bool refListUp: host.refListWanted || refList.opened

    /// The automation's two handles into this pair (`row-card` /
    /// `ref-list-card`), an automation-only exposure the same as
    /// `GraphPane.view` is (app-ui.md).
    readonly property alias listPopup: refList
    readonly property alias hoverCard: rowCard

    /// A stacked row was chosen, or right-clicked. The list's rows answer
    /// the same right-click the chip does, with no row of the graph to
    /// fall back to.
    signal recordActivated(string record)
    signal recordMenuAsked(string record)

    anchors.fill: parent

    function openRowCard(row) {
        if (!row || host.refListUp)
            return
        rowCard.subject = row.subject
        rowCard.body = row.body
        rowCard.author = row.author
        rowCard.atime = row.atime
        rowCard.mates = row.co_authors
        // Under the pointer, not under the row: a row is as wide as the
        // pane, so its left edge is nowhere near the hand.
        const at = row.mapToItem(host, row.pointerX, row.height)
        rowCard.x = at.x
        rowCard.y = at.y
        host.rowCardWanted = true
        rowCard.open()
    }
    function settleRowCard() {
        rowCardKeep.settle()
    }
    /// Down now, not in a beat's time: what makes way for the chip's list
    /// has to be gone before it is drawn, or the two overlap for as long
    /// as the wait.
    function closeRowCard() {
        rowCard.close()
    }

    function openRefList(records, anchor) {
        const at = anchor.mapToItem(host, 0, anchor.height)
        // The row's card opens under the pointer, which is on the chip
        // — it would be drawn over the list the chip is opening.
        host.closeRowCard()
        refList.records = records
        // Sized before it is shown, so it does not grow under the hand
        // that is walking into it — see the function.
        refList.layOutRows()
        refList.x = at.x
        refList.y = at.y
        host.refListWanted = true
        host.refListAnchor = anchor
        refList.open()
    }
    function closeRefListUnlessEntered() {
        host.refListWanted = false
        host.settleRefList()
    }
    function settleRefList() {
        refListKeep.settle()
    }

    // The refs one chip had to stack, unstacked under it. It opens and
    // closes with the pointer, and the pointer is over exactly one of the
    // two things that keep it up: the chip, or the list itself.
    RefListPopup {
        id: refList
        currentBranch: host.currentBranch
        onPicked: record => host.recordActivated(record)
        onMenuAsked: record => host.recordMenuAsked(record)
        onClosed: {
            host.refListWanted = false
            host.refListAnchor = null
        }
    }
    // A ref menu standing on one of the list's rows keeps the list up
    // under it: the hand went into the menu, not away, and closing the
    // list would pull the ground out from what it right-clicked. The
    // menu's own close settles this again (`RepoPage.onDismissed`).
    HoverCardHost {
        id: refListKeep
        card: refList
        pointedAt: host.refListWanted
        grace: host.menuStanding
    }
    // ---- the row's own card -----------------------------------------
    // Opened by a row once the pointer has rested on it, closed when the
    // pointer leaves both it and the row.
    CommitHoverCard {
        id: rowCard
        textWidth: host.graphPane.width / 2
        // A quarter each to the subject and the body, so the card can
        // never pass half the pane however long a message is.
        textHeight: host.graphPane.height / 4
    }
    HoverCardHost {
        id: rowCardKeep
        card: rowCard
        pointedAt: host.rowCardWanted
    }

    Connections {
        // The row it hangs off is a delegate, and delegates travel: once
        // the graph moves under it the list is pointing at nothing.
        target: host.graphPane.view
        function onContentYChanged() { refList.close() }
    }
}
