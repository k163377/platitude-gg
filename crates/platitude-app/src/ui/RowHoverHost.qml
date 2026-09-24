pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

// The two things a row puts under a resting pointer — the card of its own
// message, and the refs one chip had to stack — and the beat that decides
// when either of them goes away. The card serves two lists: the graph's
// rows and the ones the right pane lists under a choice
// (デザイン規約 §複数のコミットを選ぶ); the chip's list is the graph's alone.
//
// Held here: delegates are recycled out from under
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
    /// The two sections that answer for a reading: which remote a branch
    /// is measured against and how far it stands from it, and which
    /// branch reads a remote-tracking ref. Owned by the page, and the
    /// same pair the left panel's rows ask (`NavFacts`).
    required property var branchesModel
    required property var remotesModel
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
    /// *on* the list.
    required property bool hoverBlocked

    /// The pointer is on the row, or on the chip whose list is up (or is
    /// about to be). The graph writes the first as the pointer comes and
    /// goes; both are what the settle beats read.
    property bool rowCardWanted: false
    property bool refListWanted: false
    /// The chip the open list hangs off (null when none). The graph's rows
    /// read it back through `GraphPane.chipListAnchor`, so a hand that
    /// walked down into the list and comes back to that chip re-holds it
    /// at once; the opening rest was already served.
    property var refListAnchor: null
    /// The commit whose card is out (empty when none). The graph's rows
    /// read it back through `GraphPane.rowCardOid`, so the row the card
    /// came out of keeps its hover band while the card stands: the card
    /// opens off the row's own bottom edge, so the hand that walks down
    /// into it to read the message is off the row from that moment, and
    /// the row went dark under a card that is still up — leaving the
    /// message with nothing on screen saying which commit it is of.
    property string rowCardOid: ""
    /// And which row of the graph that is, held for the reason the
    /// chip's list holds its own (`refListRow`): a press in the card
    /// picks the row, and looking one up is a walk over every loaded
    /// row (CLAUDE.md §性能予算).
    property int rowCardRow: -1
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
    signal recordActivated(var chip)
    signal recordChosen(string oidHex, int atRow)
    signal recordMenuAsked(string oidHex, var chip)
    /// The name a stacked row opens under itself was pressed: the reader is going to the commit it stands on
    /// (`RefListPopup.followed`).
    signal mateFollowed(string oidHex)

    /// The note under a cut message was pressed in the row's card: the
    /// reader is asking for the whole of it, and the whole of it is in
    /// the pane one click away (デザイン規約 §hover のツールチップ).
    /// The same pair the card's rows are chosen by — this is that click,
    /// made from inside the card.
    signal messageAsked(string oidHex, int atRow)

    anchors.fill: parent

    /// A menu went up over whatever was resting: the card goes now,
    /// the same as when the chip's list takes over.
    onHoverBlockedChanged: {
        if (host.hoverBlocked)
            host.closeRowCard()
    }

    function openRowCard(row) {
        if (!row || host.refListUp || host.hoverBlocked)
            return
        // **Already out, of this very commit** — the hand walked down into the card and came back to the row it came
        // off. What is being asked for is the hold, and the hold is the whole of what is given: the seat below comes
        // off the pointer, so working it out again slides the card sideways under a hand that only went back where it
        // started (P3-確認事項, observed). **By the commit and by the card**: delegates travel, and a
        // card that has closed has no commit of its own left (`rowCard.onClosed`), so a second look at the same row
        // after it went opens properly.
        if (rowCard.opened && host.rowCardOid === row.oid_hex) {
            host.rowCardWanted = true
            return
        }
        rowCard.subject = row.subject
        rowCard.body = row.body
        rowCard.author = row.author
        rowCard.atime = row.atime
        rowCard.mates = row.co_authors
        // Under the pointer: a row is as wide as the
        // pane, so its left edge is nowhere near the hand. **Worked out
        // first**, since the bounds below are the room left under it.
        const at = row.mapToItem(host, row.pointerX, row.height)
        rowCard.x = at.x
        rowCard.y = at.y
        // **What the card is for depends on what the row already showed.** A graph row carries the whole subject, so
        // the card is a glance at the body and offers the way to the rest; a row of a choice carries one cut line, so
        // the card is where the message is read and holds none of it back — and offers nothing, a door out of it
        // being a door out of what the reader was picking (デザイン規約 §複数のコミットを選ぶ). Assigned:
        // the card is one object serving two lists, and a binding would have to name both.
        const whole = row.wholeMessage === true
        // **Bounded by the room there is either way.** What "holds nothing back" buys is a paragraph limit lifted
        // — uncapped, a five thousand byte body drew a slab the height of the
        // window over the very list it was opened from (measured `--preset edges`). The room is what lies under the
        // row, all of it: a card that stops short of the floor is holding back for no reason a reader can see. The
        // subject takes a quarter of it and the body the rest, less the three rows the author, the date and the
        // margins stand in.
        const below = Math.max(0, host.height - at.y)
        rowCard.bodyRows = whole ? 0 : Metrics.hoverBodyRows
        rowCard.subjectHeight = whole ? below / 4 : host.graphPane.height / 4
        rowCard.bodyHeight = whole ? Math.max(0, below - below / 4 - 3 * Theme.rowHeight) : 0
        rowCard.asksForMore = !whole
        host.rowCardWanted = true
        // Which row it is of, for the row itself to read back — the card
        // holds no commit of its own beyond the fields copied above.
        host.rowCardOid = row.oid_hex
        host.rowCardRow = row.index
        rowCard.open()
    }
    function settleRowCard() {
        rowCardKeep.settle()
    }
    /// Down now: what makes way for the chip's list
    /// has to be gone before it is drawn, or the two overlap for as long
    /// as the wait.
    function closeRowCard() {
        rowCard.close()
    }

    /// Opens the chip's names on the chip's own seat.
    ///
    /// **The first row lands exactly on the chip** (規約 §グラフ行の
    /// ダブルクリック), so the name the chip was showing is not written
    /// out again beside itself. The sheets behind it go down as the card
    /// comes up (`RefChipStack.unstacked`), which is what the card stands
    /// in for. From there it always grows the one way, into the
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
        refList.mates = host.matesFor(records)
        // What it has to cover (see the property). The anchor is the whole
        // stack, so its width is the front card and the fan behind it —
        // read before `refListAnchor` below takes the sheets down.
        refList.coverWidth = anchor.width
        // What is left of the page from the chip's own left edge, less
        // what stands outside a row's names on the far side — the bar's
        // own gutter, which is the wider of the two a row can end with
        // (`RefListPopup.rightInset`; which one it takes is not known
        // until the rows are measured, and a ceiling guessed at the
        // narrower one would let the longest name run past it) — and the
        // frame the card draws around the rows, and the stop every
        // floating card in the app shares: `spaceXxl` short of the edge,
        // the ceiling a menu's width has (規約 §メニュー). Without it the
        // longest name takes the card flat against the window frame.
        refList.chipRoom = host.width - Theme.spaceXxl - at.x
                           - Theme.navBarGutter - refList.padding
        // And how far down it may run: the page, and no further. This is
        // the one card whose rows scroll, and one that ran past the
        // bottom would put its lower rows where nothing can reach them
        // (see the property).
        refList.listRoom = host.height
        // Sized before it is shown, so it does not grow under the hand
        // that is walking into it — see the function.
        refList.layOutRows()
        refList.x = at.x - Theme.spaceXs - refList.padding
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
    /// What each of the names on this commit reads, or is read
    /// by — one entry per record, null where there is nothing to name.
    ///
    /// **Asked as the card opens** (デザイン規約 §行が読む答えはどこから
    /// 来るか): the card is measured over its rows in that same turn, so
    /// an answer landing after it is one the card has no room for.
    function matesFor(records) {
        const here = ({})
        for (let i = 0; i < records.length; ++i)
            here[records[i].name] = 1
        const out = []
        for (let j = 0; j < records.length; ++j)
            out.push(host.mateOf(records[j], here))
        return out
    }
    /// And the one line a single record opens under itself — **the same
    /// line the left panel's rows open**, drawn from the same table
    /// (`NavFacts`), so the two places cannot come to say one relation
    /// two ways.
    ///
    /// **A counterpart this commit already carries is a row of this
    /// card** — standing on one commit is what being level means, and a
    /// line naming it would write the same name twice in one card.
    ///
    /// **The working copy holding the branch has no line here**: the
    /// chip's own frame is already green for it (デザイン規約 §ref の種別),
    /// where a panel row has no colour of its own to say it with.
    function mateOf(chip, here) {
        const name = chip.name
        if (chip.kind === "branch") {
            const gone = host.branchesModel.upstreamGoneOf(name)
            const reads = gone !== ""
                        ? gone : host.branchesModel.upstreamOf(name)
            if (reads === "" || (gone === "" && here[reads] === 1))
                return null
            // Where a press on the name goes: the commit that reading stands on (`NavFacts.place`).
            const line = NavFacts.readingLine(
                { "upstream": reads, "gone": gone !== "",
                  "upstreamTo": NavFacts.place("remote", reads, host.branchesModel.upstreamOidOf(name)) })
            // The measure is the branch's, and the branch here is the
            // name over this line — so the chip draws it there
            // (`RefChip.trackOnName`), the way the panel's own row does.
            line.ahead = host.branchesModel.aheadOf(name)
            line.behind = host.branchesModel.behindOf(name)
            return line
        }
        if (chip.kind === "remote") {
            const local = host.remotesModel.trackedBy(name)
            if (local === "" || here[local] === 1)
                return null
            // This line names the branch, so the measure rides it.
            return NavFacts.branchLine(
                local,
                { "ahead": host.branchesModel.aheadOf(local),
                  "behind": host.branchesModel.behindOf(local) },
                NavFacts.place("branch", local, host.branchesModel.oidOfName(local)))
        }
        return null
    }
    function closeRefListUnlessEntered() {
        host.refListWanted = false
        host.settleRefList()
    }
    /// Down now — the same as the row's card, and
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
        // The current branch's stand-in is the other thing a chip unfolds from, and a press on the card is then its
        // press (`GraphPane.pinPressed`).
        onStandIn: host.refListAnchor !== null && host.refListAnchor === host.graphPane.headPin.chipItem
        onStandInPressed: modifiers => host.graphPane.pinPressed(host.refListRow, modifiers)
        onPicked: chip => host.recordActivated(chip)
        onChose: host.recordChosen(host.refListOid, host.refListRow)
        // The row this card stands on travels with the name: the menu it raises is that row's, aimed at the name that
        // was pressed (デザイン規約 §グラフ行の右クリック).
        onMenuAsked: chip => host.recordMenuAsked(host.refListOid, chip)
        // The card went with the press and the graph goes somewhere under a hand that stayed: the rest of that
        // gesture, and the row that comes under it, are not the rows' (`GraphPane.settleUnderHand`).
        onFollowed: to => {
            host.graphPane.settleUnderHand()
            host.mateFollowed(to.oid)
        }
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
            // **Which row was clicked last stays remembered here.** The card is a window onto rows that stay on
            // screen, wearing the mark a second click is aimed at — unlike the folded rail's peek, which takes its
            // rows away with it and has to forget them (app-ui.md). The memory is the graph's, and so is the row.
        }
    }
    // A ref menu standing on one of the list's rows keeps the list up
    // under it: the hand went into the menu, and closing the
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
        // The subject's own bound is set when the card opens, beside the body's — the two lists want different shares
        // and one card serves both (`openRowCard`).
        // The band the row is holding goes with the card, and it is the
        // card's own close that says when — the row lost the pointer a
        // beat before that and cannot tell.
        onClosed: {
            host.rowCardOid = ""
            host.rowCardRow = -1
        }
        // **Read before the close**: closing is what clears
        // the pair above, so a card that took its own commit down with it
        // would send the page looking for nothing. Down now —
        // what the press leads to is behind this card.
        onMessageAsked: {
            const oidHex = host.rowCardOid
            const atRow = host.rowCardRow
            host.closeRowCard()
            host.messageAsked(oidHex, atRow)
        }
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
