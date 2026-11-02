pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// What the diff pane says about the file itself, above the rows — one line each, in one seat (デザイン規約
// §改行コードの警告): the mode in the rows' weight, the rest in the notes' — and the picture that stands in for a file
// no rows can show. None of them changes the ground.
//
// With a mode change, top to bottom: the mode line, then whatever else the file says — `Line endings change · …`
// (warning), `Whole file only · stored by Git LFS` (right-aligned) then the rows, `Binary file · 8 B → 9 B`, the
// pictures before and after, or the hunks. A rename with the same contents gets the mode line alone (the file list's
// `old → new` says the rename). No mode line for a file created or deleted (`new file mode` / `deleted file mode`
// declare a file, they change none), for a change of type (file ↔ symlink: git prints a deletion and a creation), or
// for a conflict's combined diff (its `mode` line is another shape, and the conflict lines say what each side did);
// the parser reads only the `old mode` / `new mode` pair (`FilePatch::mode_change`).
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

    // -- a conflict with only one side left (`DU` / `UD` / `AU` / `UA`): the index holds one side's version only, so
    //    git prints no patch (`* Unmerged path`) and this sentence is all the pane shows — saying what each side did
    //    (`Words.conflict`) already says why, so no `No changes to show`.
    // Each side's branch behind its mark (デザイン規約 §ref の種別「名前の印」), on one line, never wrapped: the names give
    // way in the middle, the longer first, never the words (user decision — ask before changing; the menus' rule,
    // `CodeLine`, `middle`), and the whole line is the tip of a hand on it. In the explaining words' `textSecondary`,
    // not the disabled `textMuted`, which would sink the names and marks into the row's colour (user decision — ask
    // before changing): its names a step down with them, and the branch stood on in its own `textLink`.
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
    // -- the mode, first (user decisions — ask before changing): an attribute of the file, where the line endings below
    //    are how its contents are written, next to the rows. A change of the file's as its lines are, so it is said,
    //    one line, in their weight — the code face and size, in `textPrimary` — on the pane's own ground: no file holds
    //    these words, so dressed as lines it would read as diff. Unsaid, a mode-only change would look like `No changes
    //    to show` and one beside lines would go unseen (as would the mode a part left behind on its side). No row of
    //    the diff carries it, and no part of the diff moves it (`patch::header`): it has no tools. On a mode-only
    //    change it stands instead of `No changes to show` — it says the whole change.
    PlainTextLabel {
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
    // -- line endings: one line for the file, words only (デザイン規約 §改行コードの警告). This line is the diff's one
    //    mark for it — no row is marked: a CR is invisible, with nowhere in a line to stand, and the count of (b) says
    //    enough.
    PlainTextLabel {
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
    // -- a file the pane offers whole only: no hunk or line tools, and this says why (デザイン規約 §diff の中のステージ;
    //    user decision — ask before changing). The line-ending warning's seat and voice. On the right (user
    //    decision), under the header's word it explains — the staging tools stand at that edge. Only where that is the
    //    one reason tools are missing (`DiffPane.wholeWhy`): a new file's one hunk shows itself, and another
    //    worktree's diff and a commit's never had tools.
    PlainTextLabel {
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
    // -- an embedded git repository (a directory with its own `.git`, which `status` answers as one entry): no patch,
    //    ever, so the file's one line of note (user decision — ask before changing), in the line-ending warning's seat
    //    and voice, its words in `textMuted`. Heading and value: the name, and the commit it stands on in 8 digits
    //    (core `details::embedded`) — the commit staging records, so one value answers both questions. The name is
    //    git's: `git add` says `warning: adding embedded git repository`, so the log at the stage and the screen agree.
    //    Not `submodule` — a tracked gitlink's word, and a registered one never shows as this untracked row — and no
    //    noun of our own (a reader would need it explained). What staging loses (the files inside, a clone's copy) is
    //    git's to say, in its warning and hint in the log: said here too, one line would hold two causes and keep
    //    neither (規約 §長さ). A note, not `No changes to show`: that says "nothing changed", this "there is no patch at
    //    all", and the same words would make the directory look empty. The row's `+` stays — git decides whether it
    //    can, and a refusal is shown in git's words. Its file row is one line too (`models::pathtree`). Headless
    //    `wip-embedded` (preset `embedded`).
    PlainTextLabel {
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
    //    for both (user decision — ask before changing) — git is not asked a second question for why. A mode change,
    //    the third headers-only read, has its own line above (`FilePatch::mode_change`).
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
