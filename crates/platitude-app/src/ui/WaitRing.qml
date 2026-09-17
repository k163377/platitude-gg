import QtQuick
import platitude.ui

// What the hand is given while a write that replays history is out: the window's one turning ring, tucked into the
// corner the pointer leaves empty. This file owns only where the mark sits — whoever knows what is running says
// whether it turns (`SpinnerIcon.spinning`).
//
// **The platform's own cursor is left untouched** — the decision `RefusalBadge` was made under (デザイン規約 §掴める境界は
// 答える), and it holds here for the same two reasons and one more. An arrow drawn by this app matches neither the OS's
// line weight nor the way it is edged, so a swapped cursor reads as a different tool; and a cursor is not in a
// screenshot at all (verify-ui スキル), so the one state a headless run would be blind to would be exactly this one.
// A mark the app drew photographs.
SpinnerIcon {
    id: waitRing

    /// Where the hand is, in this item's own parent's coordinates.
    property point at: Qt.point(0, 0)

    width: Theme.iconMd
    height: Theme.iconMd
    // The corner and the offset are `RefusalBadge`'s single decision, followed: the two marks a
    // pointer can be wearing have to sit in the same place, or the second reads as something else standing nearby.
    // And out of that corner where there is no room for it, for the reason that badge gives — a window's edge leaves
    // the corner outside, and half a ring reads as a broken one.
    x: waitRing.at.x + Theme.spaceXs + waitRing.width > waitRing.parent.width
       ? waitRing.at.x - Theme.spaceXs - waitRing.width
       : waitRing.at.x + Theme.spaceXs
    y: waitRing.at.y + Theme.spaceXs + waitRing.height > waitRing.parent.height
       ? waitRing.at.y - Theme.spaceXs - waitRing.height
       : waitRing.at.y + Theme.spaceXs
}
