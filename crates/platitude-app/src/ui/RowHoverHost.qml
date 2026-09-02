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
    /// Any of the page's menus is standing, this pair's own included. The
    /// row's card does not come out behind one: the hand is in the menu,
    /// and a card opened now is drawn over it (the card opens last, so it
    /// wins the overlay — observed).
    ///
    /// The chip's list is the exception, and it is `menuStanding` above
    /// that holds it: a ref menu raised from one of its rows is standing
    /// *on* the list, not over it.
    required property bool hoverBlocked

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
    /// The commit whose card is out (empty when none). The graph's rows
    /// read it back through `GraphPane.rowCardOid`, so the row the card
    /// came out of keeps its hover band while the card stands: the card
    /// opens off the row's own bottom edge, so the hand that walks down
    /// into it to read the message is off the row from that moment, and
    /// the row went dark under a card that is still up — leaving the
    /// message with nothing on screen saying which commit it is of.
    property string rowCardOid: ""
    /// The chip's list is up, or is about to be. Only one of the two is
    /// ever out, and the chip's is the more particular
    /// (デザイン規約 §hover のツールチップ).
    readonly property bool refListUp: host.refListWanted || refList.opened

    /// The automation's two handles into this pair (`row-card` /
    /// `ref-list-card`), an automation-only exposure the same as
    /// `GraphPane.view` is (app-ui.md).
    readonly property alias listPopup: refList
    readonly property alias hoverCard: rowCard

    /// The commit whose chip the open list belongs to. Every row in it
    /// names a ref on that one commit, so a click in the card is a click
    /// on that row of the graph.
    property string refListOid: ""
    /// And which row of the graph it is, so a click in the card does not send the page looking for one it was handed
    /// (`GraphModel::row_of` walks every loaded row — CLAUDE.md §性能予算).
    property int refListRow: -1

    /// A stacked row was double-clicked, clicked once, or right-clicked.
    /// The list's rows answer the same gestures the row under them does,
    /// with no row of the graph to fall back to. **The second click,
    /// spaced, has no signal here**: the wait it opens is the graph's own
    /// (`GraphPane.noteRowClick`), because the card and the row it stands
    /// on are one target.
    signal recordActivated(string record)
    signal recordChosen(string oidHex, int atRow)
    signal recordMenuAsked(string oidHex, string record)

    anchors.fill: parent

    /// A menu went up over whatever was resting: the card goes now rather
    /// than in a beat's time, the same as when the chip's list takes over.
    onHoverBlockedChanged: {
        if (host.hoverBlocked)
            host.closeRowCard()
    }

    function openRowCard(row) {
        if (!row || host.refListUp || host.hoverBlocked)
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
        // Which row it is of, for the row itself to read back — the card
        // holds no commit of its own beyond the fields copied above.
        host.rowCardOid = row.oid_hex
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

    /// Opens the chip's names on the chip's own seat.
    ///
    /// **The first row lands exactly on the chip** (規約 §グラフ行の
    /// ダブルクリック), so the name the chip was showing is not written
    /// out again beside itself, and the `+N` is covered by the row that
    /// takes its place. From there it always grows the one way, into the
    /// graph: a card that picked its side by how long the names were
    /// answered the same chip differently on different rows, and the
    /// reason was not on screen to be read (observed —
    /// the same condition §hover のツールチップ turns down for the file
    /// rows). The names' heads hold still instead, which is what a name
    /// is told apart by.
    function openRefList(oidHex, atRow, records, anchor) {
        const at = anchor.mapToItem(host, 0, 0)
        host.refListOid = oidHex
        host.refListRow = atRow
        // The row's card opens under the pointer, which is on the chip
        // — it would be drawn over the list the chip is opening.
        host.closeRowCard()
        refList.records = records
        // Never narrower than the chip it is covering — the rows draw no
        // `+N` and the chip may be wearing one (see the property).
        refList.minRowWidth = anchor.width
        // What is left of the page from the chip's own left edge, less
        // what the card puts around a chip on its own edges (its
        // padding, and nothing else — `RefListPopup`'s rows keep no
        // margin) and the stop every floating card in the app shares:
        // `spaceXxl` short of the edge, the ceiling a menu's width has
        // (規約 §メニュー). Without it the longest name takes the card
        // flat against the window frame.
        refList.chipRoom = host.width - Theme.spaceXxl - at.x - refList.padding
        // Sized before it is shown, so it does not grow under the hand
        // that is walking into it — see the function.
        refList.layOutRows()
        refList.x = at.x - refList.padding
        // Down from the chip, and kept inside the page: a card longer
        // than what is under the chip would otherwise be moved by
        // `Popup` itself, which knows nothing about the seat it is
        // keeping (it still covers the chip either way — the card is
        // taller than one).
        refList.y = Math.max(0, Math.min(at.y - refList.padding - refList.chipInset,
                                         host.height - refList.cardHeight))
        host.refListWanted = true
        host.refListAnchor = anchor
        refList.open()
    }
    function closeRefListUnlessEntered() {
        host.refListWanted = false
        host.settleRefList()
    }
    /// Down now, not in a beat's time — the same as the row's card, and
    /// for the same reason: what takes this card's place is drawn on the
    /// ground it is standing on (the name box opens in the chip column
    /// this covers), and the two would overlap for as long as the wait.
    function closeRefList() {
        host.refListWanted = false
        refList.close()
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
        // The rows of this card and the rows of the graph are the same targets, so a click here is answered by the row
        // it is standing on (see the card's `rowClicks`).
        rowClicks: host.graphPane
        rowOid: host.refListOid
        onPicked: record => host.recordActivated(record)
        onChose: host.recordChosen(host.refListOid, host.refListRow)
        // The row this card stands on travels with the name: the menu it raises is that row's, aimed at the name that
        // was pressed (デザイン規約 §グラフ行の右クリック).
        onMenuAsked: record => host.recordMenuAsked(host.refListOid, record)
        // The card is drawn over the chip that raised it, so the chip
        // stops being able to say the hand is still on it — the row
        // under a popup sees no hover at all. Until the card itself has
        // the pointer, the ask that opened it is what holds it up
        // (`refListWanted` stays on through the leave the row reports
        // the instant this is drawn); from the moment the card has it,
        // the card holds itself, and letting go is what closes it.
        onPointerInsideChanged: {
            if (refList.pointerInside)
                host.refListWanted = false
        }
        onClosed: {
            host.refListWanted = false
            host.refListAnchor = null
            host.refListOid = ""
            host.refListRow = -1
            // **Which row was clicked last is not forgotten here.** The card is a window onto rows that stay on
            // screen, wearing the mark a second click is aimed at — unlike the folded rail's peek, which takes its
            // rows away with it and has to forget them (app-ui.md). The memory is the graph's, and so is the row.
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
        // A quarter of the pane to the subject, which is the only field
        // here whose bound is the room there is — the body is held to a
        // count of lines instead (規約 §hover のツールチップ).
        subjectHeight: host.graphPane.height / 4
        // The band the row is holding goes with the card, and it is the
        // card's own close that says when — the row lost the pointer a
        // beat before that and cannot tell.
        onClosed: host.rowCardOid = ""
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
