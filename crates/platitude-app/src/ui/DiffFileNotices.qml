pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// What the diff pane says about the file itself, above the rows — one line each, in the same seat and voice
// (デザイン規約 §改行コードの警告) — and the picture that stands in for a file no rows can show.
ColumnLayout {
    id: notices

    required property var diffModel
    /// How many rows the list built; none, with nothing else to show, is `No changes to show`.
    required property int rowCount
    /// The two stage letters git reports for a conflicted path and what each side is called (`DiffPane`).
    required property string conflictChange
    required property string sideOurs
    required property string sideTheirs
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

    spacing: 0

    // -- a conflict with only one side left: git prints no patch (`* Unmerged path`), so say what each side did
    //    (デザイン規約 §conflict の種別).
    Label {
        visible: notices.diffModel.unmerged
        Layout.margins: Theme.spaceSm
        Layout.fillWidth: true
        elide: Text.ElideRight
        text: Words.conflict(notices.conflictChange, notices.sideOurs, notices.sideTheirs)
        color: Theme.textMuted
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
        oursColor: notices.oursColor
        theirsColor: notices.theirsColor
        nameCap: notices.nameCap
    }
    // -- line endings: one line for the file, words only (デザイン規約 §改行コードの警告).
    Label {
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
    //    `details::embedded`); what staging costs is git's to say (デザイン規約 §作業コピーの中の別リポジトリ).
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
    // -- headers and no hunks (a pure rename, a mode change, an empty file added): a blank frame would read as a
    //    failed load. One line for all three — git is not asked a second question for why.
    Label {
        visible: notices.rowCount === 0 && !notices.diffModel.loading && !notices.diffModel.isBinary
                 && !notices.diffModel.unmerged && !notices.diffModel.embedded
                 && notices.diffModel.previewKind === "" && notices.diffModel.title !== ""
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
