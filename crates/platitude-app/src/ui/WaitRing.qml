import QtQuick
import platitude.ui

// The ring beside the pointer while a write that replays history is out; the OS cursor stays as it is, since a
// cursor is not in a screenshot (デザイン規約 §進行中・長押しの定数). This file owns only where the mark sits —
// whoever knows what is running says whether it turns (`SpinnerIcon.spinning`).
SpinnerIcon {
    id: waitRing

    /// Where the hand is, in this item's own parent's coordinates.
    property point at: Qt.point(0, 0)

    width: Theme.iconMd
    height: Theme.iconMd
    // `RefusalBadge`'s corner and offset (the two pointer marks share one decision), flipped to the pointer's other
    // side where the window's edge would cut the ring (デザイン規約 §掴める境界は答える).
    x: waitRing.at.x + Theme.spaceXs + waitRing.width > waitRing.parent.width
       ? waitRing.at.x - Theme.spaceXs - waitRing.width
       : waitRing.at.x + Theme.spaceXs
    y: waitRing.at.y + Theme.spaceXs + waitRing.height > waitRing.parent.height
       ? waitRing.at.y - Theme.spaceXs - waitRing.height
       : waitRing.at.y + Theme.spaceXs
}
