pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The diff pane's header band: which file is open, the one word that stages the whole of it, and the way out.
Rectangle {
    id: header

    /// What the pane is showing; empty when nothing is open.
    required property string title
    /// Whether the shown diff is a working-tree file (stageable), which side it is, whether git stopped on it, and
    /// whether a write is running (`DiffPane`).
    required property bool fromWorkTree
    required property bool staged
    required property bool conflicted
    required property bool busy

    signal stageFileRequested()
    signal closeRequested()

    implicitHeight: Theme.headerHeight
    color: Theme.bgElevated
    // The hairline every pane header closes with (see PaneHeader). This is the band it was written for: the row under
    // it is a hunk heading of the same colour.
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: Theme.borderWidth
        color: Theme.borderSubtle
        z: 1
    }
    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceSm
        anchors.rightMargin: Theme.spaceSm
        spacing: Theme.spaceSm
        // The word, the dot, and the file. Drawing the dot (`DotMark`) puts the space either side of it back in this
        // row's hands — as a glyph it was whatever a full-width cell had left over (規約 §余白).
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            Label {
                text: qsTr("DIFF")
                font.pixelSize: Theme.fontSm
                font.weight: Font.DemiBold
                color: Theme.textSecondary
            }
            DotMark {
                // Nothing open, nothing to separate.
                visible: header.title !== ""
                tint: Theme.textSecondary
            }
            Label {
                // Takes the slack, so the pair above stays put (FileRowDelegate learned this the hard way).
                Layout.fillWidth: true
                text: header.title
                font.pixelSize: Theme.fontSm
                font.weight: Font.DemiBold
                color: Theme.textSecondary
                elide: Text.ElideMiddle
            }
        }
        // The file's own word, and the loudest thing in the pane: the pair colour the hunks and the file rows use, a
        // step up in size, and lit whether or not the pointer is near (デザイン規約 §diff の中のステージ). It is the only standing
        // colour word in the view — the hunks' wait for the pointer — which is what puts the scopes back in order:
        // staging a file is the larger of the two.
        ActionButton {
            visible: header.fromWorkTree
            // The same `git add` on a conflicted file is not a staging at all — it is how git is told the conflict has
            // been dealt with, so the word says that instead (デザイン規約 §diff の中のステージ).
            text: header.conflicted ? qsTr("Mark resolved")
                  : header.staged ? qsTr("Unstage file") : qsTr("Stage file")
            tone: header.staged ? Theme.diffRemovedFg : Theme.diffAddedFg
            enabled: !header.busy
            tip: header.conflicted ? qsTr("Takes the file as it stands now") : header.staged
                 ? qsTr("Unstage the whole file at once") : qsTr("Stage the whole file at once")
            onActivated: header.stageFileRequested()
        }
        HoverToolButton {
            implicitWidth: Theme.iconLg
            implicitHeight: Theme.iconLg
            padding: 0
            contentItem: Item {
                // A step under what it closes (デザイン規約 §寸法).
                NavIcon {
                    anchors.centerIn: parent
                    width: Theme.iconSm
                    height: Theme.iconSm
                    kind: "close"
                    tint: Theme.textSecondary
                }
            }
            onClicked: header.closeRequested()
        }
    }
}
