// Graph / interaction constants — a verbatim mirror of the
// グラフ・インタラクション定数 and 進行中・長押しの定数 tables in
// internal-docs/デザイン規約.md. Edit the document first, then reflect
// changes here; never introduce values that are not in those tables
// (same rule as Theme). `stateFlushMs` is the one value that lives here
// alone: how often state is offered to disk is persistence timing, not
// something drawn, so the design document has no seat for it.
pragma Singleton

import QtQuick
import platitude.ui

QtObject {
    readonly property int laneW: Theme.iconLg
    readonly property int laneInset: Theme.spaceXs
    readonly property int nodeIcon: Theme.iconLg
    readonly property int laneStroke: 2
    readonly property real iconStroke: 1.5
    readonly property real identiconFill: 0.72
    readonly property int wheelRows: 6
    readonly property real middleScrollGain: 0.12
    readonly property int labelColW: 152
    readonly property int graphDefaultLanes: 12
    readonly property int messageMinW: 160
    readonly property int detailsAvatar: 40
    readonly property int anchorDelayMs: 50
    readonly property int pollIntervalMs: 10000
    readonly property int spinMs: 1000
    readonly property int holdMs: 500
    readonly property int holdFillMin: Theme.spaceXs
    readonly property int holdBackMs: 150
    readonly property int opticalDrop: 1
    readonly property int menuMinW: 160
    readonly property var laneDash: [1, 1]
    // The one opacity anything is dimmed to (デザイン規約 §暗く落とした段).
    // Same 45% the `*Dim` colours already are — measured, not chosen.
    readonly property real dimFade: 0.45
    readonly property int tipDelayMs: 600
    readonly property int hoverKeepMs: 150
    // How often the window offers its shape to be written down. Nothing
    // is written unless something moved, so this only has to be short
    // enough that a crash loses a layout nobody would miss.
    readonly property int stateFlushMs: 2000
}
