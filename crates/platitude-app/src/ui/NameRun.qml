import QtQuick
import platitude.ui

// A name behind its kind's mark as one code chip, cut in the middle to the width it is given (デザイン規約 §ref の種別
// 「文の中の名前」): the run a band, a tab or a row draws where it names a worktree (`TabTreeMark`, `CarriedPane`,
// `DiffPaneHeader`, the uncommitted row) — so a fix to the run reaches them all. The mark in its kind's colour, the
// name in the code face, both on one ground half a gap off their ink (`WordFace.groundStart` / `groundEnd`).
Item {
    id: run

    property string name: ""
    /// The mark's `NavIcon` kind: `tree`, or `home` for the repository's own worktree.
    property string kind: "tree"
    property real pixelSize: Theme.fontMd
    property int weight: Font.Normal
    /// The name's colour: plain, or the line's where that line cannot be pressed (`Words.nameInk`).
    property color color: Theme.textPrimary

    /// Whether the name was cut to the width the run was given.
    readonly property bool cutting: nameCut.cutting
    /// The mark's seat ahead of the name.
    readonly property real markCell: runMark.wordCell
    /// The name's whole width, uncut.
    readonly property real nameWidth: nameCut.implicitWidth

    implicitWidth: run.markCell + nameCut.implicitWidth
    implicitHeight: nameCut.implicitHeight
    baselineOffset: nameCut.y + nameCut.baselineOffset

    /// The code face at the run's size, for where the name's ink ends (`WordFace.groundEnd`).
    FontMetrics {
        id: codeFace
        font.family: Theme.monoFamily
        font.pixelSize: run.pixelSize
        font.weight: run.weight
    }
    CodeGround {
        visible: run.name !== ""
        x: WordFace.groundStart(runMark.x + runMark.inkLeft)
        width: WordFace.groundEnd(codeFace, run.name, nameCut.x + nameCut.inkWidth) - x
        anchors.verticalCenter: nameCut.verticalCenter
        size: run.pixelSize
    }
    // A sign on the name (デザイン規約 §アイコンの席 の修飾): its box is the name's line, so the two stand on one
    // baseline, and its cell the face's own room round it.
    NavIcon {
        id: runMark
        x: 0
        width: runMark.wordCell
        height: run.height
        kind: run.kind
        set: "sign"
        size: run.pixelSize
        wordBase: nameCut.baselineOffset
        tint: runMark.inkBeside(run.color)
    }
    CutName {
        id: nameCut
        x: run.markCell
        width: Math.max(0, run.width - run.markCell)
        height: run.height
        text: run.name
        pixelSize: run.pixelSize
        weight: run.weight
        mono: true
        color: run.color
    }
}
