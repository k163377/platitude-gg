import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The details pane's band while the selected row is a stash, standing in place of `COMMIT` and dressed like
// `PaneHeader`: `STASH` with the stash mark at its left, and at its right the two things git can do with it from here
// — `apply` (keeps the entry) and `pop` (drops it) —, spelled as the stash row's menu spells them, `stash` itself being
// the band's name. In place of `COMMIT`, not under it: a `COMMIT` band with a "this is really a stash" band below
// spends two lines on the heading and makes the wrong word read first. `drop` is not here: it is held on the stash's
// row menus, the left menu's and the graph's, in the same words.
//
// The reflog spelling (`stash@{n}`) is not drawn: what the entry is, three places already say — its message in the box
// below, its name in the left list, its kind in the mark and in `apply` / `pop`. What is left is its place in the
// stack, which the left list says by listing newest first. Not drawn, so not searched for either (`find`'s module
// docs): a row must not light up for a word it does not show.
//
// Chips although the band has no column of them: the graph's stash row already says these in git's
// spelling, and one gesture must not go by two names. At `fontSm`, the band's step — one `fontMd` word among them
// would be another voice.
Rectangle {
    id: band

    /// Reflog selector — what `apply` and `pop` are run on, and `""` hides the band. Not drawn.
    property string stashRef: ""

    signal applyRequested(string selector)
    signal popRequested(string selector)

    /// One box for both words, the two answers to one question: each sized to its own word, the shorter would be the
    /// visibly smaller target (デザイン規約 §余白). Measured off the chips, which are what is drawn.
    readonly property real actBox: Math.max(applyChip.implicitWidth, popChip.implicitWidth)
    /// How far the caption moves up to stand its capitals on the band's middle (`WordFace.capitalDrop`; デザイン規約
    /// §アイコンの席「大文字だけの見出しの語」); its mark goes with it.
    readonly property real capsLift: -WordFace.capitalDrop(stashCaption)

    visible: band.stashRef !== ""
    implicitHeight: Theme.headerHeight
    color: Theme.bgElevated
    BandRule { z: 1 }
    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceXs
        anchors.rightMargin: Theme.spaceXs
        spacing: Theme.spaceXs
        // The heading's mark: a step over its capitals, its box the caption's line so the two share a baseline
        // (デザイン規約 §アイコンの席 の文頭記号の見出し).
        NavIcon {
            kind: "stash"
            tint: Theme.textSecondary
            set: "head"
            size: stashCaption.font.pixelSize
            wordBase: stashCaption.baselineOffset + band.capsLift
            Layout.preferredWidth: Theme.iconMd
            Layout.preferredHeight: stashCaption.implicitHeight
            width: Theme.iconMd
            height: stashCaption.implicitHeight
        }
        Label {
            id: stashCaption
            transform: Translate { y: band.capsLift }
            text: qsTr("STASH")
            font.pixelSize: Theme.fontMd
            font.weight: Theme.fontWeightStrong
            color: Theme.textSecondary
            // Unlike `PaneHeader`'s, this caption shares its row with the buttons and would paint over them when
            // squeezed.
            elide: Text.ElideRight
            Layout.fillWidth: true
        }
        HoverToolButton {
            padding: Theme.spaceSm
            implicitWidth: band.actBox + 2 * padding
            tip: qsTr("Apply this stash, keeping it")
            onClicked: band.applyRequested(band.stashRef)
            contentItem: CodeChip {
                id: applyChip
                word: "apply"
                size: Theme.fontSm
                buttonWord: true
                tint: Theme.textPrimary
            }
        }
        HoverToolButton {
            padding: Theme.spaceSm
            implicitWidth: band.actBox + 2 * padding
            tip: qsTr("Apply this stash and drop it")
            onClicked: band.popRequested(band.stashRef)
            contentItem: CodeChip {
                id: popChip
                word: "pop"
                size: Theme.fontSm
                buttonWord: true
                tint: Theme.textPrimary
            }
        }
    }
}
