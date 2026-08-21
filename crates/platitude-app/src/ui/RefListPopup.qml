pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The refs a graph row has, unstacked: every one on a row of its own — branches first, then the tags the chip had no
// room for. The branches are what can be moved to; the tags are read here and nowhere else.
//
// **It opens over the chip, not under it** (規約 §グラフ行のダブルクリック): the owner lands the first row on the chip's own
// seat, so the name the chip was showing is not written out a second time, and the `+N` goes with it — a row carries
// one record, and a chip with one record has no count to draw. What is left reads as the chip's own frame widening to
// the side there was room on, with the rest of the names stacked under it.
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
    /// One was chosen; the whole record, so its kind travels with it.
    signal picked(string record)
    /// One was right-clicked: its menu is asked for, the same one the chip itself answers with. The rows that lead
    /// nowhere still have one — a tag goes nowhere but deletes fine — except the marker, which names no ref at all.
    signal menuAsked(string record)

    /// What a row has to divide between the chip and the reading's remote. **Handed over by the owner, measured from
    /// the side this is opening toward** (規約 §hover のツールチップ「hover で開いたものの幅は、開く位置から測る」) — the pane's own width
    /// is not it: a card that starts partway across the page has only what is left beyond that point, and one sized to
    /// the pane is placed inside the window by `Popup` without ever being narrowed, so it ends up flat against an edge
    /// with its anchor nowhere near the chip.
    ///
    /// Both strings on a row are ref components and can run to the same wall (250 bytes, measured), so a row that just
    /// added them together grew until it left the window: a wall-length tag beside a 90-byte remote already reached
    /// edge to edge, and the remote may be as long as the tag.
    property real chipRoom: Metrics.labelColW
    /// Which edge the chips stand flush against — the one the card is anchored by, so that whichever way it grew, the
    /// first row still sits exactly on the chip it came out of. Right when it opened to the left (the side that does
    /// not cover the graph), left when it had to fall back to the right.
    property bool alignRight: false

    /// How wide the card would be if nothing capped the names, which is what the owner picks a side with — see
    /// `RefChip.wantWidth` on why this cannot be read off `implicitWidth`.
    readonly property real wantWidth: rows.wantWidth + 2 * refList.padding
    /// The card's own size once the rows have been laid out, for an owner placing it against the chip. Read off the
    /// rows rather than off `width` / `height`, which a `Popup` only settles when it is shown.
    readonly property real cardWidth: rows.rowWidth + 2 * refList.padding
    readonly property real cardHeight: rows.implicitHeight + 2 * refList.padding
    /// How far a chip stands off the top of its row — the room a `rowHeight` row leaves around a one-line chip, halved.
    /// A chip that wrapped keeps the same margin above and below and takes the extra height for itself.
    readonly property real chipInset:
        Math.round((Theme.rowHeight - (Theme.fontChipLine + 2 * Theme.borderWidth)) / 2)
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

    /// Lays the rows out now, for an owner that is about to show this in the same turn it handed over the records. A
    /// `Column` positions in the polish that runs after the turn, so without this the list is shown at the size it had
    /// before the records arrived — measured at 8x8, its padding and nothing else, growing to the real 111x80 a frame
    /// later. That matters because **Qt works out what is hovered from pointer events, not from geometry**: a list that
    /// grows after it appears cannot tell that the hand is already inside it, and the hand that walked down off the
    /// chip is exactly that hand (2026-08-09 trace — the list took itself down under the pointer). The height is what
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
        // The same scan over what the rows would have taken uncapped, which is the question "does this side fit".
        readonly property real wantWidth: {
            let widest = 0
            for (let i = 0; i < rows.children.length; i++) {
                const want = rows.children[i].wantWidth
                if (want !== undefined)
                    widest = Math.max(widest, want)
            }
            return widest
        }
        Repeater {
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
                    && refRow.modelData.substring(5).split("\u001E")[0] === refList.currentBranch
                // The detached-HEAD marker is the one row that is only a marker: nowhere to go and no ref to read, so
                // it mutes.
                readonly property bool unavailable: refRow.modelData[0] === "H"
                readonly property bool leadsNowhere:
                    refRow.unavailable || refRow.current || refRow.modelData[0] === "T"

                // The chip is the row: no margin of the row's own on either side, so that what stands between a chip's
                // frame and the card's is the card's padding and nothing else (規約 §グラフ行のダブルクリック — the card lands on
                // the chip column's own edge, and a wider one would reach past the divider into the lanes).
                implicitWidth: rowChip.width + (whose.visible ? whose.width + Theme.spaceSm : 0)
                /// What this row would take with nothing capping the name — the card's half of picking a side.
                readonly property real wantWidth:
                    rowChip.wantWidth + (whose.visible ? whose.width + Theme.spaceSm : 0)
                width: rows.rowWidth
                // The chip sets the height, so a wrapped name makes its own row taller and leaves the others alone.
                height: rowChip.height + 2 * refList.chipInset
                radius: Theme.radiusSm
                color: rowHover.hovered && !refRow.leadsNowhere ? Theme.bgHover : "transparent"

                RefChip {
                    id: rowChip
                    y: refList.chipInset
                    // Flush against the edge the card is anchored by, so every frame in the list ends where the chip's
                    // own frame ended.
                    x: refList.alignRight ? refRow.width - rowChip.width : 0
                    records: [refRow.modelData]
                    muted: refRow.unavailable
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
                    // On the outer side — the edge the card grew toward, which is the one the chips are not flush
                    // against. Put on the chip's own side it would push the frame off the edge the whole list is lined
                    // up on, which is the line that says these rows are the chip (2026-08-21 ユーザー判断: 左揃えで).
                    x: refList.alignRight ? 0 : refRow.width - whose.width
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
                HoverHandler {
                    id: rowHover
                }
                TapHandler {
                    enabled: !refRow.leadsNowhere
                    onTapped: {
                        refList.close()
                        refList.picked(refRow.modelData)
                    }
                }
                TapHandler {
                    acceptedButtons: Qt.RightButton
                    enabled: !refRow.unavailable
                    // The list stays: the menu opens over it, and the owner keeps the list up for as long as the menu
                    // stands (its settle checks the menu).
                    onTapped: refList.menuAsked(refRow.modelData)
                }
            }
        }
    }
}
