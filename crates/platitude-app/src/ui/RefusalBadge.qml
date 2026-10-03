import QtQuick
import platitude.ui

// What a boundary answers a drag it cannot honour with: the platform's cursor left untouched, and this badge in the
// corner the cursor leaves empty — at the hand, not the clamped line (規約 §掴める境界は答える). Every draggable
// boundary draws it from here, so the corner and the line weight are one decision.
NavIcon {
    id: badge

    /// Where the hand is, in this item's own parent's coordinates.
    property point at: Qt.point(0, 0)

    /// Whether to draw. Set and read this, not `visible`: read from another file `visible` comes back stale, while a
    /// plain property of the component's own carries the answer out intact.
    property bool shown: false
    visible: badge.shown

    kind: "no"
    // White, not the platform's red forbidden sign: the boundary is at its limit, nothing is wrong
    // (規約 §掴める境界は答える).
    tint: Theme.textPrimary
    width: Theme.iconMd
    height: Theme.iconMd
    // `spaceXs` off the hand, as the platform's badged cursors sit — further out reads as a separate mark. Flipped to
    // the pointer's other side where that corner is outside the window: half a badge reads as a broken one.
    x: badge.at.x + Theme.spaceXs + badge.width > badge.parent.width
       ? badge.at.x - Theme.spaceXs - badge.width
       : badge.at.x + Theme.spaceXs
    y: badge.at.y + Theme.spaceXs + badge.height > badge.parent.height
       ? badge.at.y - Theme.spaceXs - badge.height
       : badge.at.y + Theme.spaceXs
    z: 4
}
