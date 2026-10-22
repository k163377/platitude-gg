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
    readonly property real naturalWidth: treeRun.name === "" ? 0 : treeRun.fixedWidth + treeRun.nameWhole
    /// The name alone, whole and at its floor — both ends and the `…` — in the face it is drawn in, for a holder that
    /// prices the run (`OpsPicker`).
    readonly property real nameWhole: Math.ceil(nameRun.nameWidth)
    readonly property real nameFloor: Math.ceil(floorRuler.advanceWidth)
    /// Everything but the name: the gap before it and the mark's seat.
    readonly property real fixedWidth: treeRun.gap + nameRun.markCell
    /// The least this is drawn at: the fixed parts plus a readable name, or the whole run if shorter — a floor above
    /// the natural width would drop a run that fits.
    readonly property real floorWidth: Math.min(treeRun.naturalWidth, treeRun.fixedWidth + treeRun.minNameW)
    /// Whether the name was cut to the width the run was given.
    readonly property bool cutting: nameRun.cutting
    /// The natural width with the name held under the name ceiling (`Theme.nameMaxChars`), for a holder with no width
    /// ceiling of its own. The tab has one (its share of the strip) and asks for `naturalWidth`.
    readonly property real cappedWidth: Math.min(treeRun.naturalWidth, treeRun.fixedWidth + treeRun.capWidth)
    /// The name ceiling in the run's face, whole pixels.
    readonly property real capWidth: Math.ceil(capRuler.advanceWidth)

    implicitWidth: treeRun.naturalWidth
    implicitHeight: nameRun.implicitHeight

    TextMetrics {
        id: capRuler
        font.family: Theme.monoFamily
        font.pixelSize: nameRun.pixelSize
        text: "n".repeat(Theme.nameMaxChars)
    }
    // The same reading a tab's floor is taken from (規約 §レイアウト初期値).
    TextMetrics {
        id: floorRuler
        font: capRuler.font
        text: "nnn…nnn"
    }

    // The mark and the folder as one code chip (`NameRun`), a step under the tab's name in size and colour — the mark
    // keeps its own (デザイン規約 §タイポグラフィ「読むだけの添え物」).
    NameRun {
        id: nameRun
        anchors.left: treeRun.left
        anchors.leftMargin: treeRun.gap
        anchors.right: treeRun.right
        anchors.verticalCenter: treeRun.verticalCenter
        height: nameRun.implicitHeight
        name: treeRun.name
        pixelSize: Theme.fontSm
        color: Words.nameInk(Theme.textSecondary)
    }
}
