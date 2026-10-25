import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// The list the seat opens over the left menu's sections (破棄記録仕様.md). The git commands' seat stays under it. One
// entry is picked at a time, and what it would bring back is told over the graph (`RecoverBand`); a rest on an entry
// opens its card, as a graph row's does (`RecoverHoverCard`).
Rectangle {
    id: pane

    /// The RepoPage this list belongs to.
    required property var curPage

    readonly property var entries: pane.curPage !== null ? pane.curPage.recoverEntries : []
    readonly property int picked: pane.curPage !== null ? pane.curPage.recoverPick : -1
    /// The reflogs are being read: on an empty list, before the first answer, and over the entries already up when
    /// the list reads again.
    readonly property bool reading: pane.curPage !== null && pane.curPage.recoverReading
    /// The log's words, its times included (`RecoverEntries`), and the moment the times are read against — the
    /// page's, which the band over the graph reads too.
    readonly property var words: pane.curPage !== null ? pane.curPage.recoverWords : null
    readonly property real now: pane.curPage !== null ? pane.curPage.recoverNow : 0
    /// What is typed into the filter. Automation writes it into the field itself, as a typist would.
    property alias filterText: filterField.text
    /// The entries the filter leaves, as `{key, at}` — the entry across reads (`DiscardModel.keyAt`), and where it
    /// stands in the whole list, the number a pick goes by. **Kept by key** (`keepRows`): a read moves only the rows
    /// that came or went, so a row that stays keeps its delegate — the hand on it, the card hung off it — and the
    /// list puts it back under the hand (`rowKeep`). A fresh array would send the list back to the top with every row
    /// built again (`QQuickItemView::setModel`).
    readonly property ListModel shownEntries: ListModel {}
    /// Weighs the entries against the filter, and moves the rows that changed. `read`: a read of the log, or the time
    /// moving under a filter that reads it — no reader's doing (デザイン規約 §hover のツールチップ「外乱では閉じない」);
    /// else the reader typed, and the rows moving under the hand are theirs (`HandWatch.rowsMoved`: the card goes,
    /// and the row that came under a still hand opens only once the hand has moved).
    function refilter(read) {
        const line = filterField.text
        // Nothing typed leaves every entry in without asking the rule entry by entry.
        const asked = line.trim() !== ""
        const kept = []
        for (let i = 0; i < pane.entries.length; i++) {
            if (!asked || GitFacts.shownMatches(pane.wordsOf(pane.entries[i]), line))
                kept.push({ "key": pane.entries[i].key, "at": i })
        }
        const rows = pane.shownEntries
        const same = kept.length === rows.count
            && kept.every((shown, i) => shown.key === rows.get(i).key && shown.at === rows.get(i).at)
        if (same)
            return
        if (read === true)
            rowKeep.arriving()
        pane.keepRows(kept)
        // An entry the read took away takes its card with it, the hand inside or not: what it said is gone. Asked of
        // the rows as now kept — the list lays out the row's going at its next polish.
        entryKeep.forget(target => !kept.some(shown => "entry:" + shown.key === target.key), "gone")
        if (read === true)
            rowKeep.arrived()
        else
            paneHand.rowsMoved()
    }
    /// The entry a row stands for: the one at its number, found again by its key where a read has moved the entries
    /// and not yet the row's number (the rows are told after the entries), and nothing for one that went.
    function entryOf(key, at) {
        const here = pane.entries[at]
        if (here !== undefined && here.key === key)
            return here
        const found = pane.entries.find(entry => entry.key === key)
        return found !== undefined ? found : pane.noEntry
    }
    /// What a row draws while it stands for no entry.
    readonly property var noEntry: ({ "key": "", "mark": "", "tint": "transparent", "badged": false, "paired": false,
                                      "title": "", "titleMarks": [], "listTitle": "", "listMarks": [], "at": 0,
                                      "worktree": "", "worktreeMark": "tree", "parts": [], "byPart": false })
    /// Turns the rows into `kept` by key: the rows no longer kept go, a kept one out of place moves up to its place,
    /// a new one comes in at its place, and one that stayed takes its new number.
    function keepRows(kept) {
        const rows = pane.shownEntries
        const wanted = ({})
        for (let i = 0; i < kept.length; i++)
            wanted[kept[i].key] = true
        for (let i = rows.count - 1; i >= 0; i--) {
            if (wanted[rows.get(i).key] !== true)
                rows.remove(i)
        }
        for (let i = 0; i < kept.length; i++) {
            const want = kept[i]
            if (i < rows.count && rows.get(i).key === want.key) {
                if (rows.get(i).at !== want.at)
                    rows.setProperty(i, "at", want.at)
                continue
            }
            let from = -1
            for (let j = i + 1; j < rows.count; j++) {
                if (rows.get(j).key === want.key) {
                    from = j
                    break
                }
            }
            if (from >= 0) {
                rows.move(from, i, 1)
                rows.setProperty(i, "at", want.at)
            } else {
                rows.insert(i, want)
            }
        }
        if (rows.count > kept.length)
            rows.remove(kept.length, rows.count - kept.length)
    }
    /// The hand over the list (`HandWatch`), as the left panel keeps one: the rows read their light and their ask
    /// off it by geometry — a row the list's scrolling or a read brought under a still hand lights as the row under
    /// it, and its card waits for the hand to move.
    HandWatch {
        id: paneHand
    }
    /// The row under the hand, kept where it was through a read of the log (`RowKeep`): an entry written in above it
    /// — as far as the list can scroll.
    readonly property RowKeep rowKeep: RowKeep {
        id: rowKeep
        hand: paneHand
        views: [list]
    }
    /// Automation only: the list, the hand and the card, for a run that moves the first two or puts the hand in the
    /// third (`tst_recoverpane`, `recover-reread`).
    readonly property alias entryList: list
    readonly property alias hand: paneHand
    readonly property alias entryHoverCard: entryCard
    onEntriesChanged: pane.refilter(true)
    onNowChanged: pane.refilter(true)
    Component.onCompleted: pane.refilter(false)
    /// Automation: the card is out, and for which entry (-1 for none) — where that entry stands now, a read having
    /// moved it.
    readonly property bool cardOpen: entryCard.opened
    readonly property int cardAt: entryCard.opened && entryCard.key !== ""
                                  ? pane.entries.findIndex(entry => entry.key === entryCard.key) : -1

    /// The words an entry shows, which the filter reads: what it did in the row's words and the card's, where, what it
    /// took, and when — as the row writes its time and as its card does.
    function wordsOf(entry) {
        const words = [entry.listTitle, entry.title, entry.worktree]
        for (let i = 0; i < entry.parts.length; i++)
            words.push(entry.parts[i].text)
        words.push(pane.words.ago(entry.at, pane.now), pane.words.whenWords(entry.at, pane.now))
        return words
    }

    /// The hand is on `row`, or left it (`EntryRow`): the card opens on a rest and goes a beat after the hand has left
    /// the row and the card both — the seat's doing (`entryKeep`). Rows are recycled (`AppListView.reuseItems`), so
    /// the target is the entry — by its key, which a read leaves alone where it moves the entry's number — and the row
    /// is asked again at the rest whether it still shows it.
    function pointEntry(row, inside, pointerX) {
        const key = "entry:" + row.key
        if (!inside) {
            entryKeep.unpoint(key)
            return
        }
        entryKeep.point({ "key": key, "row": row, "x": pointerX,
                          "alive": () => row.visible && !row.pooled && "entry:" + row.key === key })
    }
    /// The rest ran out on `row`: its card opens under the pointer, as a graph row's does (`RowHoverHost.openRowCard`).
    /// **Its words follow the entry while it stands** — by its key, not by the row, whose delegate a read may pool and
    /// use for another entry: a read words the entry as it does, and the time moves on (§hover のツールチップ「外乱では
    /// 閉じない」: what is still there changes its words). Placed once — a read that moves the row leaves the card where
    /// it opened.
    function openCard(row, pointerX) {
        const key = row.key
        entryCard.key = key
        entryCard.entry = Qt.binding(() => pane.entryOf(key, -1))
        entryCard.when = Qt.binding(() => pane.words.whenWords(entryCard.entry.at, pane.now))
        const at = row.mapToItem(pane, pointerX, row.height)
        entryCard.x = at.x
        entryCard.y = at.y
        entryCard.open()
    }
    /// Automation: a hand that has rested on the shown entry `index` — hover cannot be injected (verify-ui スキル), and
    /// the rest is taken as done, so the card opens at once.
    function restOnShown(index) {
        const row = list.itemAtIndex(index)
        if (row !== null)
            entryKeep.point({ "key": "entry:" + row.key, "row": row, "x": row.width / 2 }, true)
        return row !== null
    }

    color: Theme.bgSurface

    // Laid over the left menu, which stays up underneath (`SidebarPane`): no press, hover or wheel the list leaves
    // falls through to the filter, its fold block or the sections (rules-refs/app-ui.md の `RecoverPane` の行).
    MouseArea {
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.AllButtons
        onWheel: wheel => wheel.accepted = true
    }

    // ---- the band: the seat itself at the head, as the log's band carries `>_` (CommandsPane) ----
    // The filter band's height, which it stands over. **The whole band is the way out**: a press anywhere on it closes
    // the list, and the hand anywhere on it lights the `✕` it ends in.
    Rectangle {
        id: band
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: Theme.headerHeight
        color: Theme.bgElevated
        RowLayout {
            anchors.fill: parent
            spacing: Theme.spaceXs
            RecoverToggle {
                id: bandToggle
                Layout.fillHeight: true
                captioned: true
                ruled: false
                curPage: pane.curPage
            }
            // Read again over the entries already up: the ring beside the heading (`WipBucketHeader`), a sign after
            // its words — as tall as their capitals, turning about their own `+`'s middle (デザイン規約 §アイコンの席
            // の修飾). Centred in the band as the toggle is, then moved onto that middle.
            SpinnerIcon {
                id: rereadRing
                wordSize: bandToggle.captionSize
                transform: Translate { y: Math.round(bandToggle.captionSignMiddle - bandToggle.height / 2) }
                // A word's step after the caption: the toggle ends a step past it already, so the row's own step is
                // taken back.
                Layout.leftMargin: -Theme.spaceXs
                Layout.preferredWidth: rereadRing.width
                Layout.preferredHeight: rereadRing.height
                spinning: pane.reading && pane.entries.length > 0
            }
            Item { Layout.fillWidth: true }
            // The fold block's seat at the band's end (`SidebarFilterRow`).
            CloseToolButton {
                id: closeMark
                Layout.fillHeight: true
                seat: Theme.headerHeight
                pointedAt: bandHand.containsMouse
                Accessible.name: qsTr("Close the discards")
                onClicked: pane.curPage.toggleRecover()
            }
        }
        // Over the seat's caption, under the `✕`, which keeps its own press.
        MouseArea {
            id: bandHand
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            anchors.right: parent.right
            anchors.rightMargin: closeMark.width
            hoverEnabled: true
            onClicked: pane.curPage.toggleRecover()
        }
    }
    Rectangle {
        id: bandRule
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: band.bottom
        height: Theme.borderWidth
        color: Theme.borderSubtle
    }

    // ---- the filter, a row of its own under the band: the left menu's own field (`SidebarFilterRow`) ----
    Item {
        id: filterRow
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: bandRule.bottom
        height: Theme.headerHeight
        TextField {
            id: filterField
            anchors.fill: parent
            onTextChanged: pane.refilter()
            font.pixelSize: Theme.fontMd
            // At the band's caption's x, as the left menu's field sits at its section headers'.
            leftPadding: Theme.spaceXs
            rightPadding: Theme.spaceXs
            topPadding: 0
            bottomPadding: 0
            placeholderText: qsTr("Filter")
            background: null
            // The product's right-click menu, not the style's, and the menu key's (`FieldMenuSeat`).
            ContextMenu.menu: null
            ContextMenu.onRequested: filterMenu.offer()
            Keys.onMenuPressed: event => event.accepted = filterMenu.offer()
            FieldMenuSeat {
                id: filterMenu
                editor: filterField
            }
        }
        BandRule {
            color: filterField.activeFocus || filterMenu.holding ? Theme.borderFocus : Theme.borderSubtle
        }
    }

    AppListView {
        id: list
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: filterRow.bottom
        anchors.bottom: parent.bottom
        model: pane.shownEntries
        // The rows moved under the hand: the reader's wheel and bar (`HandWatch.rowsMoved`) — the hand the rows' tips
        // read too (`TipAsk.hand`). The list put back after a read tells the hand itself (`rowKeep`).
        rowsHand: paneHand
        onContentYChanged: if (!rowKeep.busy) paneHand.rowsMoved()
        delegate: EntryRow {
            pickedIndex: pane.picked
            page: pane.curPage
            host: pane
            now: pane.now
            // By the pane's name: inside the component, `paneHand` is the row's own property.
            paneHand: pane.hand
        }
    }
    // The first read, before any entry: a ring in a row's height, then the rows (`AppCombo`).
    SpinnerIcon {
        x: (pane.width - width) / 2
        y: filterRow.y + filterRow.height + (Theme.rowHeight - height) / 2
        width: Theme.iconMd
        height: Theme.iconMd
        spinning: pane.reading && pane.entries.length === 0
    }
    // An answer with nothing in it, or a filter that leaves nothing: not a row, so centred where the ring stood, as the
    // command log's empty list says its line (`CommandsPane`).
    Label {
        visible: pane.curPage !== null && pane.curPage.recoverAnswered && pane.shownEntries.count === 0
        x: (pane.width - width) / 2
        y: filterRow.y + filterRow.height + (Theme.rowHeight - height) / 2
        // The words list and search boxes everywhere answer an empty search with (VS Code, Windows, Chrome).
        text: pane.entries.length === 0 ? qsTr("Nothing was thrown away here") : qsTr("No results found")
        font.pixelSize: Theme.fontMd
        color: Theme.textMuted
    }
    // A read that failed, in git's words: the list holds nothing — no rows of an earlier read left to pick.
    Column {
        visible: pane.curPage !== null && pane.curPage.recoverFailed
        x: Theme.spaceXs
        y: filterRow.y + filterRow.height + Theme.spaceXs
        width: pane.width - 2 * Theme.spaceXs
        spacing: Theme.spaceXs
        Label {
            width: parent.width
            text: qsTr("What was thrown away could not be read")
            font.pixelSize: Theme.fontMd
            color: Theme.danger
            wrapMode: Text.Wrap
        }
        Label {
            width: parent.width
            text: pane.curPage !== null ? pane.curPage.recoverError : ""
            font.pixelSize: Theme.fontSm
            color: Theme.textSecondary
            wrapMode: Text.Wrap
        }
    }

    RecoverHoverCard {
        id: entryCard
        /// The key of the entry it words (`DiscardModel.keyAt`); "" while it is down.
        property string key: ""
        textWidth: pane.width
        onClosed: entryCard.key = ""
    }
    readonly property HoverSeat entryKeep: HoverSeat {
        id: entryKeep
        hand: paneHand
        card: entryCard
        // Behind a menu of the page's the card goes the moment it stands, and nothing opens while it does
        // (デザイン規約 §メニュー「メニューが立っている間、その裏の hover は引っ込む」).
        blockedBy: pane.curPage !== null && pane.curPage.menuStanding ? "menu" : ""
        onOpenAsked: target => pane.openCard(target.row, target.x)
    }
    // The rows moved under the hand: the card goes with its row's place — but for a read that moved them with the hand
    // inside the card, which stays where it stands (デザイン規約 §hover のツールチップ「外乱では閉じない」).
    Connections {
        target: paneHand
        function onRowsShifted() {
            if (!(paneHand.disturbed && entryKeep.cardPointed))
                entryKeep.displace("slid")
        }
    }

    // One operation, in a left menu row's columns: its mark in the seat, what it did where the names begin, when at the
    // right edge. An inline component has its own ids, so the list's state comes in as properties.
    component EntryRow: Rectangle {
        id: row
        /// The entry across reads, and where it stands in the whole list (`shownEntries`).
        required property string key
        required property int at
        /// Its place in the rows the filter leaves, which a read's keeper brings it back by (`RowKeep.land`).
        required property int index
        property int pickedIndex: -1
        property var page: null
        /// The list, whose entries, words for a time and card the row uses.
        property var host: null
        property real now: 0

        /// The entry as the list words it.
        readonly property var entry: row.host !== null ? row.host.entryOf(row.key, row.at) : ({})
        readonly property bool isPicked: row.pickedIndex === row.at

        width: ListView.view ? ListView.view.width : 0
        height: Theme.rowHeight
        color: row.isPicked ? Theme.bgSelected : row.handOn ? Theme.bgHover : "transparent"

        /// The pane's hand (`HandWatch`), which the row reads its light and its ask off by geometry each time the
        /// hand or the rows move: a row brought under a still hand is the row under it, and what it asks for waits on
        /// the seat for the hand (as the graph's rows, `GraphRowDelegate.syncHand`).
        property HandWatch paneHand: null
        property real pointerRowX: -1
        readonly property int handMoves: row.paneHand !== null ? row.paneHand.moves : 0
        onHandMovesChanged: row.syncHand()
        readonly property int handShifts: row.paneHand !== null ? row.paneHand.shifts : 0
        onHandShiftsChanged: row.syncHand()
        // The row's place in the list changed with the list where it was (a filter typed): told as a move
        // (`GraphRowDelegate.placeMoved`). A read's is the list's to weigh as it lands (`rowKeep`).
        onYChanged: row.placeMoved()
        function syncHand() {
            const hand = row.paneHand
            if (hand === null || !hand.known || !row.visible || row.pooled) {
                row.pointerRowX = -1
                return
            }
            const view = row.ListView.view
            if (view) {
                const v = view.mapFromItem(null, hand.at.x, hand.at.y)
                if (v.x < 0 || v.y < 0 || v.x >= view.width || v.y >= view.height) {
                    row.pointerRowX = -1
                    return
                }
            }
            const p = row.mapFromItem(null, hand.at.x, hand.at.y)
            row.pointerRowX = p.x >= 0 && p.y >= 0 && p.x < row.width && p.y < row.height ? p.x : -1
        }
        function placeMoved() {
            const hand = row.paneHand
            if (hand === null || !row.visible || row.pooled || (row.host !== null && row.host.rowKeep.busy))
                return
            if (!hand.known) {
                hand.rowsMoved()
                return
            }
            const p = row.mapFromItem(null, hand.at.x, hand.at.y)
            const over = p.x >= 0 && p.y >= 0 && p.x < row.width && p.y < row.height
            if (over !== (row.pointerRowX >= 0))
                hand.rowsMoved()
        }
        // The card opens on a rest, as a graph row's does (規約 §hover のツールチップ「補足は待ってから開く」), and goes
        // a beat after the hand has left the row and the card both: the pane's seat counts both (`pointEntry`).
        readonly property bool handOn: row.pointerRowX >= 0
        onHandOnChanged: {
            if (row.host !== null)
                row.host.pointEntry(row, row.handOn, Math.max(0, row.pointerRowX))
        }
        // A row pooled is off screen; one handed another entry is another row: what the hand found on it does not
        // carry over, and nothing lights or opens until the hand moves. Pooled, it stays `visible` where it last stood
        // (`GraphRowDelegate.pooled`).
        property bool pooled: false
        ListView.onPooled: {
            row.pooled = true
            row.pointerRowX = -1
            // The entry it stood for left the list or the screen; its words' tip goes with it, but for the box the hand
            // is reading (`Tips.dropIn`).
            Tips.dropIn(row)
        }
        ListView.onReused: row.pooled = false
        onKeyChanged: row.pointerRowX = -1
        TapHandler {
            onTapped: row.page.pickRecover(row.at)
        }

        // The mark of what it took and where, in the seat a left menu row draws its own in (`NameCell`): the left
        // menu's tint for that kind (`RecoverEntries.tintOf`).
        RecoverMark {
            id: entryMark
            // A lone mark's box centred on the seat as `NameCell` centres its own; a pair starts the seat.
            x: Theme.spaceXs - (row.entry.paired ? 0 : (Theme.iconSm - Theme.iconXs) / 2)
            // The head of the row's words, on their capitals' middle (デザイン規約 §アイコンの席 の文頭記号).
            y: Math.round(title.y + title.baselineOffset - WordFace.capitalMiddle(entryMark.size)
                          - entryMark.height / 2)
            size: title.pixelSize
            kind: row.entry.mark
            tint: row.entry.tint
            badged: row.entry.badged
            paired: row.entry.paired
        }
        // What was done to what, after the seat every left menu row opens with (`NameCell.seatSize`), the names as
        // code with no mark of their own (デザイン規約 §ref の種別「破棄記録は種類を語で言わない」), one line cut at the
        // end (`CodeLine`). The whole of it is the card's and the band's. A pair of marks is wider than the seat, and
        // that row's words move over by the difference.
        CodeLine {
            id: title
            x: Theme.spaceXs + (row.entry.paired ? entryMark.pairSpread : 0) + Theme.iconXs + Theme.spaceXs
            y: 0
            width: whenLabel.x - Theme.spaceXs - title.x
            height: Theme.rowHeight
            text: row.entry.listTitle
            marks: row.entry.listMarks
            pixelSize: Theme.fontMd
        }
        Label {
            id: whenLabel
            anchors.right: parent.right
            anchors.rightMargin: Theme.navBarGutter
            y: (Theme.rowHeight - whenLabel.height) / 2
            text: row.host !== null && row.host.words !== null ? row.host.words.ago(row.entry.at, row.now) : ""
            font.pixelSize: Theme.fontSm
            color: Theme.textMuted
        }
    }
}
