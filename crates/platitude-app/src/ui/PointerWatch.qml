import QtQuick

/// Where the hand is over the window's content, for whoever opens something beside it (`SharedToolTip`). One per
/// window.
///
/// **Declared on the item that parents the content**: a `HoverHandler` leaves its own subtree answering the pointer
/// (rules-refs/app-ui.md「`PointerWatch` は窓の内容を抱える親に直付けする」). `known` falls while a popup covers the hand.
HoverHandler {
    id: watch

    /// Whether the position below means anything right now.
    readonly property bool known: watch.hovered
    /// Whether a hand has ever been over this window. Latched: it tells "the pointer is elsewhere" from "there is no
    /// pointer" (an offscreen run), where waiting for a hand to walk has nothing to wait for (`SharedToolTip`).
    property bool seen: false
    onHoveredChanged: if (watch.hovered) watch.seen = true
    /// The hand, in the coordinates of the item this is declared in.
    readonly property real handX: watch.point.position.x
    readonly property real handY: watch.point.position.y

    /// Whether the hand is on `item` — for things with no `hovered` of their own (a row is not a `Control`).
    function over(item) {
        if (!watch.hovered || item === null || item === undefined)
            return false
        const p = item.mapFromItem(watch.parent, watch.point.position.x, watch.point.position.y)
        return p.x >= 0 && p.y >= 0 && p.x < item.width && p.y < item.height
    }
}
