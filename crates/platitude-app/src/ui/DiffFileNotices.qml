pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// What the diff pane says about the file itself: the conflict with only one side left,
// which branch each colour is, the line endings, the size of a binary, the diff that came back empty — and the picture
// that stands in for a file no rows can show.
//
// One line each, in the same seat and the same voice (デザイン規約 §diff の中のステージ). They sit above the rows and give their
// space back the moment they have nothing to say.
ColumnLayout {
    id: notices

    required property var diffModel
    /// How many rows the list built. A diff with none and nothing else to show is the one that says so out loud.
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
    /// (`ImagePreviewCell.settled` / `shown`). A side with no URL has nothing to wait for.
    readonly property bool picturesSettled: (before.url === "" || before.settled) && (after.url === "" || after.settled)
    readonly property int picturesShown: (before.shown ? 1 : 0) + (after.shown ? 1 : 0)

    spacing: 0

    // -- a conflict with only one side left: git has two versions of the path but not the third to compare them
    //    against, so it prints no patch at all (`* Unmerged path`). The one thing worth saying is what the two sides
    //    each did — and this pane is the one place that sentence is shown (デザイン規約 §conflict の種別). Which way out to take
    //    is still the file row's right-click, unchanged.
    Label {
        visible: notices.diffModel.unmerged
        Layout.margins: Theme.spaceSm
        Layout.fillWidth: true
        elide: Text.ElideRight
        text: Words.conflict(notices.conflictChange, notices.sideOurs, notices.sideTheirs)
        color: Theme.textMuted
    }
    // -- which branch each of the two colours is. The colours are the ones the graph already gives those branches, so
    //    this line is the whole of what has to be learned; it is only here when there are two colours to bind names to
    //    (`sidesTold`), since otherwise it would explain a distinction the rows are not making. The names swap over
    //    during a rebase and the model has already sorted that out (デザイン規約 §conflict の ours / theirs), so this says
    //    whatever it is handed.
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
    // -- line endings: one line for the file. A CR is invisible and has nowhere inside a line to sit, and the mixed
    //    case is already saying how many lines it is about. It is a statement and nothing else — `warning` because it
    //    is a change that carries past this machine (デザイン規約 §状態の 3 段).
    Label {
        visible: notices.diffModel.endingKind !== ""
        // The binary notice's seat, down to the margins: both are one line about the
        // file itself.
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
    // -- a git repository of its own, sitting in the working copy. git will not open it, so there is no patch here and
    //    never will be. The line names the thing and says which commit it stands on — the same commit staging the row
    //    would record (core `details::embedded`). **What that costs is git's to say**: the warning and
    //    the hint land in the command log the moment the row is staged (デザイン規約 §作業コピーの中の別リポジトリ / §長さ). The
    //    binary notice's seat and voice, like the others here.
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
    // -- a diff whose body is empty. git prints headers and no hunks for a rename that changed nothing, for a mode-only
    //    change and for an empty file added, and a pane that answers all three with a blank frame reads as one that
    //    failed to load. The binary notice's seat and voice: one line about the file itself.
    //
    //    Said the same way for all three: what the pane knows is that there is nothing to
    //    show, and git is not asked a second question to find out why.
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
