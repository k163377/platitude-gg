pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The run a tab draws after the repository's name while it stands in a linked worktree: the WORKTREES mark and
// the worktree's folder name (デザイン規約 §タブの所作). Drawn by both the tab and its stand-in.
//
// The mark is what parts the two names — without it the run reads as the repository's name carrying on — so the run
// is dropped whole rather than drawn without it (`floorWidth`, `TabShare.splitName`).
//
// Given a width, not asking for one: it gets the cap's leftover after the name, and cuts its name to it (`CutName`).
Item {
    id: treeRun

    /// The worktree's folder name (`TabItem.worktree_name`); empty for the repository's own worktree, and then nothing
    /// is drawn.
    property string name: ""
    /// The strip's shared arithmetic (`TabMetrics`): `treeSeat` and `treeRunGap`.
    required property var metrics
    /// The shortest the name inside is worth cutting to (`TabStrip.tabTitleMinW`). Measured at the tab name's size, so
    /// it holds a letter or two more here — harmless, since this run is given up first anyway.
    property real minNameW: 0

    /// This run at its own width: what a tab asks the strip for on top of its name, and the most it is ever drawn at.
    readonly property real naturalWidth:
        treeRun.name === "" ? 0 : treeRun.fixedWidth + Math.ceil(nameCut.implicitWidth)
    /// Everything but the name: the gap before it and the mark's seat.
    readonly property real fixedWidth: treeRun.metrics.treeRunGap + treeRun.metrics.treeSeat
    /// The least this is drawn at: the fixed parts plus a readable name, or the whole run if shorter — a floor above
    /// the natural width would drop a run that fits.
    readonly property real floorWidth: Math.min(treeRun.naturalWidth, treeRun.fixedWidth + treeRun.minNameW)

    implicitWidth: treeRun.naturalWidth
    implicitHeight: nameCut.implicitHeight

    // Set as a letter of the run (`TabMetrics.treeSeat`).
    NavIcon {
        id: treeIcon
        kind: "tree"
        anchors.left: treeRun.left
        anchors.leftMargin: treeRun.metrics.treeRunGap
        anchors.verticalCenter: treeRun.verticalCenter
        width: treeRun.metrics.treeSeat
        height: treeRun.metrics.treeSeat
        // The grid scales with the seat and the line does not, so unscaled the mark is heavier than the letters beside
        // it (`NavIcon.stroke`).
        stroke: Metrics.iconStroke * treeRun.metrics.treeSeat / Theme.iconMd
        tint: Theme.textSecondary
    }
    CutName {
        id: nameCut
        anchors.left: treeIcon.right
        anchors.right: treeRun.right
        anchors.verticalCenter: treeRun.verticalCenter
        text: treeRun.name
        color: Theme.textSecondary
        // A step under the tab's name (デザイン規約 §タイポグラフィ「読むだけの添え物」).
        pixelSize: Theme.fontSm
    }
}
