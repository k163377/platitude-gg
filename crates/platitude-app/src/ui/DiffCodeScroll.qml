pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// How far sideways the diff's code has been sent, and the hands that send it: the bar along the bottom edge of the rows
// and the middle-click autoscroll, which carries the rows up and down at the same time (デザイン規約 §diff を横へ送る).
//
// **Declared beside the pane's body, not inside the list.** Two measured reasons (measured with qmltestrunner):
//
//  - a `HoverHandler` on an overlay laid over the rows takes their hover away entirely — the `+` a line puts out under
//    the pointer never appears. A plain `MouseArea` does not, so the hand may stay over the rows while the question "is
//    the pointer in this pane" is answered by a handler on the pane itself (an **ancestor** handler leaves the rows
//    their own hover).
//  - the bar is a child of the pane rather than of the `Flickable`, so a drag along it is never taken away by the
//    list's own filtering.
Item {
    id: codeScroll

    /// The list this sits over: its frame is where the hand and the bar go, and its `contentY` is what the hand carries
    /// up and down.
    required property var view
    /// How wide the code is and how much of it fits — the pane owns both, since it owns the gutter that eats the
    /// difference.
    required property real codeWidth
    required property real roomWidth
    /// Whether the pointer is anywhere in the pane (the pane's own handler). The bar lies over the last row, so it is
    /// only out while there is a hand here to use it (デザイン規約 §diff を横へ送る), and it is the same answer the bar
    /// dims by — attached to no flickable, it has no view of its own to ask (`AutoScrollBar.inArea`).
    required property bool paneHovered
    /// The file on screen. A different one starts at its own left edge; the same one read again — which is what every
    /// partial write ends with — keeps the place it was being read from, the way the vertical place is kept
    /// (`DiffScrollPlace`).
    required property string file

    /// How far the code has been sent, and how far it can go.
    ///
    /// **The rows do not get a say in this.** The reach is what the pane measured off the lines the model picked
    /// (`DiffTextMetrics.codeW`), and a floor taken from the rows as they are laid out — which would have covered a
    /// line the pick missed — reads a width at a moment that is nobody's: the colours arrive a second after the rows
    /// (`DiffModel.repaint_rows`), and in the turn the markup lands a row is briefly measured with its `<font …>`
    /// tags counted as text. Holding the largest width ever seen latches that, and the bar then reaches the tags' own
    /// width past the end of every coloured line (measured: 23,000px against the 15,710 the line is drawn at). A row's
    /// own width is trustworthy only while the row is settled, and nothing here knows when that is.
    property real offset: 0
    readonly property real maxOffset: Math.max(0, codeScroll.codeWidth - codeScroll.roomWidth)
    readonly property bool canPan: codeScroll.maxOffset > 0

    /// The strip the bar stands on, below the last row rather than over it — the pane keeps its rows this much clear
    /// of its own bottom edge (`DiffPane`). A line of a file is read while it is being sent sideways, so the bar
    /// cannot be allowed to lie on one; a file with nowhere sideways to go has no bar and gives the strip back
    /// (デザイン規約 §diff を横へ送る). Read off the bar itself: its own size is the only thing that says how tall it is.
    readonly property real barRoom: codeScroll.canPan ? bar.height : 0

    /// Sends the code `dx` further, as far as it goes. The bar, the wheel and the hand all arrive here, so one clamp
    /// holds for all of them.
    function shift(dx) {
        codeScroll.offset = Math.max(0, Math.min(codeScroll.offset + dx, codeScroll.maxOffset))
    }
    onFileChanged: codeScroll.offset = 0
    // A window that grew, or a shorter file: the place being read can end up past the end of what there is to read.
    onMaxOffsetChanged: codeScroll.shift(0)

    /// Automation: the hand, started and drifted without a pointer — and what it and the bar are doing, for the run to
    /// read back (a middle button cannot be injected any more than a hover can — verify-ui).
    function startHand(x, y) { hand.start(x, y) }
    function driftHand(x, y) { hand.drift(x, y) }
    readonly property alias handScrolling: hand.scrolling
    readonly property alias barShown: bar.visible

    // The rows' own frame, taken off the list. The pane's body is a column filling this same parent, so the list's own
    // x/y are already in these coordinates.
    Item {
        id: frame
        x: codeScroll.view.x
        y: codeScroll.view.y
        width: codeScroll.view.width
        height: codeScroll.view.height

        // The hand: middle-click autoscroll, both ways at once. Sideways is offered everywhere in this pane — unlike
        // the graph, where only the lanes column goes sideways, a diff is one column of text throughout.
        MiddleAutoScroll {
            id: hand
            anchors.fill: parent
            panFrom: 0
            panTo: frame.width
            canPan: codeScroll.canPan
            onDrifted: (dy, dx) => {
                codeScroll.view.contentY = codeScroll.view.clampY(codeScroll.view.contentY + dy)
                if (dx !== 0)
                    codeScroll.shift(dx)
            }
        }
        // Fusion draws a bar's handle only in the style's "active" state, which for one not attached to a Flickable
        // means while the pointer is on the bar itself — a strip on the bottom edge nobody would find. When this bar is
        // out it is because this pane put it there, so the style stops deciding (GraphPane's lane bar, same reason).
        //
        // **Out while the hand is in the pane, and bright once the code has been sent** — the same two steps of ink
        // every bar in the window carries (`AutoScrollBar`), over a visibility this bar keeps for itself
        // (デザイン規約 §diff を横へ送る).
        //
        // **It stands below the rows, not on them.** A line of the file is being read at the very moment it is sent
        // sideways, so the pane holds its rows `barRoom` clear of its own bottom edge and the bar takes that strip
        // — the one place in this window where the room is taken by the pane and not by a gutter in the rows.
        //
        // The thumb itself is the style's see-through one, as every bar that takes no gutter is (デザイン規約 §色
        // スクロールバー): the ink comes from the window's palette, so the bar wears it without asking.
        AutoScrollBar {
            id: bar
            inArea: codeScroll.paneHovered
            visible: codeScroll.canPan && (codeScroll.paneHovered || pressed || hand.scrolling)
            policy: ScrollBar.AlwaysOn
            orientation: Qt.Horizontal
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.bottom
            size: codeScroll.codeWidth > 0 ? codeScroll.roomWidth / codeScroll.codeWidth : 1
            position: codeScroll.codeWidth > 0 ? codeScroll.offset / codeScroll.codeWidth : 0
            onPositionChanged: {
                // No flickable to be wired to, so the bar's own position is what says the code has just been sent —
                // by the thumb, the wheel, or the hand drifting sideways.
                bar.moved()
                if (pressed)
                    codeScroll.offset = Math.max(0, Math.min(position * codeScroll.codeWidth, codeScroll.maxOffset))
            }
        }
    }
}
