pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

// The two popups a row opens under a resting pointer — its commit's card, and the refs one chip stacked — each on a
// seat of its own (`HoverSeat`). The card serves the graph's rows and a choice's rows (デザイン規約 §複数のコミットを選ぶ),
// told apart by area: the same commit shown twice is two targets. The chip's list is the graph's alone, keyed by the
// commit and the surface its chip stands on (a row, the current branch's stand-in). Held here because delegates are
// recycled under an open popup; held together because only one of them is ever out — the list wins over the card
// (デザイン規約 §hover のツールチップ「1 つのポインタが開けるものは 1 つ」).
//
// The rows report what the hand did (`pointRow` / `pointChip` / `unpointChip`) and read back what stands (`cardKey` /
// `listKey`); nothing here is written by a row.
Item {
    id: host

    /// The graph pane: its list (`view`), its hand (`hand`), the current branch's stand-in (`headPin`) and the click
    /// doors the list's rows go through. `var`, so a test can hand in a stand-in.
    required property var graphPane
    /// The branch the worktree is on; that one leads nowhere.
    required property string currentBranch
    /// The sections `mateOf` asks.
    required property var branchesModel
    required property var remotesModel
    required property var tagsModel
    /// The remote a tag's readings are weighed against — the one this window's tag rows act on, as the left panel's
    /// TAGS rows read them (`NavList.pushRemote`).
    required property string tagAgainst
    /// The page's standing menu's door (`RepoPage.menuDoor`): where it was raised and on what, null while none stands.
    /// Nothing here comes out behind a menu, and what is out goes at once — but for the chip's list under a menu
    /// raised on its own rows, which stands on it and keeps it up (デザイン規約 §メニュー). **Every reading below is
    /// one expression over this one property**: two readings derived from it are updated one after the other, and a
    /// blocker read between the two takes the list down under its own menu.
    required property var menuDoor
    /// The hand over the right pane (`DetailsPane.hand`), whose rows the card serves too: a choice's rows are that
    /// pane's, so their seat reads that hand — its hold, its latch, its walk — not the graph's. `var`, so a test can
    /// hand in a stand-in.
    required property var chosenHand

    readonly property var view: host.graphPane.view
    readonly property HandWatch hand: host.graphPane.hand

    /// What stands, as the rows read it back: the card's target key (`card:<area>:<commit>`) and the list's
    /// (`chip:<commit>:<surface>`), empty for none.
    readonly property string cardKey: cardSeat.shown ? cardSeat.targetKey : chosenSeat.shown ? chosenSeat.targetKey : ""
    readonly property string listKey: listSeat.shown ? listSeat.targetKey : ""
    /// The commit whose card is out, and the row and commit the open list is of (empty / -1 for none).
    readonly property string cardOid: cardSeat.shown ? cardSeat.target.oid
                                     : chosenSeat.shown ? chosenSeat.target.oid : ""
    readonly property string refListOid: listSeat.shown ? listSeat.target.oid : ""
    readonly property int refListRow: listSeat.shown ? listSeat.target.row : -1
    /// The chip the open list hangs off, null for none — what a run reads to say which surface the list is on.
    readonly property var listAnchor: listSeat.shown ? listSeat.target.anchor : null
    /// The hand is on what the card / the list hangs off (the seats' own answer).
    readonly property bool cardPointed: cardSeat.pointed || chosenSeat.pointed
    readonly property bool listPointed: listSeat.pointed
    /// Automation only: the list's beat after a leave is still counting (PGG_AUTO_ACT=menu-list own).
    readonly property bool listSettling: listSeat.leaving
    /// Automation only: the card's seat as it stands — its state, the target it latched against, whether an ask is
    /// kept (`tst_graphhover`).
    readonly property string cardSeatState: cardSeat.state
    readonly property string cardLatched: cardSeat.latchedKey
    readonly property bool cardAskKept: cardSeat.pending !== null
    /// …and whether the list's seat keeps an ask (a chip asked for behind a hold).
    readonly property bool listAskKept: listSeat.pending !== null
    /// Automation-only handles (`row-card` / `ref-list-card`), the same exposure as `GraphPane.view`
    /// (rules-refs/app-ui.md).
    readonly property alias listPopup: refList
    readonly property alias hoverCard: rowCard

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

    // ---- what the rows report --------------------------------------------------------------------------------------
    /// The hand came to rest on a commit's row in `area` (`graph` / `chosen`), or left it. `row` is the delegate, for
    /// its fields and its place when the card opens; the target is the commit in that area, and the delegate is
    /// asked again whether it still shows it (`alive`). A leave is the delegate's: it lets go of what it asked for,
    /// even once the graph has rewritten it to another commit in place (`splice_notified`) — and nothing another
    /// delegate asked for.
    function pointRow(row, inside, area) {
        const oid = row.oid_hex
        const key = "card:" + area + ":" + oid
        // Each area's seat: the graph's rows on the graph's hand, a choice's on the right pane's.
        const seat = area === "chosen" ? chosenSeat : cardSeat
        if (!inside) {
            seat.unpoint(key)
            seat.unpoint(seat.keyOf(target => target.item === row))
            return
        }
        seat.point({
            "key": key, "oid": oid, "area": area, "row": row.index, "item": row,
            "alive": () => row.oid_hex === oid && row.visible && row.pooled !== true
        })
    }
    /// The hand came onto the chip `anchor` of `oidHex`'s row (or the stand-in's), with `records` to unstack: the
    /// list opens at once (規約 §hover のツールチップ「展開は即時に開く」).
    function pointChip(oidHex, atRow, records, anchor) {
        const key = host.chipKey(oidHex, anchor)
        listSeat.point({
            "key": key, "oid": oidHex, "row": atRow, "records": records, "anchor": anchor,
            "alive": () => host.chipStands(oidHex, anchor)
        }, true)
        // The list out is this chip's already — the hand on the row a reread put back under it — and says its names as
        // they are now.
        if (listSeat.shown && listSeat.targetKey === key)
            host.refreshRefList()
    }
    /// The hand left that chip's column. Only the chip the list stands on lets it go — the rows read the hand in no
    /// set order, so the row the hand came onto may have asked first — and that is the seat's own rule
    /// (`HoverSeat.unpoint`); the chip lets go of what it asked for whatever commit its row has been rewritten to
    /// since (as `pointRow`).
    function unpointChip(oidHex, anchor) {
        listSeat.unpoint(host.chipKey(oidHex, anchor))
        listSeat.unpoint(listSeat.keyOf(target => target.anchor === anchor))
    }
    function chipKey(oidHex, anchor) {
        return "chip:" + oidHex + ":" + (anchor === host.graphPane.headPin.chipItem ? "pin" : "row")
    }
    /// Whether `anchor` still shows `oidHex`'s chip: the stand-in while it stands for that commit, a row's while the
    /// delegate it is in shows that commit (the first ancestor with a commit is the delegate).
    function chipStands(oidHex, anchor) {
        const pin = host.graphPane.headPin
        if (anchor === pin.chipItem)
            return pin.visible && pin.wanted && pin.standsFor === oidHex
        for (let at = anchor; at !== null && at !== undefined; at = at.parent) {
            if (at.oid_hex !== undefined)
                return at.oid_hex === oidHex && at.visible === true && at.pooled !== true
        }
        return false
    }
    /// Whether `item` is `owner` or inside it.
    function within(item, owner) {
        for (let at = item; at !== null && at !== undefined; at = at.parent) {
            if (at === owner)
                return true
        }
        return false
    }
    /// `row` stands for another commit now — the graph's reread rewrote it in place (`GraphList.rowRewritten`, the
    /// choice's rows too), or the current branch's stand-in moved to another (`GraphHeadPin.rewritten`). **By what
    /// asked**, the delegate and the chip in it, not by the key: the key is derived from the commit the row shows,
    /// which has already moved on. A commit the graph still holds moved rather than went — a reread wrote others in
    /// above it — and what hangs off it stays, the hand inside it or not: the view puts itself back over it
    /// (`GraphRowWalk.shiftRows`), and the row under the hand asks again as the rows read it (デザイン規約 §hover
    /// のツールチップ「外乱では閉じない」). The stand-in standing for another commit is another chip: what its list said
    /// is gone (§グラフの中で HEAD を見失わない). What went goes, out or kept, the hand inside it, a menu on the list, or
    /// not; a choice's rows, with no view putting itself back, let theirs go either way.
    function rowGone(row) {
        const pin = host.graphPane.headPin
        if (row === pin) {
            // Gone from its edge with the hand in its list, still standing for the same commit — the real row back on
            // screen, or HEAD's names on their way after a reread — the stand-in leaves the list where it stands, and
            // the list says its names again as it comes back (`onVisibleChanged` below).
            const waits = host.listOnStandIn && listSeat.cardPointed && !pin.visible
                          && listSeat.target.oid === pin.standsFor
            if (!waits)
                listSeat.forget(target => target.anchor === pin.chipItem, "gone")
            return
        }
        cardSeat.forget(target => target.item === row && !host.inGraph(target.oid), "gone")
        chosenSeat.forget(target => target.item === row, "gone")
        listSeat.forget(target => host.within(target.anchor, row) && !host.inGraph(target.oid), "gone")
    }
    /// The graph still holds `oidHex`'s commit.
    function inGraph(oidHex) {
        return host.graphPane.graphModel.rowOf(oidHex) >= 0
    }
    /// The row that shows `oidHex`'s commit now, null where none is built.
    function rowShowing(oidHex) {
        const at = host.graphPane.graphModel.rowOf(oidHex)
        const row = at < 0 ? null : host.view.itemAtIndex(at)
        return row !== null && row.oid_hex === oidHex ? row : null
    }
    /// The graph was read again (`RepoPage`, once the view has put itself back over its rows): what is out of a
    /// commit stands on the row that shows that commit now. A hand on the rows has done this already as the rows read
    /// it; a hand inside what is out has not. The list says the names that row's chip shows now, and goes with a chip
    /// that shows none (デザイン規約 §hover のツールチップ「外乱では閉じない」: what is still there changes its words, what
    /// went goes).
    function reseat() {
        const card = cardSeat.target
        if (card !== null && card.area === "graph" && (card.item === null || card.item.oid_hex !== card.oid)) {
            const row = host.rowShowing(card.oid)
            if (row !== null) {
                cardSeat.target = Object.assign({}, card, {
                    "item": row, "row": row.index,
                    "alive": () => row.oid_hex === card.oid && row.visible && row.pooled !== true
                })
            }
        }
        const list = listSeat.target
        if (list === null || list.anchor === host.graphPane.headPin.chipItem || host.chipStands(list.oid, list.anchor))
            return
        const row = host.rowShowing(list.oid)
        if (row === null)
            return
        if (row.labelRecords.length === 0) {
            listSeat.forget(target => target.key === list.key, "gone")
            return
        }
        listSeat.target = Object.assign({}, list, {
            "row": row.index, "anchor": row.chipItem, "alive": () => host.chipStands(list.oid, row.chipItem)
        })
        host.refreshRefList()
    }
    /// The names on `anchor` changed under the same commit (`GraphList.chipChanged`, the stand-in's too): the list out
    /// of it says the names as they are now (`refreshRefList`), and goes with the last of them — what it said is gone
    /// (デザイン規約 §hover のツールチップ「外乱では閉じない」). A chip whose row was rewritten to another commit is not
    /// the list's any more (`rowGone`).
    function chipChanged(oidHex, anchor) {
        const target = listSeat.target
        if (target === null || target.anchor !== anchor || !host.chipStands(target.oid, anchor))
            return
        if (anchor.records.length === 0)
            listSeat.forget(was => was.anchor === anchor, "gone")
        else
            host.refreshRefList()
    }
    /// The open list says the names its chip shows now, seated and measured as those names open afresh
    /// (`seatRefList`) — over the chip laid out first, in the turn its names changed (`RefChipStack.layOutNow`): read
    /// unforced, the chip answers with its width for the names before.
    function refreshRefList() {
        const target = listSeat.target
        if (JSON.stringify(target.anchor.records) === JSON.stringify(refList.records))
            return
        target.anchor.layOutNow()
        host.seatRefList(target)
    }
    /// Down now, as the owner asks: the name box that takes the column's place, a jump the list's line made.
    function closeRefList() {
        listSeat.takeDown("owner", false)
    }
    function closeRowCard() {
        cardSeat.takeDown("owner", false)
        chosenSeat.takeDown("owner", false)
    }

    // ---- the seats -------------------------------------------------------------------------------------------------
    readonly property HoverSeat cardSeat: HoverSeat {
        id: cardSeat
        hand: host.hand
        card: rowCard
        restMs: Metrics.tipDelayMs
        graceMs: Metrics.hoverKeepMs
        // Behind a menu nothing comes out; under the chip's list the card gives way (the list is the more particular
        // of the two). Only while the list stands, not its beat: `tipDelayMs` outlasts `hoverKeepMs`, so a card asked
        // for as the hand comes down off the chip opens after the list has gone.
        blockedBy: host.menuDoor !== null ? "menu" : listSeat.state === "shown" ? "list" : ""
        // A second click waiting out its window holds hover still (規約 §グラフ行のダブルクリック).
        frozen: host.view.renameWaiting
        onOpenAsked: target => host.openRowCard(target)
    }
    readonly property HoverSeat listSeat: HoverSeat {
        id: listSeat
        hand: host.hand
        card: refList
        // The list opens the moment the hand is on the chip: a hold lifts with the hand's move, a dismissal with the
        // hand leaving the chip and coming back.
        expands: true
        restMs: 0
        graceMs: Metrics.hoverKeepMs
        blockedBy: host.menuDoor !== null && host.menuDoor.area !== "refList" ? "menu" : ""
        frozen: host.view.renameWaiting
        onOpenAsked: target => host.openRefList(target)
    }
    // The choice's rows' seat, on the same card: the right pane's hand. The two seats never show at once — the hand
    // is in one pane, and the beat one seat leaves on is shorter than the rest the other waits — and the card's
    // closing is read by the seat showing it alone (`HoverSeat.noteClosed`).
    readonly property HoverSeat chosenSeat: HoverSeat {
        id: chosenSeat
        hand: host.chosenHand
        card: rowCard
        restMs: Metrics.tipDelayMs
        graceMs: Metrics.hoverKeepMs
        blockedBy: host.menuDoor !== null ? "menu" : ""
        onOpenAsked: target => host.openRowCard(target)
    }
    /// The standing menu was raised on a row of the list out — on that list's commit: the menu stands on the list and
    /// holds it, and the hold is that list's (a held target is not handed over, `HoverSeat`); its close lets the
    /// hand decide again (the hold falling starts the beat).
    readonly property bool menuOnList: host.menuDoor !== null && host.menuDoor.area === "refList"
                                       && listSeat.shown && listSeat.target.oid === host.menuDoor.key
    onMenuOnListChanged: listSeat.hold("menu", host.menuOnList)

    // The rows moved under the hand — a scroll, a search, the keyboard: the list hangs off a delegate and points at
    // nothing now, and the card at a row that is not where it was (デザイン規約 §hover のツールチップ「的が手の下で動けば
    // hover は外れ、出したものはその瞬間に消える」) — the stand-in's list too, which a reader's move may take to the other
    // edge or off the graph. A move a reread made that the view could not take back (`HandWatch.disturbed`) leaves what
    // the hand is inside where it stands (「外乱では閉じない」); the view putting itself back over the rows tells no move
    // at all (`HandWatch.rowsKept`). Rewritten or renamed, it is told as such (`rowGone`, `chipChanged`).
    Connections {
        target: host.hand
        function onRowsShifted() {
            if (!(host.hand.disturbed && listSeat.cardPointed))
                listSeat.displace("slid")
            if (!(host.hand.disturbed && cardSeat.cardPointed))
                cardSeat.displace("slid")
        }
    }
    /// The list out stands on the current branch's stand-in's chip.
    readonly property bool listOnStandIn: listSeat.shown && listSeat.target.anchor === host.graphPane.headPin.chipItem
    // The stand-in back at its edge, for the same commit, with the list it left still out (`rowGone`): the list says
    // the names it shows now. Back for another commit, it has said so (`GraphHeadPin.rewritten`) and the list is gone.
    Connections {
        target: host.graphPane.headPin
        function onVisibleChanged() {
            const pin = host.graphPane.headPin
            if (pin.visible && host.listOnStandIn && listSeat.target.oid === pin.standsFor)
                host.refreshRefList()
        }
    }
    Connections {
        target: host.chosenHand
        function onRowsShifted() { chosenSeat.displace("slid") }
    }

    // ---- opening ---------------------------------------------------------------------------------------------------
    function openRowCard(target) {
        const row = target.item
        rowCard.subject = row.subject
        rowCard.body = row.body
        rowCard.author = row.author
        rowCard.authorTime = row.author_time
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
        rowCard.open()
    }
    /// Opens the chip's names on the chip's own seat: the first row lands exactly on the chip, and the card always
    /// grows right, into the graph (規約 §グラフ行のダブルクリック) — picking a side by name length would open the same
    /// chip differently on different rows.
    function openRefList(target) {
        host.seatRefList(target)
        refList.open()
    }
    /// Seats the list on its chip as the chip stands now and measures it over the names the chip shows now — as it
    /// opens, and again as the names change under it (`refreshRefList`): the chip stands on its right edge, so fewer
    /// names move its left edge right, and a card left where it was would show the chip's end past its own.
    function seatRefList(target) {
        const anchor = target.anchor
        // Seated on the stack as the reader saw it, every sheet drawn: `listKey`, already this chip's, has taken the
        // sheets down by now, and the stack stands on its right edge and rises by half its fan (`RefChipStack.lift`),
        // so its corner has moved since.
        const now = anchor.mapToItem(host, 0, 0)
        const at = Qt.point(now.x + anchor.width - anchor.stackedWidth, now.y + anchor.lift - anchor.stackedLift)
        // The chip's names as they stand now, not as they stood at the ask: an ask kept behind a menu opens after the
        // chip may have changed.
        const records = anchor.records
        refList.records = records
        refList.mates = host.matesFor(records, target.oid)
        // The anchor is the whole stack, front card and fan — as it stands with nothing over it (`listKey`, already
        // this chip's, has taken the sheets down by now).
        refList.coverWidth = anchor.stackedWidth
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
    /// counterpart already a row of this card (same commit = level), and none for the worktree holding the
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

    // ---- the popups ------------------------------------------------------------------------------------------------
    RefListPopup {
        id: refList
        currentBranch: host.currentBranch
        // A click here is answered by the graph row the card stands on (`RefListPopup.rowClicks`).
        rowClicks: host.graphPane
        rowOid: host.refListOid
        // Unfolded from the current branch's stand-in, a press on the card is the pin's (`GraphPane.pinPressed`).
        onStandIn: host.listAnchor !== null && host.listAnchor === host.graphPane.headPin.chipItem
        onStandInPressed: modifiers => host.graphPane.pinPressed(host.refListRow, modifiers)
        onPicked: chip => host.recordActivated(chip)
        onChose: host.recordChosen(host.refListOid, host.refListRow)
        // That row's menu, aimed at the pressed name (デザイン規約 §グラフ行の右クリック).
        onMenuAsked: chip => host.recordMenuAsked(host.refListOid, chip)
        // The rest of the gesture that pressed the line is not the rows' under it (`GraphPane.hushRowClicks`); the
        // graph moving under the still hand is the hand's to notice (`HandWatch.rowsMoved`).
        onFollowed: to => {
            host.graphPane.hushRowClicks()
            host.mateFollowed(to.oid)
        }
    }
    CommitHoverCard {
        id: rowCard
        textWidth: host.graphPane.width / 2
        // The subject's and body's bounds are set in `openRowCard` — the two lists want different shares.
        // Read before the close, which clears the pair. Down now: what the press leads to is behind this card.
        onMessageAsked: {
            const oidHex = host.cardOid
            const atRow = cardSeat.target !== null && cardSeat.target.area === "graph" ? cardSeat.target.row : -1
            host.closeRowCard()
            host.messageAsked(oidHex, atRow)
        }
    }
}
