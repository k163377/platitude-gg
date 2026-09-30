import QtQuick
import platitude.ui

/// The hand over a row's face: the press is the control's until it travels past `drag.threshold`, then a drag over
/// the words, anchored where it landed. A plain `MouseArea` (rules-refs/app-ui.md「手の実装は素の」);
/// `preventStealing` keeps the pane's Flickable from taking the drag part-way.
MouseArea {
    id: hand

    /// The words this row draws.
    required property LineText field
    /// A press that never travelled.
    signal tapped()
    /// Raised as the press lands, before it is decided: the owner clears the last gesture's selection.
    signal taken()

    /// Where the press landed, and whether it has since travelled far enough to be a drag.
    property real fromX: 0
    property real fromY: 0
    property bool dragging: false

    /// The three the handlers call and runs enter, in this hand's coordinates (verify-ui §壊れない動詞の実装).
    function takeAt(x, y) {
        hand.fromX = x
        hand.fromY = y
        hand.dragging = false
        hand.taken()
    }
    function followAt(x, y) {
        if (!hand.dragging) {
            if (Math.abs(x - hand.fromX) < hand.drag.threshold
                    && Math.abs(y - hand.fromY) < hand.drag.threshold)
                return
            hand.dragging = true
            // Anchoring also takes the window's one selection off any other field (`LineText`).
            hand.field.anchorFrom(hand, hand.fromX, hand.fromY)
        }
        hand.field.extendFrom(hand, x, y)
    }
    function releaseNow() {
        const travelled = hand.dragging
        hand.dragging = false
        if (!travelled)
            hand.tapped()
    }

    anchors.fill: parent
    preventStealing: true
    cursorShape: Qt.PointingHandCursor
    onPressed: mouse => hand.takeAt(mouse.x, mouse.y)
    onPositionChanged: mouse => { if (hand.pressed) hand.followAt(mouse.x, mouse.y) }
    onReleased: hand.releaseNow()
    onCanceled: hand.dragging = false
}
