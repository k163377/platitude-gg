pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The run a tab draws after the repository's name while it stands in a linked worktree: the WORKTREES mark and
// the worktree's folder name (デザイン規約 §タブの所作). Drawn wherever a name is followed by the worktree it stands
// in — the tab and its stand-in, the operation panel's name line (`OpsPicker.trail`), a menu row
// (`AppMenuItem.trail`) — so a fix to the run reaches them all.
//
// The mark is what parts the two names — without it the run reads as the repository's name carrying on — so the run
// is dropped whole rather than drawn without it (`floorWidth`, `TabShare.splitName`).
//
// Given a width, not asking for one: a tab hands it the cap's leftover after the name, and it cuts its name to it
// (`CutName`).
Item {
    id: treeRun

    /// The worktree's folder name (`TabItem.worktree_name`); empty for the repository's own worktree, and then nothing
    /// is drawn.
    property string name: ""
    /// The step before the mark, from the words the run follows: the strip's `TabMetrics.treeRunGap`, or the holder's
    /// own step between two runs.
    required property real gap
    /// The shortest the name inside is worth cutting to (`TabStrip.tabTitleMinW`). Measured at the tab name's size, so
    /// it holds a letter or two more here — harmless, since this run is given up first anyway.
    property real minNameW: 0

    /// This run at its own width: what a tab asks the strip for on top of its name, and the most it is ever drawn at.
    readonly property real naturalWidth:
        treeRun.name === "" ? 0 : treeRun.fixedWidth + Math.ceil(nameCut.implicitWidth)
    /// Everything but the name: the gap before it and the mark's seat.
    readonly property real fixedWidth: treeRun.gap + treeIcon.wordCell
    /// The least this is drawn at: the fixed parts plus a readable name, or the whole run if shorter — a floor above
    /// the natural width would drop a run that fits.
    readonly property real floorWidth: Math.min(treeRun.naturalWidth, treeRun.fixedWidth + treeRun.minNameW)

    implicitWidth: treeRun.naturalWidth
    implicitHeight: nameCut.implicitHeight

    // A sign on the name (デザイン規約 §アイコンの席 の修飾): its box is the name's line, so the two stand on one
    // baseline, and its cell the face's own room round it.
    NavIcon {
        id: treeIcon
        kind: "tree"
        set: "sign"
        size: nameCut.pixelSize
        wordBase: nameCut.baselineOffset
        anchors.left: treeRun.left
        anchors.leftMargin: treeRun.gap
        anchors.verticalCenter: treeRun.verticalCenter
        width: treeIcon.wordCell
        height: nameCut.height
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
