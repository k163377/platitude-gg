pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// What a tab says after the repository's name while it is standing in a linked working copy: that copy's own folder
// name, behind the mark the WORKTREES section is read by (デザイン規約 §タブの所作). The name stays the repository's
// — this is the second run beside it, and it is set a step down and a shade quieter because the tab is read for the
// first (§タイポグラフィ「読むだけの添え物」).
//
// **The mark is the whole of what parts the two names.** Both are names, set one after the other in one line, and
// with nothing between them the run reads as the repository's own name carrying on — so the mark is drawn before a
// letter of the name is, and the run is dropped whole rather than drawn without it (`floorWidth`, `TabShare`).
//
// **The tab and its stand-in draw this same part**, so a strip that has scrolled cannot say two different things
// about where the reader is standing (`TabItemDelegate` / `TabPin`).
//
// **Given a width, not asking for one.** The strip hands every tab one cap and the tab spends it on the name first
// (`TabShare.splitName`), so what arrives here is the leftover — and the name inside is cut to it, the middle way a
// name is cut anywhere in this band (`CutName`).
Item {
    id: treeRun

    /// The copy's own folder name (`TabItem.copy_name`). Empty for a tab standing in the repository's own copy,
    /// which is the whole of what says this run is not drawn.
    property string name: ""
    /// The strip's shared arithmetic (`TabMetrics`): the mark's seat, and the one step this run does spend — the
    /// one off the name it stands after, which is the same step as the gaps in the tab around it.
    required property var metrics
    /// The shortest the name inside is worth cutting to, pushed in by whoever draws this
    /// (`TabStrip.tabTitleMinW` — the floor measured off the font, in letters). **Measured in the strip's own step**,
    /// which this run is drawn a step under: the floor comes out worth a letter or two more here, and the run is the
    /// thing the strip gives up first either way.
    property real minNameW: 0

    /// This run at its own width: what a tab asks the strip for on top of its name, and the most it is ever drawn at.
    readonly property real naturalWidth:
        treeRun.name === "" ? 0 : treeRun.fixedWidth + Math.ceil(nameCut.implicitWidth)
    /// Everything in it but the name — the step off the name before it and the mark's seat. What the leftover has to
    /// hold before any of this is drawn at all.
    readonly property real fixedWidth: treeRun.metrics.treeRunGap + treeRun.metrics.treeSeat
    /// The least this is drawn at: the fixed parts and a name still worth reading — or, for a name shorter than
    /// that, the run itself. **A short name is all or nothing**: cutting `b` is cutting nothing, and a floor above
    /// its natural width would drop a run that fits (デザイン規約 §タブの所作).
    readonly property real floorWidth: Math.min(treeRun.naturalWidth, treeRun.fixedWidth + treeRun.minNameW)

    implicitWidth: treeRun.naturalWidth
    implicitHeight: nameCut.implicitHeight

    // The WORKTREES section's own mark, at the step a mark beside a word takes (デザイン規約 §寸法), and set here as
    // a letter of the run rather than stood in a row: **nothing is written either side of it** and what opens around
    // it is the air its own box holds (`TabMetrics.treeSeat`).
    NavIcon {
        id: treeIcon
        kind: "tree"
        anchors.left: treeRun.left
        anchors.leftMargin: treeRun.metrics.treeRunGap
        anchors.verticalCenter: treeRun.verticalCenter
        width: treeRun.metrics.treeSeat
        height: treeRun.metrics.treeSeat
        // The 16-grid scales with the seat and the line does not, so a mark this size beside a word would carry 4/3
        // the weight of the letters next to it (`NavIcon.stroke`).
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
        // A step under the name it stands beside: this is read-only meta, and the tab is read for the repository
        // (デザイン規約 §タイポグラフィ「読むだけの添え物」).
        pixelSize: Theme.fontSm
    }
}
