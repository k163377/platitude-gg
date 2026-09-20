import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The details pane's own band while the selected row is a stash: what kind of thing the pane is showing, and the two
// things git can do with it from here. (Delete lives on the row's menu, with the question held on the row —
// rules-refs/app-ui.md §stash.)
//
// It stands *in place of* `COMMIT`, not under it. A stash is a commit in git's storage and nowhere else — nothing here
// switches to it, nothing rewrites its message — so a band naming it as one, with a second band underneath saying what
// it really is, spends two rows of the pane on one heading and leads with the wrong word.
//
// **The caption is a heading, wearing what `PaneHeader` puts on `COMMIT`** — the band this one replaces, so the two
// read as the same row of the pane answering the same question. **The reflog selector is not written here**
// (デザイン規約 §変更を退避する): the entry's own name is the message in the box below and the row in the list on the
// left, and `stash@{n}` is a handle that shifts the moment the next push lands in front of it. No row is searched
// for that spelling either (`platitude-core::find`), so no query lights a row for a word no row carries.
//
// The two words are the commands themselves (デザイン規約 §git 用語のコード表記), spelled and dressed the way the
// stash row's own right-click menu spells them: one gesture, two places to reach it, one spelling.
Rectangle {
    id: band

    /// Reflog selector — what `apply` and `pop` are run on, and `""` hides the band. Not drawn (see above).
    property string stashRef: ""

    signal applyRequested(string selector)
    signal popRequested(string selector)

    /// Both words stand in the same box. They are the two answers to one question, and a pair drawn to its own word
    /// reads as two unrelated buttons — the wash and the hover of the shorter one would be visibly the smaller target
    /// (デザイン規約 §余白). Measured off the chips, which are what is actually drawn.
    readonly property real actBox: Math.max(applyChip.implicitWidth, popChip.implicitWidth)

    visible: band.stashRef !== ""
    implicitHeight: Theme.headerHeight
    color: Theme.bgElevated
    BandRule { z: 1 }
    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceXs
        anchors.rightMargin: Theme.spaceXs
        spacing: Theme.spaceXs
        NavIcon {
            kind: "stash"
            tint: Theme.textSecondary
            width: Theme.iconMd
            height: Theme.iconMd
        }
        Label {
            text: qsTr("STASH")
            font.pixelSize: Theme.fontMd
            font.weight: Font.DemiBold
            color: Theme.textSecondary
            // `PaneHeader` has nothing beside it and needs none of this; here the two buttons are in the same row, and
            // a caption that outgrows the squeezed pane would paint over them.
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
                tint: Theme.textPrimary
            }
        }
    }
}
