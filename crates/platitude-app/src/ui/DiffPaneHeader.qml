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
    /// Whether the file is this window's own to write (`DiffPane.writable`). **The word does not stand where it is
    /// not** — a file read out of another working copy is read the way a commit's file is, and a commit's diff has
    /// no whole-file word either (デザイン規約 §別の作業コピーを読む).
    required property bool writable
    /// The copy the file belongs to, where it is not this window's. **Said in front of the path**: this band is the
    /// only thing on screen while a diff is open that can say whose file is being read, and the pane that named the
    /// copy is behind the diff.
    required property string copyName
    required property bool conflicted
    required property bool busy

    /// The hand the path is dragged over from the band's own air, named so a run can enter it (the way a card is
    /// reached at `<card>.background.pad`).
    property alias pad: headerHand
    /// Whether the band ran out of room for the path and is showing the tail of it. The output side, and the half a
    /// picture of a wide pane cannot answer.
    readonly property alias cut: titleField.clipped
    /// Automation: whether the file's own word is there to press, read off the button rather than off what was
    /// handed in — a report built from `writable` would go green with the button unwired (app-ui.md §UI 自動化の因果性).
    readonly property bool stageOffered: stageFileButton.visible && stageFileButton.enabled

    signal stageFileRequested()
    signal closeRequested()

    implicitHeight: Theme.headerHeight
    color: Theme.bgElevated
    // This is the band the hairline was written for: the row under it is a hunk heading of the same colour.
    BandRule { z: 1 }
    // The hand the path is dragged over from the band's own air (規約 §右のペインの字は掴める): the inset at the near
    // end, the step either side of the words, the room beside a short path. Under the row rather than over it, so the
    // word that stages the file and the way out keep every press they had.
    SweepPad {
        id: headerHand
        anchors.fill: parent
        content: headerRow
    }
    RowLayout {
        id: headerRow
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
                font.pixelSize: Theme.fontMd
                font.weight: Font.DemiBold
                color: Theme.textSecondary
            }
            DotMark {
                // Nothing open, nothing to separate.
                visible: header.title !== ""
                tint: Theme.textSecondary
            }
            // Whose file, where it is not this window's. **Before the path and outside the field**: the path is a
            // value the reader takes away (`titleField`), and a copy's name pasted with it would be a path nothing
            // can open. Its own cut, so a long name loses its middle rather than the file's.
            //
            // **A quarter of the band at most**, because the path is what the band is for: a copy can be named
            // anything, and left to take what it asks for it would push the file it is about off the end.
            CutName {
                visible: header.copyName !== ""
                Layout.maximumWidth: header.width / 4
                Layout.preferredHeight: Theme.rowHeight
                text: header.copyName
                pixelSize: Theme.fontMd
                weight: Font.DemiBold
                color: Theme.textMuted
            }
            DotMark {
                visible: header.copyName !== ""
                tint: Theme.textSecondary
            }
            // The file, in a field the reader can take away with them (規約 §右のペインの字は掴める): this band is
            // where the eye already is while the diff is being read, and the path here is the same one the list's
            // own row hands over from its hover.
            //
            // **Cut in the middle** (`LineText.cutAt`): a path is told apart by both of its ends — the leaf names the
            // file and the first folders say which tree it is in — and this band has nothing beside it to carry
            // either. A field has no `elide`, so the field itself is held against the far edge and the head is
            // painted onto the mark's ground beside the `…`; the whole path stays in the field, so a drag and
            // `Ctrl+A` both come away with all of it and never with a `…`.
            LineText {
                id: titleField
                // Takes the slack, so the pair above stays put (FileRowDelegate learned this the hard way).
                Layout.fillWidth: true
                text: header.title
                pixelSize: Theme.fontMd
                weight: Font.DemiBold
                color: Theme.textSecondary
                cutAt: "middle"
                ground: Theme.bgElevated
            }
        }
        // The file's own word, and the loudest thing in the pane: the pair colour the hunks and the file rows use, a
        // step up in size, and lit whether or not the pointer is near (デザイン規約 §diff の中のステージ). It is the only standing
        // colour word in the view — the hunks' wait for the pointer — which is what puts the scopes back in order:
        // staging a file is the larger of the two.
        ActionButton {
            id: stageFileButton
            visible: header.fromWorkTree && header.writable
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
        CloseToolButton {
            onClicked: header.closeRequested()
        }
    }
}
