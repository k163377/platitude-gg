pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

// The refs a graph row has, unstacked one per row: branches first, then the tags the chip had no room for. Opens
// over the chip with its first row on the chip's own seat, and scrolls inside the page (規約 §グラフ行のダブルクリック)
// — so the width is measured over the rows the page can seat (`layOutRows`).
//
// Owned by the page: delegates are recycled out from under an open popup. A popup, because anything declared inside
// the list is clipped by it and painted under the row below.
AppCard {
    id: refList

    /// The chips (`encode::Chip` — `kind`, `name`, `isHead`, `held`, … — see `RefChip`), as shown.
    property var records: []
    /// One entry per record: its counterpart (the ref it reads, or the one reading it) where that is not a row of this
    /// card — `{mark, markTint, text, tone, ahead, behind}` — or null. Set as the card opens
    /// (`RowHoverHost.openRefList`): the card is measured in that turn, so a later fact would have no room.
    property var mates: []
    /// The branch the working tree is on; that one leads nowhere.
    property string currentBranch: ""
    /// One was double-clicked: the reader is going there. The whole chip, so its kind travels with it. A single click
    /// never moves (規約 §グラフ行のダブルクリック).
    signal picked(var chip)
    /// One was clicked once, plainly: its row becomes the one being read. The second click and a held click have no
    /// signal here — both go in at the graph's own row (`rowClicks`).
    signal chose(var chip)
    /// One was right-clicked: the chip's own menu is asked for. Rows leading nowhere still have one (a tag deletes
    /// fine); `unavailable` rows have none.
    signal menuAsked(var chip)
    /// The name a row opens under itself was pressed (`RefChip.mateFollowed`). `to` is a `NavFacts.place` on another
    /// row, so the card closes with the press.
    signal followed(var to)
    /// Whether this card stands on the current branch's stand-in (`GraphHeadPin`) rather than a row. A press on its
    /// rows is then the stand-in's (`standInPressed`), which brings the off-screen row on — the rows' own door would
    /// pick a row nobody can see.
    property bool onStandIn: false
    signal standInPressed(int modifiers)

    /// What a row divides between the chip and the reading's remote, measured by the owner from where this opens
    /// (規約 §hover のツールチップ「hover で開いたものの幅は、自分で測る」): `Popup` moves an oversized card into the
    /// window but never narrows it. Both names can be ref-length, so a row divides this rather than adding them.
    property real chipRoom: Metrics.labelColW

    /// The card's ceiling, handed over by the owner: the page's height, which the list scrolls inside. Past the page,
    /// the lower rows would be out of reach.
    property real listRoom: 0

    /// The card's size once the rows are laid out, for an owner placing it: `width` / `height` are only settled by a
    /// `Popup` when it is shown.
    readonly property real cardWidth:
        Math.max(refList.coverWidth + 2 * (Theme.spaceXs + refList.padding), rows.rowsWidth + 2 * refList.padding)
    readonly property real cardHeight:
        refList.measuring ? refList.listRoom
                          : Math.min(rows.rowsHeight + 2 * refList.padding, refList.listRoom)
    /// True only during [`layOutRows`]: the list is handed the whole page so it builds every row the page could seat
    /// — the set the card is measured over.
    property bool measuring: false
    /// A chip's margin above and below within its row; a wrapped chip keeps it and grows the row.
    readonly property real chipInset:
        Math.round((Theme.rowHeight - (Theme.fontChipLine + 2 * Theme.borderWidth)) / 2)
    /// The graph pane's click handling (`noteRowClick` / `dropRowRename` / `rowRenameArmed` / `rowClickGuarded` /
    /// `heldRowClick`) and the commit the rows are on, handed over by the owner. **These rows are the graph's row**:
    /// the card opens on the chip's seat, so a reader clicking there twice hits the row, then the card — answered
    /// apart, the second click would count as a first (規約 §グラフ行のダブルクリック).
    property var rowClicks: null
    property string rowOid: ""
    /// The width of the chip this card covers, `+N` and fan included, handed over by the owner: a floor, with a row's
    /// air on both sides, since the card is placed that air and a border left of the chip
    /// (`RowHoverHost.openRefList`) and rows measured to their names alone would leave the chip's end showing.
    property real coverWidth: 0

    // Sized here: a recycling list has no implicit height of its own to hand a popup.
    width: refList.cardWidth
    height: refList.cardHeight
    // The rows' wash reaches the frame, where a menu keeps a `spaceXs` band; the corner answers that instead
    // (規約 §グラフ行のダブルクリック「行の重ね色はカードの枠の内側いっぱいに届く」).
    padding: Theme.borderWidth
    // The chip's own corner, so the first and last rows' wash meets the frame flush (規約 §グラフ行のダブルクリック).
    faceRadius: Theme.radiusSm
    // Nothing stands between the chip and this: the pointer has to be able to walk down into it without leaving both.
    margins: 0
    // The hand walks down off the chip into this, so both halves of `AppCard.pointerInside` are wanted.
    tracksPointer: true
    contentPointed: contentHover.hovered

    /// The row holding the `i`th record, scrolled on screen first: a recycling list's `itemAtIndex` is null for a row
    /// not shown.
    function rowAt(i) {
        if (i < 0 || i >= refList.records.length)
            return null
        rowsList.positionViewAtIndex(i, ListView.Contain)
        rowsList.forceLayout()
        return rowsList.itemAtIndex(i)
    }

    /// Automation only, like `NavList.clickRow` / `rowArmed` / `rowGuarded`: the gesture goes in at the row's own
    /// handler, since a copy of what the row decides would prove nothing about the row.
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
    /// The right-click, as the row answers one (デザイン規約 §グラフ行の右クリック).
    function menuRow(i) {
        const row = refList.rowAt(i)
        if (!row)
            return false
        row.rightClick()
        return true
    }
    /// Hover cannot be injected (verify-ui §hover の絵の撮り方): a run writes the row's `pointedAt` and reads the wash
    /// back with `rowLit` (PGG_AUTO_ACT=ref-list-lit).
    function pointRow(i) {
        const row = refList.rowAt(i)
        if (!row)
            return false
        row.pointedAt = true
        return true
    }
    /// A held click, in at the row's own handler, which hands it to the graph's.
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
    /// Presses the name a row opens under itself (`RefChip.followMate`; PGG_AUTO_ACT=ref-list-follow). False where
    /// the row has nowhere to go.
    function followRow(i) {
        const row = refList.rowAt(i)
        if (!row || !row.chip.mateGoes)
            return false
        row.chip.followMate()
        return true
    }
    /// The pointer resting on that name, written where the pointer writes; read back with `mateAimed`
    /// (PGG_AUTO_ACT=ref-list-follow-lit).
    function pointMate(i) {
        const row = refList.rowAt(i)
        if (!row || !row.chip.mateGoes)
            return false
        row.chip.matePointedAt = true
        return true
    }
    function mateAimed(i) {
        const row = refList.rowAt(i)
        return row ? row.chip.mateLit : false
    }
    /// Where that row's name goes (`NavFacts.place`), null for one going nowhere.
    function mateTo(i) {
        const row = refList.rowAt(i)
        return row && row.chip.mateGoes ? row.chip.mate.to : null
    }
    /// A row going nowhere wears the same wash as one leading somewhere, so a run has to say which it aimed at.
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
    /// Automation only: the list, for a run that scrolls it and reads where it went.
    readonly property alias list: rowsList

    /// Lays the rows out and measures them now, for an owner about to show this in the turn it handed over the
    /// records. **Qt works out what is hovered from pointer events, not geometry**: a card that grows after it appears
    /// cannot tell that the hand walking down off the chip is already inside it, and closes under it.
    ///
    /// It measures the rows the list is about to show (the card is given the page for this call): a second set laid
    /// out just to measure is over the operation budget alone (ci/baseline/code-costs-windows-x64.md). A card that
    /// does not fit is measured over the rows in view, and a longer name below the fold wraps
    /// (規約 §グラフ行のダブルクリック「カードの中の名前は全文」). Rows past the fold count a plain row each for the
    /// height — they can only be taller, so such a card is already at its cap.
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
            // Laid out before it is read: the first row is built before the room is set, and the chip's positioner
            // answers a pass late (`RefChip.layOutNow`) — read unasked, the name is cut at the column's width.
            row.layOutNow()
            widest = Math.max(widest, row.implicitWidth)
            stacked += row.height
            seen = i + 1
        }
        rows.rowsHeight = stacked + Math.max(0, refList.records.length - seen) * Theme.rowHeight
        // Height before width: whether the bar comes out sets a row's far gap, which is part of the width.
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

        /// The widest row (every row is laid out to it) and the rows' stacked height, set once by [`layOutRows`]. Not
        /// bindings: a recycling list's built rows change as it scrolls, and the card would breathe.
        property real rowsWidth: 0
        property real rowsHeight: 0
        /// What a row keeps between its far end and the frame, set with the width: the bar's trough when a bar comes
        /// out, else `spaceXs` (規約 §グラフ行のダブルクリック「バーは張り付く側」).
        property real rightInset: Theme.spaceXs

        AppListView {
            id: rowsList
            anchors.fill: parent
            model: refList.records
            // The idle step counted from this card's `bgElevated` ground (規約 §ペインのスクロールバー).
            verticalBar: PaneScrollBar {
                idleColor: Theme.borderSubtle
            }
            delegate: Rectangle {
                id: refRow
                required property var modelData
                /// Which record this row draws — what its counterpart is looked up by (`refList.mates`).
                required property int index
                // A recycled delegate keeps a written wash (`AppListView.reuseItems`); a new ref starts unpointed.
                onModelDataChanged: {
                    refRow.pointedAt = false
                    rowChip.matePointedAt = false
                }
                /// The chip this row draws, for the hooks above.
                readonly property alias chip: rowChip
                /// A run's stand-in for the pointer resting on this row (`pointRow`): hover cannot be injected. Named
                /// for the sidebar's (`NavItemDelegate.tipPointedAt`).
                property bool pointedAt: false
                /// Wearing the hover wash — every row, whether or not it leads anywhere (規約 §グラフ行のダブルクリック).
                readonly property bool lit: rowHover.hovered || refRow.pointedAt
                // The reader is already there: only the move comes off, the colour stays (§ref の種別). A tag leads
                // nowhere too (§タグのダブルクリックは入力欄へ).
                readonly property bool current:
                    refRow.modelData.kind === "branch" && refRow.modelData.name === refList.currentBranch
                // The two markers (a detached HEAD, a working copy with no branch here) name no ref, and git refuses
                // the move to a branch another copy has out (§無効). Only the move and the menu come off: the chip
                // keeps its colour, or the detached HEAD would be amber on the row and grey in its card.
                readonly property bool unavailable:
                    refRow.modelData.kind === "head" || refRow.modelData.kind === "worktree" || refRow.modelData.held
                readonly property bool leadsNowhere:
                    refRow.unavailable || refRow.current || refRow.modelData.kind === "tag"

                /// For `layOutRows`: without it the chip's width is a pass behind its room (`RefChip.layOutNow`).
                function layOutNow() {
                    rowChip.layOutNow()
                }
                readonly property real whoseRoom: whose.visible ? whose.width + Theme.spaceSm : 0
                /// The counterpart named inside this row's chip frame (`RefChip.mate`), or null.
                readonly property var mate:
                    refRow.index >= 0 && refRow.index < refList.mates.length
                        ? refList.mates[refRow.index] : null
                /// What is left for the chip: the page while the card is measured, the card once it has a width. A row
                /// scrolled up after measuring was never read, and asking for the page it would be cut by the frame;
                /// held to the card, it wraps.
                readonly property real chipRoom: {
                    const page = refList.chipRoom - refRow.whoseRoom
                    if (refList.measuring)
                        return page
                    return Math.min(page, rows.width - rows.rightInset - Theme.spaceXs - refRow.whoseRoom)
                }

                // The row is the band and owns the air around its chip, so the wash covers the whole target
                // (規約 §当たり判定).
                implicitWidth: Theme.spaceXs + rowChip.width + refRow.whoseRoom
                width: rows.width
                height: rowChip.height + 2 * refList.chipInset
                radius: Theme.radiusSm
                color: refRow.lit ? Theme.bgHover : "transparent"

                RefChip {
                    id: rowChip
                    y: refList.chipInset
                    // The card is placed so this lands where the covered chip's name started: a name's head is what
                    // holds still (規約 §グラフ列は最も広い所のレーンまで).
                    x: Theme.spaceXs
                    records: [refRow.modelData]
                    // The wait mark the row's own chip wears, on whichever of the two the reader is looking at
                    // (規約 §グラフ行のダブルクリック).
                    waiting: refList.rowClicks ? refList.rowClicks.rowRenameArmed(refRow.modelData) : false
                    // Names here are shown to be read: what does not fit wraps
                    // (規約 §グラフ行のダブルクリック「カードの中の名前は全文」).
                    wrapped: true
                    maxWidth: refRow.chipRoom
                    mate: refRow.mate
                    onMateFollowed: to => refRow.follow(to)
                }
                function follow(to) {
                    refList.close()
                    refList.followed(to)
                }
                // Whose reading this is: a drifted tag puts one bare name on two rows, with no namespace to tell them
                // apart. Meta, in the row's meta colour (§ref の種別).
                Label {
                    id: whose
                    visible: rowChip.recWhere !== ""
                    // At the row's far end, so it lines up across rows; it stops at the bar's gutter.
                    x: refRow.width - whose.width - rows.rightInset
                    // Level with the chip's first line, where a wrapped name starts.
                    y: refList.chipInset + Theme.borderWidth
                       + Math.round((Theme.fontChipLine - whose.height) / 2)
                    text: rowChip.recWhere
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontSm
                    // A third at most: the name it qualifies comes first.
                    width: Math.min(implicitWidth, refList.chipRoom / 3)
                    elide: Text.ElideRight
                }
                // Whether there is a ref name to change — every row but the two markers. Apart from `leadsNowhere`:
                // the current branch and a tag lead nowhere but can still be renamed.
                readonly property bool nameable: GitFacts.refKind(refRow.modelData.kind) !== ""
                /// A left click and a double-click, named so a run puts its clicks in here
                /// (PGG_AUTO_ACT=graph-reclick-list / ref-list-pick). Every row takes the click: a row leading nowhere
                /// still has a name, and renaming starts with a click.
                function leftClick(modifiers) {
                    const mods = modifiers === undefined ? Qt.NoModifier : modifiers
                    // The press goes out before the card closes: closing forgets the row it stood on
                    // (`RowHoverHost.onClosed`), and that row is where the press goes.
                    if (refList.onStandIn) {
                        refList.standInPressed(mods)
                        refList.close()
                        return
                    }
                    // A held click is the graph row's own, decided in its handler alone (デザイン規約 §複数のコミットを選ぶ).
                    if (mods & (Qt.ControlModifier | Qt.ShiftModifier)) {
                        if (refList.rowClicks)
                            refList.rowClicks.heldRowClick(refList.rowOid, mods)
                        return
                    }
                    // A marker names no ref, so it goes in as a row with nothing on it.
                    if (refList.rowClicks
                            && !refList.rowClicks.noteRowClick(
                                refList.rowOid, refRow.nameable ? refRow.modelData : null))
                        return
                    if (refRow.nameable)
                        refList.chose(refRow.modelData)
                }
                function doubleClick(modifiers) {
                    const mods = modifiers === undefined ? Qt.NoModifier : modifiers
                    // A held double-click is two selection presses, never a `switch` (デザイン規約 §複数のコミットを選ぶ,
                    // as `GraphRowDelegate.doubleClick`).
                    if (mods & (Qt.ControlModifier | Qt.ShiftModifier))
                        return
                    // A double-click after all: drop the rename box the second click armed.
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
                // The row reaches under the bar: the bar is the view's child, over the rows, and takes its own
                // presses first (as in `NavItemDelegate`).
                TapHandler {
                    id: rowTap
                    // At the press, like the graph's rows (rules-refs/app-ui.md「左ボタンは押下で答える」).
                    onPressedChanged: if (rowTap.pressed) refRow.leftClick(rowTap.point.modifiers)
                    onDoubleTapped: refRow.doubleClick(rowTap.point.modifiers)
                }
                /// A right-click, named for runs as `leftClick` is (PGG_AUTO_ACT=list-menu).
                function rightClick() {
                    refList.menuAsked(refRow.modelData)
                }
                TapHandler {
                    acceptedButtons: Qt.RightButton
                    enabled: !refRow.unavailable
                    // The list stays up under the menu: the owner's settle checks the menu.
                    onTapped: refRow.rightClick()
                }
            }
        }
    }
}
