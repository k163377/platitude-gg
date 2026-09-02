pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

// The refs a graph row has, unstacked: every one on a row of its own — branches first, then the tags the chip had no
// room for. The branches are what can be moved to; the tags are read here and nowhere else.
//
// **It opens over the chip, not under it** (規約 §グラフ行のダブルクリック): the owner lands the first row on the chip's own
// seat, so the name the chip was showing is not written out a second time, and the `+N` goes with it — a row carries
// one record, and a chip with one record has no count to draw. What is left reads as the chip's own frame widening
// into the graph, with the rest of the names stacked under it. **Always that way** — see the owner on why the side is
// not chosen.
//
// Owned by the page, not by the delegate that raised it — delegates are recycled out from under an open popup, which is
// why the context menus live there too. It is a popup rather than an item in the row for the same reason a menu is:
// anything declared inside the list is clipped by it and painted under the row below.
AppCard {
    id: refList

    /// Chip records (kind + flags + name, see encode.rs), as shown.
    property var records: []
    /// The branch the working tree is on; that one leads nowhere.
    property string currentBranch: ""
    /// One was double-clicked: that is where the reader is going. The whole record, so its kind travels with it.
    ///
    /// **A single click does not move.** The card lands on the chip's own seat and opens on a rest, so a hand that
    /// stopped over a branch has one under it a beat later — with a click to go, the click a reader aims at the chip
    /// switches the working tree instead. These rows answer a click the
    /// way every other ref row in the app does (デザイン規約 §左メニューの所作: 行き先はダブルクリック、名前は間を空けた 2 回目).
    signal picked(string record)
    /// One was clicked once: the row it is on becomes the one being read. **The second click has no signal of its
    /// own** — the wait it opens belongs to the graph, and so does the box it turns into (`rowClicks`).
    signal chose(string record)
    /// One was right-clicked: its menu is asked for, the same one the chip itself answers with. The rows that lead
    /// nowhere still have one — a tag goes nowhere but deletes fine — except the marker, which names no ref at all.
    signal menuAsked(string record)

    /// What a row has to divide between the chip and the reading's remote. **Handed over by the owner, measured from
    /// where this opens** (規約 §hover のツールチップ「hover で開いたものの幅は、開く位置から測る」) — the pane's own width is not it: a card
    /// that starts partway across the page has only what is left beyond that point, and one sized to the pane is placed
    /// inside the window by `Popup` without ever being narrowed, so it ends up flat against an edge with its anchor
    /// nowhere near the chip.
    ///
    /// Both strings on a row are ref components and can run to the same wall (250 bytes, measured), so a row that just
    /// added them together grew until it left the window: a wall-length tag beside a 90-byte remote already reached
    /// edge to edge, and the remote may be as long as the tag.
    property real chipRoom: Metrics.labelColW

    /// The card's own size once the rows have been laid out, for an owner placing it against the chip. Read off the
    /// rows rather than off `width` / `height`, which a `Popup` only settles when it is shown.
    readonly property real cardWidth: rows.rowWidth + 2 * refList.padding
    readonly property real cardHeight: rows.implicitHeight + 2 * refList.padding
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
    /// The narrowest a row may be — the chip this card is covering, handed over by the owner.
    ///
    /// **The card is never narrower than what it stands on.** Its own rows carry one record each and so draw no `+N`,
    /// while the chip underneath may be wearing one: a card sized only to its own names can come out narrower than the
    /// chip and leave a sliver of the frame it is replacing showing past its edge (measured at two pixels).
    property real minRowWidth: 0

    padding: Theme.spaceXs
    // Nothing stands between the chip and this: the pointer has to be able to walk down into it without leaving both.
    margins: 0
    // The hand walks down off the chip into this and picks a row, so both halves of `AppCard.pointerInside` are wanted
    // — this is the list the pair was measured on.
    tracksPointer: true
    contentPointed: contentHover.hovered

    /// The gesture put in at one of the rows, and what that row is holding — an automation-only exposure, the same one
    /// the sidebar's sections give (`NavList.clickRow` / `rowArmed` / `rowGuarded`): a run has no pointer to press
    /// with, and a copy of what the row would have decided would prove nothing about the row.
    function clickRow(i) {
        const row = rowsRepeater.itemAt(i)
        if (!row)
            return false
        // Nothing was held down: a run with no pointer has no press to time (`ReclickGesture.click`).
        row.leftClick(0)
        return true
    }
    function doubleClickRow(i) {
        const row = rowsRepeater.itemAt(i)
        if (!row)
            return false
        row.doubleClick()
        return true
    }
    /// The right-click on one of these rows, as the row answers one: the graph row's own menu, aimed at this name
    /// instead of the one the chip draws (デザイン規約 §グラフ行の右クリック).
    function menuRow(i) {
        const row = rowsRepeater.itemAt(i)
        if (!row)
            return false
        row.rightClick()
        return true
    }
    function rowArmed(i) {
        const row = rowsRepeater.itemAt(i)
        return row && refList.rowClicks ? refList.rowClicks.rowRenameArmed(row.modelData) : false
    }
    function rowGuarded(i) {
        return refList.rowClicks ? refList.rowClicks.rowClickGuarded : false
    }

    /// Lays the rows out now, for an owner that is about to show this in the same turn it handed over the records. A
    /// `Column` positions in the polish that runs after the turn, so without this the list is shown at the size it had
    /// before the records arrived — measured at 8x8, its padding and nothing else, growing to the real 111x80 a frame
    /// later. That matters because **Qt works out what is hovered from pointer events, not from geometry**: a list that
    /// grows after it appears cannot tell that the hand is already inside it, and the hand that walked down off the
    /// chip is exactly that hand (traced: the list took itself down under the pointer). The height is what
    /// this buys, and the height is what matters: it is the edge the hand crosses. The width still settles a frame
    /// later (measured 97, then 111) because the rows and their column size each other through bindings rather than
    /// through layout — harmless, because it only ever grows, and growing to the right takes no ground away from a hand
    /// that is already inside.
    function layOutRows() {
        rows.forceLayout()
    }

    contentItem: Column {
        id: rows
        // The rows' half of `pointerInside`; the rows are its children, so their own hover leaves this one standing.
        HoverHandler {
            id: contentHover
        }
        // A Column takes its width from the widest child, and the rows have to reach that width for their highlight to
        // line up — and, now that the chips can stand flush against either edge, for them to line up as well.
        readonly property real rowWidth: {
            let widest = refList.minRowWidth
            for (let i = 0; i < rows.children.length; i++)
                widest = Math.max(widest, rows.children[i].implicitWidth)
            return widest
        }
        Repeater {
            id: rowsRepeater
            model: refList.records
            delegate: Rectangle {
                id: refRow
                required property string modelData
                // The branch the working tree already stands on is not a place to go, but it is not unavailable either
                // — it is where the reader is, and it says so in the colour the sidebar says it in (§ref の種別). Only the
                // hover and the click come off. A tag leads nowhere for its own reason (§タグでは detach しない) and keeps its
                // colour too.
                readonly property bool current:
                    refRow.modelData[0] === "L"
                    && GitFacts.recordName(refRow.modelData) === refList.currentBranch
                // The detached-HEAD marker is the one row that is only a marker: nowhere to go and no ref to read, so
                // it mutes. A branch another working copy has out mutes for the other reason there is — git refuses
                // the move outright (§無効 is for what is actually unavailable, and this one is).
                readonly property bool unavailable:
                    refRow.modelData[0] === "H" || refRow.modelData[5] === "1"
                readonly property bool leadsNowhere:
                    refRow.unavailable || refRow.current || refRow.modelData[0] === "T"

                // The chip is the row: no margin of the row's own on either side, so that what stands between a chip's
                // frame and the card's is the card's padding and nothing else (規約 §グラフ行のダブルクリック — the card lands on
                // the chip column's own edge, and a wider one would reach past the divider into the lanes).
                implicitWidth: rowChip.width + (whose.visible ? whose.width + Theme.spaceSm : 0)
                width: rows.rowWidth
                // The chip sets the height, so a wrapped name makes its own row taller and leaves the others alone.
                height: rowChip.height + 2 * refList.chipInset
                radius: Theme.radiusSm
                color: rowHover.hovered && !refRow.leadsNowhere ? Theme.bgHover : "transparent"

                RefChip {
                    id: rowChip
                    y: refList.chipInset
                    // Flush against the edge the card is anchored by, so every name in the list starts where the chip's
                    // own name started — the head of a name is what it is told apart by, so that is the edge that has
                    // to hold still (規約 §グラフ列は最も広い所のレーンまで).
                    x: 0
                    records: [refRow.modelData]
                    muted: refRow.unavailable
                    // The same wash the row's own chip wears while a second click waits out its window — this card is
                    // that chip, so the mark is on whichever of the two the reader is looking at (規約 §グラフ行のダブルクリック).
                    waiting: refList.rowClicks ? refList.rowClicks.rowRenameArmed(refRow.modelData) : false
                    // Unstacking is only worth it if the names read, so the chip takes everything the row has left once
                    // the reading's remote has its seat — and what will not fit even then is wrapped, not cut: this is
                    // the one place the name is shown in order to be read (規約 §hover のツールチップ).
                    wrapped: true
                    maxWidth: refList.chipRoom - (whose.visible ? whose.width + Theme.spaceSm : 0)
                }
                // Whose reading this is. A tag has no namespace to say it in the way `origin/main` does, and a drifted
                // one puts the same bare name on two rows — this card is where the two meet, so it is where the
                // question gets answered. Meta about the row rather than part of the name, so it is written in the
                // colour the `+N` is (§ref の種別).
                Label {
                    id: whose
                    visible: rowChip.recWhere !== ""
                    // At the far edge of the row, away from the names: put beside its own chip it would sit at a
                    // different distance on every row, and the reading is meta rather than part of the name.
                    x: refRow.width - whose.width
                    // Level with the chip's first line, not with the row: a wrapped name takes the row down with it and
                    // this is meta about the name's first line.
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
                // ref to change. **Not `leadsNowhere`** — the branch the reader is standing on and the tag that is a
                // mark rather than a place both keep their names (the same split the sidebar's rows make).
                readonly property bool nameable: GitFacts.recordKind(refRow.modelData) !== ""
                /// A left click and a double-click on this row, as the row answers them. Named so that a run with no
                /// pointer to press with puts its clicks in at the row itself rather than at a copy of what the row
                /// would have decided (PG_AUTO_ACT=graph-reclick-list / ref-list-pick).
                ///
                /// **Every row takes the click**, whether or not it leads anywhere: what a row that leads nowhere
                /// still has is a name, and the gesture that changes it begins with a click of its own.
                function leftClick(held) {
                    // Answered by the row this card is standing on — a name that leads nowhere is still a name, but
                    // the marker names no ref, so it goes in as a row with nothing on it.
                    if (refList.rowClicks
                            && !refList.rowClicks.noteRowClick(
                                refList.rowOid, refRow.nameable ? refRow.modelData : "", held))
                        return
                    if (refRow.nameable)
                        refList.chose(refRow.modelData)
                }
                function doubleClick() {
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
                TapHandler {
                    id: rowTap
                    /// When the button went down, for the gesture to take off the wait it has left (see the component).
                    property real pressAt: 0
                    onPressedChanged: if (rowTap.pressed) rowTap.pressAt = Date.now()
                    onSingleTapped: refRow.leftClick(Date.now() - rowTap.pressAt)
                    onDoubleTapped: refRow.doubleClick()
                }
                /// A right-click on this row, as the row answers one. Named for the same reason `leftClick` is: a run
                /// with no pointer to press with puts its press in at the row itself rather than at a copy of what the
                /// row would have decided (PG_AUTO_ACT=list-menu).
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
