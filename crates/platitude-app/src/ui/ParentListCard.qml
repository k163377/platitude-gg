pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// A commit's parents one to a row, opened by the parent line's count. The line meanwhile stands as a single parent's
// would (`HashPlate.unrolled`): the arrow, then the seat of one hash, and this card on that seat — its first row where
// that hash would be, the rest stacked under it, the arrow left outside (デザイン規約 §右のペインの親). One hash wide:
// every hash whole, and no more air than a row's. A popup, owned by the pane (`DetailsAuthorCards`): declared in the
// block it would be clipped by it.
AppCard {
    id: parentCard

    /// Every parent, full hex in git's order. Set as the card opens, like the other cards' records.
    property var parents: []
    /// How tall the card may stand, from where it opens to the pane's floor; past it the rows scroll.
    property real listRoom: 0

    /// A row was pressed and let go where it landed: the reader goes to that parent.
    signal picked(string oidHex)

    /// Where a row's hash starts: the row's air (`HashPlate.unrollInset` holds the seat for it and the frame).
    readonly property real hashX: Theme.spaceXs
    readonly property TextMetrics digitInk: TextMetrics {
        font.family: Theme.monoFamily
        font.pixelSize: Theme.fontSm
        text: "0"
    }
    /// A whole short hash, as the line draws one (`HashPlate.wholeDigits`; `tst_parentcard` holds the two together).
    readonly property int wholeDigits: 8
    readonly property real hashWidth: Math.ceil(parentCard.wholeDigits * parentCard.digitInk.advanceWidth)
    /// A row is the parent line's own line box: the first lands on the line and the card unrolls it downward. A list's
    /// `rowHeight` would stand half its extra above the line, over the commit's own hash.
    readonly property int rowStep: Theme.fontSmLine

    /// The card's size, for an owner placing it: a `Popup` settles `width` / `height` only once it is shown.
    readonly property real cardWidth: 2 * parentCard.hashInset + parentCard.hashWidth
    readonly property real cardHeight: Math.min(
        parentCard.parents.length * parentCard.rowStep + 2 * parentCard.padding,
        Math.max(parentCard.listRoom, parentCard.rowStep + 2 * parentCard.padding))
    /// From the card's edge to its hashes, and from its top to the middle of its first row: what the owner backs the
    /// card out by so the first row lands on the line's seat.
    readonly property real hashInset: parentCard.padding + parentCard.hashX
    readonly property real firstMiddle: parentCard.padding + parentCard.rowStep / 2

    /// The row drawing the `i`th parent, scrolled on first: a recycling list has no item for a row not shown.
    function rowAt(i) {
        if (i < 0 || i >= parentCard.parents.length)
            return null
        rowsList.positionViewAtIndex(i, ListView.Contain)
        rowsList.forceLayout()
        return rowsList.itemAtIndex(i)
    }
    /// Automation: the tap a hand makes on a row, in at the hand's own functions (verify-ui §壊れない動詞の実装).
    function tapRow(i) {
        const row = parentCard.rowAt(i)
        if (!row)
            return false
        row.hand.takeAt(row.width / 2, row.height / 2)
        row.hand.releaseNow()
        return true
    }
    function pick(index) {
        const hex = parentCard.parents[index]
        parentCard.close()
        parentCard.picked(hex)
    }

    width: parentCard.cardWidth
    height: parentCard.cardHeight
    // The chip card's ground: a frame's width of padding so the rows' wash meets it, and its corner.
    padding: Theme.borderWidth
    faceRadius: Theme.radiusSm
    // Opened over its own count: the hand is inside from the first frame.
    margins: 0
    // The hand reads the rows, so both halves (rules-refs/app-ui.md「hover で開くものの 5 つの罠」の (2)).
    tracksPointer: true
    contentPointed: contentHover.hovered

    contentItem: Item {
        HoverHandler {
            id: contentHover
        }
        AppListView {
            id: rowsList
            anchors.fill: parent
            model: parentCard.parents
            // The idle step counted from this card's ground (規約 §ペインのスクロールバー), as the chip card's.
            verticalBar: PaneScrollBar {
                idleColor: Theme.borderSubtle
            }
            delegate: Rectangle {
                id: parentRow

                required property string modelData
                required property int index
                readonly property bool lit: rowHand.containsMouse
                readonly property alias field: hashText
                readonly property alias hand: rowHand

                width: rowsList.width
                height: parentCard.rowStep
                radius: Theme.radiusSm
                color: parentRow.lit ? Theme.bgHover : "transparent"

                LineText {
                    id: hashText
                    x: parentCard.hashX
                    anchors.verticalCenter: parent.verticalCenter
                    text: parentRow.modelData.substring(0, parentCard.wholeDigits)
                    mono: true
                    pixelSize: Theme.fontSm
                    color: Theme.textLink
                    grabbable: false
                }
                // The link's underline, under the hash as on the line.
                Rectangle {
                    visible: parentRow.lit
                    color: Theme.textLink
                    height: Theme.borderWidth
                    x: hashText.x
                    width: hashText.width
                    y: hashText.y + hashText.height - height
                }
                // The whole row is the target, as a chip card's row is; last, so it lies over the words.
                RowHand {
                    id: rowHand
                    field: hashText
                    hoverEnabled: true
                    onTaken: hashText.deselect()
                    onTapped: parentCard.pick(parentRow.index)
                }
            }
        }
    }
}
