import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The details pane's own band while the selected row is a stash: the reflog selector named in mono, and the two things
// git can do with it from here. (Delete lives on the row's menu, with the question held on the row —
// rules-refs/app-ui.md §stash.)
//
// It stands *in place of* `COMMIT`, not under it. A stash is a commit in git's storage and nowhere else — nothing here
// switches to it, nothing rewrites its message — so a band naming it as one, with a second band underneath saying what
// it really is, spends two rows of the pane on one heading and leads with the wrong word.
//
// The two words are the commands themselves (デザイン規約 §git 用語のコード表記), spelled and dressed the way the
// stash row's own right-click menu spells them: one gesture, two places to reach it, one spelling.
Rectangle {
    id: band

    /// Reflog selector ("" hides the band).
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
            text: band.stashRef
            font.family: Theme.monoFamily
            font.pixelSize: Theme.fontSm
            color: Theme.textSecondary
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
