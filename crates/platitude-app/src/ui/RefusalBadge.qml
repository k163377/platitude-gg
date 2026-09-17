import QtQuick
import platitude.ui

// What a boundary answers a drag it cannot honour with: the platform's own cursor left untouched, and this badge tucked
// into the corner the cursor leaves empty (規約 §掴める境界は答える). Every draggable boundary in the window draws it from here,
// so the corner it sits in and the weight of its line are one decision.
//
// It answers the hand. Past a clamp the two are no longer in the same place — the line stops and the
// pointer goes on — and the badge belongs with the hand that is still asking.
NavIcon {
    id: badge

    /// Where the hand is, in this item's own parent's coordinates.
    property point at: Qt.point(0, 0)

    /// Whether to draw. Set this, and read it back through this: read from another file `visible` comes back stale —
    /// measured: a run where the badge's own binding was already true reported false through both a binding and an
    /// alias over it in the same breath. A plain property of this component's own carries the answer out intact, and
    /// `visible` follows it here where nothing has to read it.
    property bool shown: false
    visible: badge.shown

    kind: "no"
    // An answer in the white of ordinary text (規約): the boundary has nothing left to give that way. The platform's
    // red forbidden cursor would make it a problem.
    tint: Theme.textPrimary
    width: Theme.iconMd
    height: Theme.iconMd
    // Into the corner the cursor leaves empty — the platform's own badged cursors sit that close, and one held
    // further out reads as a separate mark.
    //
    // Unless there is no room: a pane flush with the window's edge (the details box is) leaves that corner outside, and
    // half a badge reads as a broken one. There it goes to the other side of the pointer, which is the same distance
    // from the hand.
    x: badge.at.x + Theme.spaceXs + badge.width > badge.parent.width
       ? badge.at.x - Theme.spaceXs - badge.width
       : badge.at.x + Theme.spaceXs
    y: badge.at.y + Theme.spaceXs + badge.height > badge.parent.height
       ? badge.at.y - Theme.spaceXs - badge.height
       : badge.at.y + Theme.spaceXs
    z: 4
    // A Canvas that was never visible was never asked to paint, and the first thing this one does is appear
    // (app-ui.md).
    onVisibleChanged: if (visible) requestPaint()
}
