pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// What the diff pane says about the file itself, above the rows — one line each, in one seat (デザイン規約
// §改行コードの警告): the mode in the rows' weight (§mode の変更), the rest in the notes' — and the picture that stands
// in for a file no rows can show.
ColumnLayout {
    id: notices

    required property var diffModel
    /// How many rows the list built; none, with nothing else to show, is `No changes to show`.
    required property int rowCount
    /// Why the pane offers the file whole only, where that is all that keeps its tools away (`DiffPane.wholeWhy`).
    required property string wholeWhy
    /// The two stage letters git reports for a conflicted path and what each side is called (`DiffPane`).
    required property string conflictChange
    required property string sideOurs
    required property string sideTheirs
    /// The branch stood on (`DiffPane.currentBranch`): a side that is it is said in the current branch's colour.
    required property string currentBranch
    /// Whether the two sides are being told apart by colour, and the two colours themselves (`DiffPane.sidesTold`).
    required property bool sidesTold
    required property var oursColor
    required property var theirsColor
    /// How wide a branch name may run in the legend.
    required property real nameCap
    /// Automation: whether every picture the preview names has decoded or failed to, and how many are on screen
    /// (`ImagePreviewCell.settled` / `shown`).
    readonly property bool picturesSettled: (before.url === "" || before.settled) && (after.url === "" || after.settled)
    readonly property int picturesShown: (before.shown ? 1 : 0) + (after.shown ? 1 : 0)
    /// A read that came back with headers and no hunks (a pure rename, a mode change, an empty file added): nothing
    /// below the notices will ever fill in.
    readonly property bool headersOnly: notices.rowCount === 0 && !notices.diffModel.loading
                                        && !notices.diffModel.isBinary && !notices.diffModel.unmerged
                                        && !notices.diffModel.embedded && notices.diffModel.previewKind === ""
                                        && notices.diffModel.title !== ""
    /// Automation: the mode line and the whole-file line as they stand, `""` while hidden, and which of the three file
    /// lines stand, top to bottom (`mode`, `endings`, `whole`) — the order only the picture otherwise holds.
    readonly property string modeShown: modeLine.visible ? modeLine.text : ""
    readonly property string wholeShown: wholeLine.visible ? wholeLine.text : ""
    readonly property string fileLinesOrder: [modeLine, endingsLine, wholeLine].filter(line => line.visible)
                                             .sort((a, b) => a.y - b.y).map(line => line.objectName).join(",")

    /// A side's own colour: the current branch's where it is the branch stood on, none for any other.
    function sideTint(side) {
        return side !== "" && side === notices.currentBranch ? Theme.textLink : undefined
    }

    spacing: 0

    // -- a conflict with only one side left: git prints no patch (`* Unmerged path`), so say what each side did
    //    (デザイン規約 §conflict の種別).
    // Each side's branch behind its mark (デザイン規約 §ref の種別「名前の印」), on one line whose names give way in the
    // middle and never its words (`CodeLine`, `middle`). In the explaining words' colour, not the disabled one: its
    // names a step down with them, and the branch stood on in its own (デザイン規約 §conflict の種別).
    CodeLine {
        visible: notices.diffModel.unmerged
        Layout.margins: Theme.spaceSm
        Layout.fillWidth: true
        cutAt: "middle"
        text: Words.conflict(notices.conflictChange, notices.sideOurs, notices.sideTheirs)
        marks: Words.marksOf((ours, theirs) => Words.conflict(notices.conflictChange, ours, theirs),
                             [[notices.sideOurs, "branch", notices.sideTint(notices.sideOurs)],
                              [notices.sideTheirs, "branch", notices.sideTint(notices.sideTheirs)]])
        color: Theme.textSecondary
    }
    // -- which branch each colour is, only while the rows are told apart by colour. The rebase swap is already sorted
    //    by the model (デザイン規約 §conflict の ours / theirs).
    ConflictSideLegend {
        visible: notices.sidesTold
        Layout.leftMargin: Theme.spaceSm
        Layout.rightMargin: Theme.spaceSm
        Layout.topMargin: Theme.spaceXs
        Layout.bottomMargin: Theme.spaceXs
        Layout.fillWidth: true
        oursName: Words.ourSide(notices.sideOurs)
        theirsName: Words.theirSide(notices.sideTheirs)
        oursIsBranch: notices.sideOurs !== ""
        theirsIsBranch: notices.sideTheirs !== ""
        oursTint: notices.sideTint(notices.sideOurs)
        theirsTint: notices.sideTint(notices.sideTheirs)
        oursColor: notices.oursColor
        theirsColor: notices.theirsColor
        nameCap: notices.nameCap
    }
    // -- the mode, first: an attribute of the file, where the line endings below are how its contents are written,
    //    next to the rows. A change of the file's as its lines are, so it is said in their weight — the code face
    //    and size, in `textPrimary` — on the pane's own ground: no file holds these words, so they are not dressed as
    //    lines (デザイン規約 §mode の変更). No row of the diff carries it, and no part of the diff moves it.
    Label {
        id: modeLine
        objectName: "mode"
        visible: notices.diffModel.modeFrom !== ""
        Layout.margins: Theme.spaceSm
        Layout.fillWidth: true
        elide: Text.ElideRight
        text: Words.modeLine(notices.diffModel.modeFrom, notices.diffModel.modeTo)
        font.family: Theme.monoFamily
        font.pixelSize: Theme.fontCode
        color: Theme.textPrimary
    }
    // -- line endings: one line for the file, words only (デザイン規約 §改行コードの警告).
    Label {
        id: endingsLine
        objectName: "endings"
        visible: notices.diffModel.endingKind !== ""
        Layout.margins: Theme.spaceSm
        Layout.fillWidth: true
        elide: Text.ElideRight
        text: Words.lineEndings(notices.diffModel.endingKind, notices.diffModel.endingFrom, notices.diffModel.endingTo,
                                notices.diffModel.endingLines, notices.diffModel.endingScope,
                                notices.diffModel.endingExt)
        font.pixelSize: Theme.fontSm
        color: Theme.warning
    }
    // -- a file the pane offers whole only: no hunk or line tools, and this says why (デザイン規約 §diff の中のステージ).
    //    On the right, under the header's word it explains — the staging tools stand at that edge.
    Label {
        id: wholeLine
        objectName: "whole"
        visible: notices.wholeWhy !== ""
        Layout.margins: Theme.spaceSm
        Layout.fillWidth: true
        horizontalAlignment: Text.AlignRight
        elide: Text.ElideLeft
        text: Words.wholeOnlyLine(notices.wholeWhy)
        font.pixelSize: Theme.fontSm
        color: Theme.warning
    }
    // -- content preview: binaries summarized by size, images rendered (added = After only, deleted = Before only,
    //    modified = both).
    BinarySizeLine {
        visible: notices.diffModel.previewKind === "binary"
                 || (notices.diffModel.isBinary && notices.diffModel.previewKind === "")
        Layout.margins: Theme.spaceSm
        Layout.fillWidth: true
        oldSize: notices.diffModel.previewOldSize
        newSize: notices.diffModel.previewNewSize
    }
    // -- an embedded git repository: no patch, ever. It names the commit staging would record (core
    //    `details::embedded`); what staging costs is git's to say (デザイン規約 §作業ツリーの中の別リポジトリ).
    Label {
        visible: notices.diffModel.embedded
        Layout.margins: Theme.spaceSm
        Layout.fillWidth: true
        elide: Text.ElideRight
        text: notices.diffModel.embeddedSha8 !== ""
            ? qsTr("Embedded git repository · on %1").arg(notices.diffModel.embeddedSha8)
            : qsTr("Embedded git repository · no commits yet")
        color: Theme.textMuted
    }
    // -- headers and no hunks (a pure rename, an empty file added): a blank frame would read as a failed load. One line
    //    for both — git is not asked a second question for why. A mode change has its own line above.
    Label {
        visible: notices.headersOnly && notices.diffModel.modeFrom === ""
        Layout.margins: Theme.spaceSm
        Layout.fillWidth: true
        elide: Text.ElideRight
        text: qsTr("No changes to show")
        color: Theme.textMuted
    }
    RowLayout {
        visible: notices.diffModel.previewKind === "image"
        Layout.fillWidth: true
        Layout.fillHeight: true
        Layout.margins: Theme.spaceSm
        spacing: Theme.spaceSm
        ImagePreviewCell {
            id: before
            Layout.fillWidth: true
            Layout.fillHeight: true
            caption: qsTr("Before")
            url: notices.diffModel.previewOldUrl
            sizeText: notices.diffModel.previewOldSize
            isVector: notices.diffModel.previewVector
        }
        ImagePreviewCell {
            id: after
            Layout.fillWidth: true
            Layout.fillHeight: true
            caption: qsTr("After")
            url: notices.diffModel.previewNewUrl
            sizeText: notices.diffModel.previewNewSize
            isVector: notices.diffModel.previewVector
        }
    }
}
