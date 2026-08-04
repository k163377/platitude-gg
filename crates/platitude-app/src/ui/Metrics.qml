// Graph / interaction constants — a verbatim mirror of the
// グラフ・インタラクション定数 table in internal-docs/デザイン規約.md.
// Edit the document first, then reflect changes here; never introduce
// values that are not in the document's table (same rule as Theme).
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
    readonly property int detailsAvatar: 40
    readonly property int anchorDelayMs: 50
    readonly property int chipExpandMs: 400
    readonly property int pollIntervalMs: 10000
    readonly property int spinMs: 1000
    readonly property int holdMs: 800
    readonly property var laneDash: [1, 1]
}
