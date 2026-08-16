pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The refs a graph row could only stack: one chip and a "+N". Hovering
// that chip opens this under it, with every ref on a row of its own —
// branches first, then the tags the chip had no room for. The branches
// are what can be moved to; the tags are read here and nowhere else.
//
// Owned by the page, not by the delegate that raised it — delegates are
// recycled out from under an open popup, which is why the context menus
// live there too. It is a popup rather than an item in the row for the
// same reason a menu is: anything declared inside the list is clipped by
// it and painted under the row below.
AppCard {
    id: refList

    /// Chip records (kind + flags + name, see encode.rs), as shown.
    property var records: []
    /// The branch the working tree is on; that one leads nowhere.
    property string currentBranch: ""
    /// One was chosen; the whole record, so its kind travels with it.
    signal picked(string record)
    /// One was right-clicked: its menu is asked for, the same one the
    /// chip itself answers with. The rows that lead nowhere still have
    /// one — a tag goes nowhere but deletes fine — except the marker,
    /// which names no ref at all.
    signal menuAsked(string record)

    /// What a row has to divide between the chip and the reading's remote
    /// — the window this opens over, less the padding and the gaps. Both
    /// of those strings are ref components and can run to the same wall
    /// (250 bytes, measured), so a row that just added them together grew
    /// until it left the window: a wall-length tag beside a 90-byte
    /// remote already reached edge to edge, and the remote may be as long
    /// as the tag.
    readonly property real room:
        (refList.parent ? refList.parent.width : 400)
        - 2 * Theme.spaceXs - 3 * Theme.spaceSm

    padding: Theme.spaceXs
    // Nothing stands between the chip and this: the pointer has to be
    // able to walk down into it without leaving both.
    margins: 0
    // The hand walks down off the chip into this and picks a row, so both
    // halves of `AppCard.pointerInside` are wanted — this is the list the
    // pair was measured on.
    tracksPointer: true
    contentPointed: contentHover.hovered

    /// Lays the rows out now, for an owner that is about to show this in
    /// the same turn it handed over the records. A `Column` positions in
    /// the polish that runs after the turn, so without this the list is
    /// shown at the size it had before the records arrived — measured at
    /// 8x8, its padding and nothing else, growing to the real 111x80 a
    /// frame later. That matters because **Qt works out what is hovered
    /// from pointer events, not from geometry**: a list that grows after
    /// it appears cannot tell that the hand is already inside it, and
    /// the hand that walked down off the chip is exactly that hand
    /// (2026-08-09 trace — the list took itself down under the pointer).
    /// The height is what this buys, and the height is what matters: it
    /// is the edge the hand crosses. The width still settles a frame
    /// later (measured 97, then 111) because the rows and their column
    /// size each other through bindings rather than through layout —
    /// harmless, because it only ever grows, and growing to the right
    /// takes no ground away from a hand that is already inside.
    function layOutRows() {
        rows.forceLayout()
    }

    contentItem: Column {
        id: rows
        // The rows' half of `pointerInside`; the rows are its children,
        // so their own hover leaves this one standing.
        HoverHandler {
            id: contentHover
        }
        // A Column takes its width from the widest child, and the rows
        // have to reach that width for their highlight to line up.
        readonly property real rowWidth: {
            let widest = 0
            for (let i = 0; i < rows.children.length; i++)
                widest = Math.max(widest, rows.children[i].implicitWidth)
            return widest
        }
        Repeater {
            model: refList.records
            delegate: Rectangle {
                id: refRow
                required property string modelData
                // The branch the working tree already stands on is not a
                // place to go, but it is not unavailable either — it is
                // where the reader is, and it says so in the colour the
                // sidebar says it in (§ref の種別). Only the hover and the
                // click come off. A tag leads nowhere for its own reason
                // (§タグでは detach しない) and keeps its colour too.
                readonly property bool current:
                    refRow.modelData[0] === "L"
                    && refRow.modelData.substring(5).split("\u001E")[0] === refList.currentBranch
                // The detached-HEAD marker is the one row that is only a
                // marker: nowhere to go and no ref to read, so it mutes.
                readonly property bool unavailable: refRow.modelData[0] === "H"
                readonly property bool leadsNowhere:
                    refRow.unavailable || refRow.current
                    || refRow.modelData[0] === "T"

                implicitWidth: rowChip.width + 2 * Theme.spaceSm
                               + (whose.visible
                                  ? whose.width + Theme.spaceSm : 0)
                width: rows.rowWidth
                height: Theme.rowHeight
                radius: Theme.radiusSm
                color: rowHover.hovered && !refRow.leadsNowhere
                       ? Theme.bgHover : "transparent"

                RefChip {
                    id: rowChip
                    anchors.verticalCenter: parent.verticalCenter
                    x: Theme.spaceSm
                    records: [refRow.modelData]
                    muted: refRow.unavailable
                    // Unstacking is only worth it if the names read, so
                    // the chip takes everything the row has left once
                    // the reading's remote has its seat.
                    maxWidth: refList.room
                              - (whose.visible ? whose.width + Theme.spaceSm : 0)
                }
                // Whose reading this is. A tag has no namespace to say it
                // in the way `origin/main` does, and a drifted one puts the
                // same bare name on two rows — this card is where the two
                // meet, so it is where the question gets answered. Meta
                // about the row rather than part of the name, so it is
                // written in the colour the `+N` is (§ref の種別).
                Label {
                    id: whose
                    visible: rowChip.recWhere !== ""
                    anchors.left: rowChip.right
                    anchors.leftMargin: Theme.spaceSm
                    anchors.verticalCenter: parent.verticalCenter
                    text: rowChip.recWhere
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontSm
                    // A third of the row at most: this is meta about the
                    // reading, and the name it qualifies comes first.
                    width: Math.min(implicitWidth, refList.room / 3)
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
                    // The list stays: the menu opens over it, and the
                    // owner keeps the list up for as long as the menu
                    // stands (its settle checks the menu).
                    onTapped: refList.menuAsked(refRow.modelData)
                }
            }
        }
    }
}
