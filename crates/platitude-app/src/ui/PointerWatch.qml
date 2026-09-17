import QtQuick

/// Where the hand is over the window's own content, for whoever has to open something beside it
/// (`SharedToolTip`). One is enough: a window has one pointer.
///
/// **Declared straight on the item that parents the content.** A `HoverHandler` takes
/// hover across the whole of the item it is put on and keeps it from everything stacked *under* that item — but its own
/// subtree is unaffected, so a handler on the parent leaves every row, cell and control below it answering the pointer
/// exactly as before (measured, qmltestrunner: with this handler on the root, a cell's own `HoverHandler` and a
/// `ToolButton`'s `hovered` both still stood, and the handler reported the pointer over each of them). The other way
/// round — a transparent sheet on top — is the shape that puts the rows out (規約 §QML 実装ルール), and a `PointHandler`
/// laid over everything is no use here: it never becomes active without a press, so a hovering pointer moves under it
/// unseen (same measurement).
///
/// The window's popups stand above this item and take the pointer with them, so `known` falls while the hand is inside
/// one. That is the honest answer: what opens beside the hand has to know when it cannot see it.
HoverHandler {
    id: watch

    /// Whether the position below means anything right now.
    readonly property bool known: watch.hovered
    /// Whether a hand has ever been over this window. Latched: it is the difference between "the pointer is
    /// somewhere else" and "there is no pointer" — an offscreen run has none at all, and what waits for a hand to walk
    /// somewhere has nothing to wait for there (`SharedToolTip`).
    property bool seen: false
    onHoveredChanged: if (watch.hovered) watch.seen = true
    /// The hand, in the coordinates of the item this is declared in.
    readonly property real handX: watch.point.position.x
    readonly property real handY: watch.point.position.y

    /// Whether the hand is on `item` — asked of things that cannot say so themselves (a row is not a `Control` and has
    /// no `hovered` of its own).
    function over(item) {
        if (!watch.hovered || item === null || item === undefined)
            return false
        const p = item.mapFromItem(watch.parent, watch.point.position.x, watch.point.position.y)
        return p.x >= 0 && p.y >= 0 && p.x < item.width && p.y < item.height
    }
}
