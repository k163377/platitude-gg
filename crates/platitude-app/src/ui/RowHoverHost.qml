pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

// The two popups a row opens under a resting pointer — its commit's card, and the refs one chip stacked — and the
// beats that close them. The card serves the graph's rows and a choice's rows (デザイン規約 §複数のコミットを選ぶ);
// the chip's list is the graph's alone. Held here because delegates are recycled under an open popup; held together
// because only one of them is ever out (デザイン規約 §hover のツールチップ).
Item {
    id: host

    required property GraphPane graphPane
    /// The branch the working tree is on; that one leads nowhere.
    required property string currentBranch
    /// The sections `mateOf` asks.
    required property var branchesModel
    required property var remotesModel
    required property var tagsModel
    /// The remote a tag's readings are weighed against — the one this window's tag rows act on, as the left panel's
    /// TAGS rows read them (`NavList.pushRemote`).
    required property string tagAgainst
    /// A ref menu is standing on one of the list's rows; it keeps the list up (`refListKeep`).
    required property bool menuStanding
    /// Any of the page's menus is standing. The row's card does not come out behind one — opened last, it would be
    /// drawn over the menu. The chip's list is exempt (`menuStanding`).
    required property bool hoverBlocked

    /// The pointer is on the row (written by `RepoPage.restOnCommit`), or on the chip whose list is up or about to
    /// be — what the settle beats read.
    property bool rowCardWanted: false
    property bool refListWanted: false
    /// The chip the open list hangs off (null when none), read back through `GraphPane.chipListAnchor`: its row stays
    /// lit, unstacks its sheets and ignores the leave the card causes (`GraphRowDelegate.listOnThisChip`).
    property var refListAnchor: null
    /// The commit whose card is out (empty when none), read back through `GraphPane.rowCardOid`: the row keeps its
    /// hover band while its card stands, though the hand walking down into the card has left the row.
    property string rowCardOid: ""
    property int rowCardRow: -1
    /// The chip's list is up or about to be; it wins over the row's card.
    readonly property bool refListUp: host.refListWanted || refList.opened

    /// Automation-only handles (`row-card` / `ref-list-card`), the same exposure as `GraphPane.view`
    /// (rules-refs/app-ui.md).
    readonly property alias listPopup: refList
    readonly property alias hoverCard: rowCard

    /// The commit whose chip the open list belongs to — a click in the card is a click on that graph row.
    property string refListOid: ""
    property int refListRow: -1

    /// A stacked row was double-clicked, clicked once, or right-clicked — the gestures of the graph row under it. The
    /// spaced second click has no signal: its wait is the graph's (`GraphPane.noteRowClick`), card and row being one
    /// target.
    signal recordActivated(var chip)
    signal recordChosen(string oidHex, int atRow)
    signal recordMenuAsked(string oidHex, var chip)
    /// The name a stacked row opens under itself was pressed: the reader is going to the commit it stands on
    /// (`RefListPopup.followed`).
    signal mateFollowed(string oidHex)

    /// The note under a cut message was pressed in the row's card: the row's click, made from inside the card — the
    /// whole message is in the pane (デザイン規約 §hover のツールチップ).
    signal messageAsked(string oidHex, int atRow)

    anchors.fill: parent

    onHoverBlockedChanged: {
        if (host.hoverBlocked)
            host.closeRowCard()
    }

    function openRowCard(row) {
        if (!row || host.refListUp || host.hoverBlocked)
            return
        // Already out for this commit (the hand came back from the card): only hold it — re-seating off the pointer
        // would slide the card sideways. By commit, since delegates travel; a closed card has cleared its commit.
        if (rowCard.opened && host.rowCardOid === row.oid_hex) {
            host.rowCardWanted = true
            return
        }
        rowCard.subject = row.subject
        rowCard.body = row.body
        rowCard.author = row.author
        rowCard.atime = row.atime
        rowCard.mates = row.co_authors
        // Under the pointer: a row is pane-wide.
        const at = row.mapToItem(host, row.pointerX, row.height)
        rowCard.x = at.x
        rowCard.y = at.y
        // A graph row shows the whole subject, so its card glances at the body and offers the rest; a choice's row
        // shows one cut line, so its card holds nothing back and offers no way out (デザイン規約 §複数のコミットを選ぶ).
        // Assigned, not bound: one card serves both lists.
        const whole = row.wholeMessage === true
        // Bounded either way — an uncapped body would cover the list it opened from. The whole card gets all the room
        // under the row: a quarter to the subject, the rest to the body less three rows for author, date and margins.
        const below = Math.max(0, host.height - at.y)
        rowCard.bodyRows = whole ? 0 : Metrics.hoverBodyRows
        rowCard.subjectHeight = whole ? below / 4 : host.graphPane.height / 4
        rowCard.bodyHeight = whole ? Math.max(0, below - below / 4 - 3 * Theme.rowHeight) : 0
        rowCard.asksForMore = !whole
        host.rowCardWanted = true
        host.rowCardOid = row.oid_hex
        host.rowCardRow = row.index
        rowCard.open()
    }
    function settleRowCard() {
        rowCardKeep.settle()
    }
    /// Down now, not after the beat: what makes way for the chip's list must be gone before the list is drawn.
    function closeRowCard() {
        rowCard.close()
    }

    /// Opens the chip's names on the chip's own seat: the first row lands exactly on the chip, and the card always
    /// grows right, into the graph (規約 §グラフ行のダブルクリック) — picking a side by name length would open the same
    /// chip differently on different rows.
    function openRefList(oidHex, atRow, records, anchor) {
        const at = anchor.mapToItem(host, 0, 0)
        host.refListOid = oidHex
        host.refListRow = atRow
        // The row's card would be drawn over the list.
        host.closeRowCard()
        refList.records = records
        refList.mates = host.matesFor(records, oidHex)
        // The anchor is the whole stack (front card and fan) — read before `refListAnchor` below takes the sheets
        // down.
        refList.coverWidth = anchor.width
        // The page right of the chip, less the `spaceXxl` stop every floating card keeps from the edge (規約 §メニュー),
        // the card's frame, and the bar's gutter — the wider of the two insets a row can end with, since which one it
        // takes is unknown until the rows are measured (`RefListPopup.rightInset`).
        refList.chipRoom = host.width - Theme.spaceXxl - at.x
                           - Theme.navBarGutter - refList.padding
        // At most the page tall: its rows scroll, and rows past the bottom could not be reached.
        refList.listRoom = host.height
        // Sized before it is shown, so it does not grow under the hand.
        refList.layOutRows()
        refList.x = at.x - Theme.spaceXs - refList.padding
        // Clamped into the page here: left to `Popup`, a card too long for the room under the chip is moved without
        // regard to its seat.
        refList.y = Math.max(0, Math.min(at.y - refList.padding - refList.chipInset,
                                         host.height - refList.cardHeight))
        host.refListWanted = true
        host.refListAnchor = anchor
        refList.open()
    }
    /// Each record's second line (`mateOf`), null where there is none. Asked as the card opens: the card is measured
    /// over its rows in that turn, so a later answer has no room (デザイン規約 §行が読む答えはどこから来るか).
    /// `oidHex` is the commit every record of the card stands on.
    function matesFor(records, oidHex) {
        const here = ({})
        for (let i = 0; i < records.length; ++i)
            here[records[i].name] = 1
        const out = []
        for (let j = 0; j < records.length; ++j)
            out.push(host.mateOf(records[j], here, oidHex))
        return out
    }
    /// The line one record opens under itself — the left panel's line, from the same `NavFacts`. None for a
    /// counterpart already a row of this card (same commit = level), and none for the working copy holding the
    /// branch: the chip's frame already says it (デザイン規約 §ref の種別).
    function mateOf(chip, here, oidHex) {
        const name = chip.name
        // A tag's reading — the copy here, or a remote's on this commit — standing apart from the right one: the test
        // the left panel's TAGS rows wear their warning by (`NavSectionModel.tagApartAt`), its note said as the line.
        if (chip.kind === "tag")
            return host.tagsModel !== null && host.tagsModel.tagApartAt(name, oidHex, host.tagAgainst)
                 ? NavFacts.apartLine(NavFacts.apartNote(host.tagsModel.tagWeighedAgainst(name, host.tagAgainst)))
                 : null
        if (chip.kind === "branch") {
            const gone = host.branchesModel.upstreamGoneOf(name)
            const reads = gone !== ""
                        ? gone : host.branchesModel.upstreamOf(name)
            if (reads === "" || (gone === "" && here[reads] === 1))
                return null
            // Where a press on the name goes (`NavFacts.place`).
            const line = NavFacts.readingLine(
                { "upstream": reads, "gone": gone !== "",
                  "upstreamTo": NavFacts.place("remote", reads, host.branchesModel.upstreamOidOf(name)) })
            // The counts are the branch's, so the chip draws them on the name (`RefChip.trackOnName`).
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
    /// Down now, as `closeRowCard`: the name box that takes its place opens in the chip column this covers.
    function closeRefList() {
        host.refListWanted = false
        refList.close()
    }
    function settleRefList() {
        refListKeep.settle()
    }

    RefListPopup {
        id: refList
        currentBranch: host.currentBranch
        // A click here is answered by the graph row the card stands on (`RefListPopup.rowClicks`).
        rowClicks: host.graphPane
        rowOid: host.refListOid
        // Unfolded from the current branch's stand-in, a press on the card is the pin's (`GraphPane.pinPressed`).
        onStandIn: host.refListAnchor !== null && host.refListAnchor === host.graphPane.headPin.chipItem
        onStandInPressed: modifiers => host.graphPane.pinPressed(host.refListRow, modifiers)
        onPicked: chip => host.recordActivated(chip)
        onChose: host.recordChosen(host.refListOid, host.refListRow)
        // That row's menu, aimed at the pressed name (デザイン規約 §グラフ行の右クリック).
        onMenuAsked: chip => host.recordMenuAsked(host.refListOid, chip)
        // The graph moves under a hand that stayed: the rest of that gesture is not the rows'
        // (`GraphPane.settleUnderHand`).
        onFollowed: to => {
            host.graphPane.settleUnderHand()
            host.mateFollowed(to.oid)
        }
        // The card covers its chip, whose row then sees no hover: `refListWanted` holds the card through the leave
        // the row reports, until the card has the pointer; from then the card holds itself.
        onPointerInsideChanged: {
            if (refList.pointerInside)
                host.refListWanted = false
        }
        onClosed: {
            host.refListWanted = false
            host.refListAnchor = null
            host.refListOid = ""
            host.refListRow = -1
            // The last click is not forgotten here, unlike the folded rail's peek (rules-refs/app-ui.md): the memory
            // is the graph's, and its rows stay on screen.
        }
    }
    // The menu's close settles this again (`RepoPage.onDismissed`).
    HoverCardHost {
        id: refListKeep
        card: refList
        pointedAt: host.refListWanted
        grace: host.menuStanding
    }
    // ---- the row's own card -----------------------------------------
    CommitHoverCard {
        id: rowCard
        textWidth: host.graphPane.width / 2
        // The subject's and body's bounds are set in `openRowCard` — the two lists want different shares.
        // The row's band goes with the card's own close — the row lost the pointer earlier and cannot tell.
        onClosed: {
            host.rowCardOid = ""
            host.rowCardRow = -1
        }
        // Read before the close, which clears the pair. Down now: what the press leads to is behind this card.
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
        // The list hangs off a delegate; once the graph scrolls, it points at nothing.
        target: host.graphPane.view
        function onContentYChanged() { refList.close() }
    }
}
