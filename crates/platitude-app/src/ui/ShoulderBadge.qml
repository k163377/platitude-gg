import QtQuick
import platitude.ui

// A mark on its parent mark's upper-right shoulder, worked out from the parent so no design of one needs a size of its
// own (デザイン規約 §寸法 の右肩の印): half the parent, never under `iconBadgeMin`; its centre a `spaceXs` in from the
// parent's right edge and its top a `spaceXs` over the parent's top — the folded rail's TAGS eye, which is the rule
// at 24 (`NavRail`). No ground under it: the parent's mark keeps its own ink, the badge sitting mostly in the air of
// its box.
NavIcon {
    width: Math.max(Theme.iconBadgeMin, parent.width / 2)
    height: width
    anchors.horizontalCenter: parent.right
    anchors.horizontalCenterOffset: -Theme.spaceXs
    anchors.top: parent.top
    anchors.topMargin: -Theme.spaceXs
}
