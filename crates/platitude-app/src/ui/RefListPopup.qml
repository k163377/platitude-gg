pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

// The refs a graph row has, unstacked: every one on a row of its own — branches first, then the tags the chip had no
// room for. The branches are what can be moved to; the tags are read here and nowhere else.
//
// **It opens over the chip** (規約 §グラフ行のダブルクリック): the owner lands the first row on the chip's own seat, so
// the name the chip was showing is not written out a second time, and the sheets behind it go with it — the stack is
// what this card stands in for, so the row it stands on stops drawing one. What is left reads as the chip's own
// frame widening into the graph, with the rest of the names stacked under it. **Always that way** — see the owner
// on why the side is fixed.
//
// **The rows scroll, and the card stops at the page.** A commit can wear more names than any window can seat at once
// (`JetBrains/kotlin` deepest: 42, and a 900px page takes 36), and what a card taller than the page loses is the only
// thing it was opened to give — the names below the fold, with no way to reach them. Scrolling costs the card the one
// thing a `Column` gave it for free, which is knowing how wide its widest row is; what it is measured over instead is
// the rows the page can seat (`layOutRows`).
//
// Owned by the page — delegates are recycled out from under an open popup, which is why the context menus live there
// too. It is a popup for the same reason a menu is: anything declared inside the list is clipped by it and painted
// under the row below.
AppCard {
    id: refList

    /// The chips (`encode::Chip` — `kind`, `name`, `isHead`, `held`, … — see `RefChip`), as shown.
    property var records: []
    /// One entry per record: the ref this one reads, or the one that reads it, where that counterpart is not a row of
    /// this card — `{mark, markTint, text, tone, ahead, behind}`, or null where the record has no counterpart to
    /// name. Worked out as the card opens (`RowHoverHost.openRefList`): the card is measured over its rows in that
    /// same turn, so a fact arriving later would be one the card has no room for.
    property var mates: []
    /// The branch the working tree is on; that one leads nowhere.
    property string currentBranch: ""
    /// One was double-clicked: that is where the reader is going. The whole chip, so its kind travels with it.
    ///
    /// **A single click stays put.** The card lands on the chip's own seat and opens on a rest, so a hand that
    /// stopped over a branch has one under it a beat later — with a click to go, the click a reader aims at the chip
    /// switches the working tree instead. These rows answer a click the
    /// way every other ref row in the app does (デザイン規約 §左メニューの所作: 行き先はダブルクリック、名前は間を空けた 2 回目).
    signal picked(var chip)
    /// One was clicked once, plainly: the row it is on becomes the one being read. **The second click has no signal
    /// of its own** — the wait it opens belongs to the graph, and so does the box it turns into (`rowClicks`) — and
    /// neither has a held click: that one goes in at the graph's own row (`rowClicks.heldRowClick`).
    signal chose(var chip)
    /// One was right-clicked: its menu is asked for, the same one the chip itself answers with. The rows that lead
    /// nowhere still have one — a tag goes nowhere but deletes fine — except the marker, which names no ref at all.
    signal menuAsked(var chip)

    /// What a row has to divide between the chip and the reading's remote. **Handed over by the owner, measured from
    /// where this opens** (規約 §hover のツールチップ「hover で開いたものの幅は、開く位置から測る」) — a card that starts partway
    /// across the page has only what is left beyond that point, and one sized to the pane is placed inside the window
    /// by `Popup` without ever being narrowed, so it ends up flat against an edge with its anchor nowhere near the
    /// chip.
    ///
    /// Both strings on a row are ref components and can run to the same wall (250 bytes, measured), so a row that just
    /// added them together grew until it left the window: a wall-length tag beside a 90-byte remote already reached
    /// edge to edge, and the remote may be as long as the tag.
    property real chipRoom: Metrics.labelColW

    /// How tall the page can let this card stand — handed over by the owner, and the whole of what the list scrolls
    /// inside. **This is the card's ceiling.** One that ran past the page put its lower rows where nothing could
    /// reach them: `JetBrains/kotlin` wears 42 refs on its deepest commit and a 900px page seats 36, so the chip said
    /// `+41` and then showed 35 of them.
    property real listRoom: 0

    /// The card's own size once the rows have been laid out, for an owner placing it against the chip. Read off the
    /// rows, since `width` / `height` are only settled by a `Popup` when it is shown.
    readonly property real cardWidth: Math.max(refList.minRowWidth, rows.rowsWidth + 2 * refList.padding)
    readonly property real cardHeight:
        refList.measuring ? refList.listRoom
                          : Math.min(rows.rowsHeight + 2 * refList.padding, refList.listRoom)
    /// True only for the turn [`layOutRows`] runs in, while the card is still down: the list is handed the whole page
    /// so that it builds every row the page could seat, which is the set the card is measured over.
    property bool measuring: false
    /// How far a chip stands off the top of its row — the room a `rowHeight` row leaves around a one-line chip, halved.
    /// A chip that wrapped keeps the same margin above and below and takes the extra height for itself.
    readonly property real chipInset:
        Math.round((Theme.rowHeight - (Theme.fontChipLine + 2 * Theme.borderWidth)) / 2)
    /// How a click on these rows is answered, and the commit they are all on — the graph pane's own four
    /// (`noteRowClick` / `dropRowRename` / `rowRenameArmed` / `rowClickGuarded`), handed over by the owner.
    ///
    /// **This card's rows are the graph's rows.** It opens on the chip's own seat once the pointer has rested, so a
    /// reader clicking that spot twice clicks the row the first time and this card the second: an answer of its own
    /// would make that second click a first one, and the gesture would read as "sometimes it does nothing". It is one
    /// target either way — the card's first row *is* the chip (規約 §グラフ行のダブルクリック) — so it goes through one
    /// door.
    property var rowClicks: null
    property string rowOid: ""
    /// The narrowest this card may come out — the chip it is covering, handed over by the owner.
    ///
    /// **The card is at least as wide as what it stands on.** Its rows are measured to their own names, and a card
    /// sized only to those can come out narrower than the chip it is covering and leave a sliver of the frame it is
    /// replacing showing past its edge (measured at two pixels).
    property real minRowWidth: 0

    // Sized here: a recycling list has no implicit height of its own to hand a popup.
    width: refList.cardWidth
    height: refList.cardHeight
    // **The rows run to the inside of the frame.** What a row wears while the pointer is on it is that row's own
    // target painted whole, the way the toolbar's cells wear theirs (デザイン規約 §当たり判定 —「重ね色の方はセル全体を塗る
    // = 的は隣と同じ」). The air around a chip is the row's own and stays there (see the rows), so nothing is left for
    // the card to hold. **A border's worth**: the menu's `spaceXs` band keeps a highlighted first or last row clear
    // of a rounded corner (`AppMenu`), and this card answers that with its corner instead.
    padding: Theme.borderWidth
    // The chip's own corner. This card is the chip's frame widening into the graph
    // (デザイン規約 §グラフ行のダブルクリック) and its rows reach that frame, so a corner rounder than a row's would be a
    // curve the wash has to be cut by.
    faceRadius: Theme.radiusSm
    // Nothing stands between the chip and this: the pointer has to be able to walk down into it without leaving both.
    margins: 0
    // The hand walks down off the chip into this and picks a row, so both halves of `AppCard.pointerInside` are wanted
    // — this is the list the pair was measured on.
    tracksPointer: true
    contentPointed: contentHover.hovered

    /// The row holding the `i`th record, brought on screen first. **A recycling list has no item for a row that is not
    /// shown** — `itemAtIndex` answers null for it — and a run that names a row means that row whether or not the list
    /// happens to be standing on it, so this walks the list there the way an arrow key would.
    function rowAt(i) {
        if (i < 0 || i >= refList.records.length)
            return null
        rowsList.positionViewAtIndex(i, ListView.Contain)
        rowsList.forceLayout()
        return rowsList.itemAtIndex(i)
    }

    /// The gesture put in at one of the rows, and what that row is holding — an automation-only exposure, the same one
    /// the sidebar's sections give (`NavList.clickRow` / `rowArmed` / `rowGuarded`): a run has no pointer to press
    /// with, and a copy of what the row would have decided would prove nothing about the row.
    function clickRow(i) {
        const row = refList.rowAt(i)
        if (!row)
            return false
        row.leftClick(Qt.NoModifier)
        return true
    }
    function doubleClickRow(i) {
        const row = refList.rowAt(i)
        if (!row)
            return false
        row.doubleClick()
        return true
    }
    /// The right-click on one of these rows, as the row answers one: the graph row's own menu, aimed at this name
    /// (デザイン規約 §グラフ行の右クリック).
    function menuRow(i) {
        const row = refList.rowAt(i)
        if (!row)
            return false
        row.rightClick()
        return true
    }
    /// The pointer coming to rest on one of these rows, and what that row is wearing because of it — hover cannot be
    /// injected (verify-ui §hover の絵の撮り方), so a run writes the row's own `pointedAt` and reads the wash back off
    /// the row itself (PGG_AUTO_ACT=ref-list-lit).
    function pointRow(i) {
        const row = refList.rowAt(i)
        if (!row)
            return false
        row.pointedAt = true
        return true
    }
    /// A held click put in at one of these rows — the choice moving the way the graph's row moves it — an
    /// automation-only exposure like `menuRow`: the press goes in at this row's own handler, which hands it to the
    /// graph's.
    function chooseRow(i, modifiers) {
        const row = refList.rowAt(i)
        if (!row)
            return false
        row.leftClick(modifiers)
        return true
    }
    function rowLit(i) {
        const row = refList.rowAt(i)
        return row ? row.lit : false
    }
    /// Which kind of row that was: a row with nowhere to go wears the same wash as one that leads somewhere, so a run
    /// that means to prove it has to say which it aimed at.
    function rowLeadsNowhere(i) {
        const row = refList.rowAt(i)
        return row ? row.leadsNowhere : false
    }
    function rowArmed(i) {
        const row = refList.rowAt(i)
        return row && refList.rowClicks ? refList.rowClicks.rowRenameArmed(row.modelData) : false
    }
    function rowGuarded(i) {
        return refList.rowClicks ? refList.rowClicks.rowClickGuarded : false
    }
    /// Automation only: the list itself, for a run that sends it by its own hand and reads where it went.
    readonly property alias list: rowsList

    /// Lays the rows out and measures them now, for an owner that is about to show this in the same turn it handed
    /// over the records. Without it the card is shown at the size it had before the records arrived — measured at 8x8,
    /// its padding and nothing else, growing to the real 111x80 a frame later. That matters because **Qt works out
    /// what is hovered from pointer events, not from geometry**: a card that grows after it appears cannot tell that
    /// the hand is already inside it, and the hand that walked down off the chip is exactly that hand (traced: the
    /// card took itself down under the pointer).
    ///
    /// **The rows it measures are the rows the list is going to show** — the card is given the page for the length of
    /// this call, the list builds what fits in it, and those are read back. Nothing is built twice: a second set laid
    /// out beside the list to be measured spent the whole 100ms a hand is answered in on the deepest row of
    /// `JetBrains/kotlin` (ci/baseline/code-costs-windows-x64.md), for rows the list was about to build anyway.
    ///
    /// A card that fits is measured whole and comes out at exactly the width it always had. One that does not is
    /// measured over the rows the reader has in front of them the moment it opens, and a longer name below the fold
    /// wraps — which is what a name too long for the room does here anyway (規約 §グラフ行のダブルクリック「カードの中の名前は
    /// 切らない」). **The odds are with it**: a commit stacked deep enough to scroll is rare to begin with, and the
    /// branches that sort to the head of the list already take most of the width a tag further down would have asked
    /// for. Rows past the fold are counted at a plain row apiece for the height — they can only ever be taller than
    /// that, so a card they would have filled is one already standing at its cap.
    function layOutRows() {
        refList.measuring = true
        rowsList.forceLayout()
        let widest = 0
        let stacked = 0
        let seen = 0
        for (let i = 0; i < refList.records.length; i++) {
            const row = rowsList.itemAtIndex(i)
            if (!row)
                break
            // **Laid out before it is read.** A row built before the card handed over its room — the first one is,
            // every time: the list builds it the moment the records land, and the room is set after that — carries
            // the chip at the width the graph's column left it, and the frame's own row is a positioner, so its sum
            // is a pass behind the room it was given. Read unasked, the card came out at the column's width and cut
            // the name off inside its own frame (`RefChip.layOutNow`).
            row.layOutNow()
            widest = Math.max(widest, row.implicitWidth)
            stacked += row.height
            seen = i + 1
        }
        rows.rowsHeight = stacked + Math.max(0, refList.records.length - seen) * Theme.rowHeight
        // In that order: whether the bar comes out is what the far side of a row is spaced by, and that is part of
        // the width (規約 §QML 実装ルール のバーの選び方 — a trough is the bar's).
        rows.rightInset = rows.rowsHeight + 2 * refList.padding > refList.listRoom
                            ? Theme.navBarGutter : Theme.spaceXs
        rows.rowsWidth = widest + rows.rightInset
        refList.measuring = false
    }

    contentItem: Item {
        id: rows
        // The rows' half of `pointerInside`; the rows are its children, so their own hover leaves this one standing.
        HoverHandler {
            id: contentHover
        }

        /// What the rows came to, taken once by [`layOutRows`] and held: the widest of them, which is the width every
        /// row is then laid out to (their highlights have to line up, and so do the chips now that they can stand
        /// flush against either edge), and what they stack up to.
        ///
        /// **Held, because the rows a recycling list has are not the rows it holds.** Left as a binding these would
        /// be read off whatever the list had built at the time, and the card would breathe as the reader
        /// scrolled.
        property real rowsWidth: 0
        property real rowsHeight: 0
        /// What a row keeps between its far end and the card's frame, taken with the width above (a bar that comes out
        /// wants its trough counted, and one that does not would leave the air standing empty).
        ///
        /// **The bar this list wears takes a trough** (規約 §QML 実装ルール のバーの選び方) — the far end of a row is
        /// where the reading's remote stands, and right-aligned ink is exactly what cannot be left under a
        /// see-through thumb. So the ink stops at the pane's own gutter and the slab stands in it, the way it does
        /// down the left menu and the file lists. Where no bar comes out there is no trough to keep, and the row ends
        /// at the ordinary gap (規約 §QML 実装ルール「バーの出ない帯は右も `spaceXs`」).
        property real rightInset: Theme.spaceXs

        AppListView {
            id: rowsList
            anchors.fill: parent
            model: refList.records
            // The panels' slab (above). Its idle step is counted from the ground it stands on, and this card's
            // ground is `bgElevated` — the same one the settings screen hands this colour in for
            // (規約 §ペインのスクロールバー).
            verticalBar: PaneScrollBar {
                idleColor: Theme.borderSubtle
            }
            delegate: Rectangle {
                id: refRow
                required property var modelData
                /// Which record this row draws — what its counterpart is looked up by (`refList.mates`).
                required property int index
                // The row now holds a different ref: whatever was resting on the one before it is not resting on
                // this. A recycled delegate is the same object with new data (`AppListView.reuseItems`), and the
                // wash is written, so nothing else would take it off.
                onModelDataChanged: refRow.pointedAt = false
                /// The pointer resting on this row, for the runs that photograph the wash — hover cannot be injected
                /// (verify-ui §hover の絵の撮り方), and the same property a real pointer writes is the only place a run
                /// may write. Named for the sidebar's own (`NavItemDelegate.tipPointedAt`).
                property bool pointedAt: false
                /// Wearing the hover wash. **Every row wears it, whether or not it leads anywhere** (規約 §グラフ行の
                /// ダブルクリック) — the pointer is on a target either way (every row takes the click, and all but the
                /// marker have a name to change), and a list where some rows answer the hand and others do not reads
                /// as one that sometimes stops working. Where a row leads is said by the colour of its name, as it is
                /// in the sidebar, whose rows light the same way.
                readonly property bool lit: rowHover.hovered || refRow.pointedAt
                // The branch the working tree already stands on is where the reader is, and it says so in the colour
                // the sidebar says it in (§ref の種別). Only the move comes off. A tag leads nowhere for its own
                // reason (§タグのダブルクリックは入力欄へ) and keeps its colour too.
                readonly property bool current:
                    refRow.modelData.kind === "branch" && refRow.modelData.name === refList.currentBranch
                // The two markers are the rows that are only markers: nowhere to go and no ref to read — the detached
                // HEAD, and a working copy standing on this commit with no branch of its own. A branch another
                // working copy has out is unavailable for the other reason there is — git refuses the move outright
                // (§無効 is for what is actually unavailable, and this one is).
                //
                // **The chip's colour is left alone here.** Two of the three already say where they lead in their own
                // record — the green frame of a copy standing there, wherever it is drawn — and the detached HEAD's
                // colour is a state rather than a kind: muting that one here would make the same marker amber on the
                // row and grey in the card it unfolds into. What this answers is the move and the menu, below.
                readonly property bool unavailable:
                    refRow.modelData.kind === "head" || refRow.modelData.kind === "worktree" || refRow.modelData.held
                readonly property bool leadsNowhere:
                    refRow.unavailable || refRow.current || refRow.modelData.kind === "tag"

                /// Lays this row's chip out now, so that the width the card's measuring pass reads off it is the one
                /// the room it has just been given asks for (`layOutRows`; `RefChip.layOutNow` says why it is a pass
                /// behind without this).
                function layOutNow() {
                    rowChip.layOutNow()
                }
                /// What the reading's remote costs this row: its own ink and the gap that keeps it off the name.
                readonly property real whoseRoom: whose.visible ? whose.width + Theme.spaceSm : 0
                /// The counterpart this row names under itself, or null where it has none (`refList.mates`). Drawn
                /// inside the chip's own frame (`RefChip.mate`), which is what makes the chip taller here.
                readonly property var mate:
                    refRow.index >= 0 && refRow.index < refList.mates.length
                        ? refList.mates[refRow.index] : null
                /// What is left for the chip. **The page while the card is being measured, the card itself once it
                /// has a width.** The measuring pass is where the card decides how wide to be, so the rows it reads
                /// there ask for everything the page leaves them (`refList.chipRoom`) — but a row built after it, one
                /// below the fold that a scroll brings up, was in no reading at all, and a chip still asking for the
                /// page is drawn wider than the card and **cut by its frame** (observed on a card of forty-three
                /// names, scrolled to the long one under them). Held to the card, the name wraps, which is what this
                /// card does with a name too long for its room.
                readonly property real chipRoom: {
                    const page = refList.chipRoom - refRow.whoseRoom
                    if (refList.measuring)
                        return page
                    return Math.min(page, rows.width - rows.rightInset - Theme.spaceXs - refRow.whoseRoom)
                }

                // **The row is the band; the chip stands in it.** The air on either side of the chip is the row's own,
                // and the card's frame ends where the band begins, so the wash the pointer puts on a row covers the
                // whole of what that row answers to (規約 §当たり判定). What stands between a chip's frame and the card's
                // is that air and nothing else (規約 §グラフ行のダブルクリック — the card lands on the chip column's own edge,
                // and a wider one would reach past the divider into the lanes).
                implicitWidth: Theme.spaceXs + rowChip.width + refRow.whoseRoom
                width: rows.width
                // The chip sets the height, so a wrapped name — or one carrying the line its reading opens under it
                // — makes its own row taller and leaves the others alone.
                height: rowChip.height + 2 * refList.chipInset
                radius: Theme.radiusSm
                color: refRow.lit ? Theme.bgHover : "transparent"

                RefChip {
                    id: rowChip
                    y: refList.chipInset
                    // One gap in from the band's own edge, and the card is placed so that puts this where the chip it
                    // stands in for was: every name in the list starts where the chip's own name started — the head of
                    // a name is what it is told apart by, so that is the edge that has to hold still
                    // (規約 §グラフ列は最も広い所のレーンまで).
                    x: Theme.spaceXs
                    records: [refRow.modelData]
                    // The same wash the row's own chip wears while a second click waits out its window — this card is
                    // that chip, so the mark is on whichever of the two the reader is looking at (規約 §グラフ行のダブルクリック).
                    waiting: refList.rowClicks ? refList.rowClicks.rowRenameArmed(refRow.modelData) : false
                    // Unstacking is only worth it if the names read, so the chip takes everything the row has left
                    // once the reading's remote has its seat — and what will not fit even then is wrapped: this is
                    // the one place the name is shown in order to be read (規約 §hover のツールチップ).
                    wrapped: true
                    maxWidth: refRow.chipRoom
                    // What this name reads, or what reads it, in the same frame under the name (`RefChip.mate`).
                    mate: refRow.mate
                }
                // Whose reading this is. A tag has no namespace to say it in the way `origin/main` does, and a
                // drifted one puts the same bare name on two rows — this card is where the two meet, so it is where
                // the question gets answered. Meta about the row, so it is written in the colour the row's other
                // meta is (§ref の種別).
                Label {
                    id: whose
                    visible: rowChip.recWhere !== ""
                    // At the far end of the row, away from the names: put beside its own chip it would sit at a
                    // different distance on every row, and the reading is meta. It stops at the bar's gutter, which
                    // is what that bar takes (`rows.rightInset`).
                    x: refRow.width - whose.width - rows.rightInset
                    // Level with the chip's first line: a wrapped name takes the row down with it and this is meta
                    // about the name's first line.
                    y: refList.chipInset + Theme.borderWidth
                       + Math.round((Theme.fontChipLine - whose.height) / 2)
                    text: rowChip.recWhere
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontSm
                    // A third of the row at most: this is meta about the reading, and the name it qualifies comes
                    // first.
                    width: Math.min(implicitWidth, refList.chipRoom / 3)
                    elide: Text.ElideRight
                }
                // A name that can be changed: every kind of ref has one, and only the detached-HEAD marker names no
                // ref to change. **Its own question, apart from `leadsNowhere`** — the branch the reader is on
                // and the tag that is a mark both keep their names (the same split the sidebar's rows make).
                readonly property bool nameable: GitFacts.refKind(refRow.modelData.kind) !== ""
                /// A left click and a double-click on this row, as the row answers them. Named so that a run with no
                /// pointer to press with puts its clicks in at the row itself
                /// (PGG_AUTO_ACT=graph-reclick-list / ref-list-pick).
                ///
                /// **Every row takes the click**, whether or not it leads anywhere: what a row that leads nowhere
                /// still has is a name, and the gesture that changes it begins with a click of its own.
                function leftClick(modifiers) {
                    const mods = modifiers === undefined ? Qt.NoModifier : modifiers
                    // **A held click is the graph row's own** (デザイン規約 §複数のコミットを選ぶ): this card's rows are
                    // that row, so the press goes in at the row's handler — what it does with the choice, the name
                    // box and the keyboard is decided there and nowhere else.
                    if (mods & (Qt.ControlModifier | Qt.ShiftModifier)) {
                        if (refList.rowClicks)
                            refList.rowClicks.heldRowClick(refList.rowOid, mods)
                        return
                    }
                    // Answered by the row this card is standing on — a name that leads nowhere is still a name, but
                    // the marker names no ref, so it goes in as a row with nothing on it.
                    if (refList.rowClicks
                            && !refList.rowClicks.noteRowClick(
                                refList.rowOid, refRow.nameable ? refRow.modelData : null))
                        return
                    if (refRow.nameable)
                        refList.chose(refRow.modelData)
                }
                function doubleClick(modifiers) {
                    const mods = modifiers === undefined ? Qt.NoModifier : modifiers
                    // **A held double-click is two selection presses** (デザイン規約 §複数のコミットを
                    // 選ぶ) — the row's rule (`GraphRowDelegate.doubleClick`), and this card is a third way to the same
                    // `switch`, which moves the working tree. The two presses have already done what they do.
                    if (mods & (Qt.ControlModifier | Qt.ShiftModifier))
                        return
                    // The second click came inside the window after all: the gesture was the double-click, and the box
                    // it was about to open is not what was meant.
                    if (refList.rowClicks)
                        refList.rowClicks.dropRowRename()
                    if (refRow.leadsNowhere)
                        return
                    refList.close()
                    refList.picked(refRow.modelData)
                }
                HoverHandler {
                    id: rowHover
                }
                // The whole row answers, the bar's strip included: the bar is a child of the view rather than of
                // these rows, so it is over them and takes its own presses first — the same as every row down the
                // left menu (`NavItemDelegate`). What stops at the gutter is the ink.
                TapHandler {
                    id: rowTap
                    // **At the press**, as the graph's own rows answer theirs (`GraphRowDelegate`). This card and
                    // those rows are one target — a hand pressing one spot twice presses the row and then the card —
                    // so the two have to answer the same half of the gesture or the second press comes up as a first
                    // (規約 §グラフ行のダブルクリック).
                    onPressedChanged: if (rowTap.pressed) refRow.leftClick(rowTap.point.modifiers)
                    onDoubleTapped: refRow.doubleClick(rowTap.point.modifiers)
                }
                /// A right-click on this row, as the row answers one. Named for the same reason `leftClick` is: a run
                /// with no pointer to press with puts its press in at the row itself
                /// (PGG_AUTO_ACT=list-menu).
                function rightClick() {
                    refList.menuAsked(refRow.modelData)
                }
                TapHandler {
                    acceptedButtons: Qt.RightButton
                    enabled: !refRow.unavailable
                    // The list stays: the menu opens over it, and the owner keeps the list up for as long as the menu
                    // stands (its settle checks the menu).
                    onTapped: refRow.rightClick()
                }
            }
        }
    }
}
