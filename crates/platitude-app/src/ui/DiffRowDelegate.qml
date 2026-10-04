pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

// One row of the diff: as one column, two numbers with the mark's seat between them and the line; side by side, each
// side's number, seat and line either side of a hairline (デザイン規約 §diff を 2 列で読む). A heading spans both.
//
// The mark is the only thing in a line that takes a press (デザイン規約 §diff の中のステージ); which row the pointer is
// on is the pane's question (`hoverHunk` / `hoverLine`, `DiffPane.settlePointedRow`).
//
// The two layouts are Loaders — only the one in use is built (rules-refs/app-ui.md「行のデリゲートが見せない部品は消す」).
Rectangle {
    id: diffRow

    required property string kind
    required property int old_no
    required property int new_no
    /// What this row draws, as `Text.StyledText` reads it — one format for every row (`encode::DiffRow`): a row whose
    /// format changes after it is measured counts its tags as letters in the diff's reach (`DiffReach`).
    required property string text
    /// Which row this is, for the width it reports back (`DiffReach.noteRow`).
    required property int index
    /// Which reading of the rows this row's text is from (`DiffModel.rowsGen`). A new one makes every row on screen
    /// say its width again: the pane files widths per reading, and a row whose width did not change would never
    /// speak for the new one.
    required property int rowsGen
    /// Where what changed inside this row falls in the line — runs of layout places (`[{ from, len }, …]`), empty
    /// for nothing (`encode::DiffRow.emph`).
    required property var emph
    /// The line's small facts under `own` — `{fence, noNewline, side}` — and, on a split row with a line on the
    /// right, that line's under `pair` (`encode::Marks`).
    required property var marks
    /// Where the reader's selection falls on this row (`DiffLineItem.sel` — `{whole, runs}`): the whole line, or
    /// runs like `emph` on a row a drag cuts through. `undefined` wherever the plain `Copy` does not take — what is
    /// not washed is not copied (デザイン規約 §diff の中身をコピーする).
    required property var sel
    required property int hunk
    required property int line
    /// The right side of a split row: the same again for the line across from this one, `pair_kind` empty where
    /// there is none (`DiffLineItem`). Empty throughout as one column.
    required property string pair_kind
    required property string pair_text
    required property var pair_emph
    required property int pair_line
    required property var pair_sel

    required property real rowWidth
    /// How far the file's text has been sent sideways (`DiffCodeScroll.offset`); the gutter and the hunk headings
    /// stay put (デザイン規約 §diff を横へ送る).
    required property real codeX
    /// One measured column of the mono font (`DiffTextMetrics.charW`) — only the wash on an empty line needs it.
    required property real charW
    /// Asked where each wash run of this row's line is drawn (`LineRuler`) — the same ruler `DiffTextSelect` reads a
    /// press against.
    required property var ruler
    /// The room beside a number for the mark (`DiffTextMetrics.seatW`).
    required property int seatW
    /// Whether a hunk and a line can be staged on their own (`DiffPane.partial`).
    required property bool partial
    /// Which way a write on this diff goes, and whether one is running.
    required property bool staged
    required property bool busy
    /// How wide a line number is (`DiffTextMetrics.numberW`).
    required property int numberW
    /// Whether the two sides are told apart by colour, and the two colours — properties, so the pane is not asked
    /// per row.
    required property bool sidesTold
    required property var oursColor
    required property var theirsColor
    /// Which hunk and line the pointer is on (`DiffPane.settlePointedRow`, or named by the automation — hover cannot
    /// be injected). Side by side the line names the side too: a paired row's two lines are two lines of the hunk.
    required property int hoverHunk
    required property int hoverLine
    /// Whether the rows are read as two columns (`DiffModel.split`), and how wide the left one is — the right
    /// starts a hairline after it (`DiffPane.halfW`).
    required property bool split
    required property real halfW

    /// How wide this row was laid out, sent again whenever it changes — a `Text` measures every glyph at the family's
    /// advance until the fallback for the wide ones resolves, so the first answer is not the last. Filed under
    /// `index` (`DiffReach`); side by side, the wider of the two lines. Headings send none: they never travel.
    signal rowDrawn(int row, real drawn)

    /// A press on a line's own mark: this line goes over to the other side now.
    signal lineStageRequested(int hunk, int line)
    signal discardRequested(int hunk)
    signal stageHunkRequested(int hunk)

    // Automation: the hunk's discard, `null` on every row without hunk tools (`hunkTools`).
    readonly property var discardButton: hunkTools.item ? hunkTools.item.discardButton : null
    /// …and how far apart the heading's two words stand, -1 on a row that is no heading.
    function hunkWordsApart() {
        return hunkTools.item ? hunkTools.item.wordsApart() : -1
    }

    // ---- the marks, read once -----------------------------------------------------------------------------------
    readonly property var ownMarks: diffRow.marks.own
    /// The right side's, or nothing where the row has no line on the right.
    readonly property var pairMarks: diffRow.marks.pair
    /// One of git's conflict fences (`encode::DiffRow`).
    readonly property bool fence: diffRow.ownMarks.fence
    /// This line ends the file without a newline (デザイン規約 §行末の改行が無いこと).
    readonly property bool noNewline: diffRow.ownMarks.noNewline
    /// "ours" / "theirs" / "" — which side of a conflict the line came from (`side_of_markers`).
    readonly property string side: diffRow.ownMarks.side
    readonly property bool pairFence: diffRow.pairMarks ? diffRow.pairMarks.fence : false
    readonly property bool pairNoNewline: diffRow.pairMarks ? diffRow.pairMarks.noNewline : false
    readonly property string pairSide: diffRow.pairMarks ? diffRow.pairMarks.side : ""

    width: diffRow.rowWidth
    height: Theme.rowHeight
    onRowsGenChanged: diffRow.tellWidth()
    /// Says what this row was laid out at. Guarded: a reading can turn over while the row is still being built,
    /// before its parts exist. **Read off the cells, not off a binding over them** — from a cell's own
    /// `onInkWidthChanged` such a binding is still one value behind (rules-refs/app-ui.md「幅の申告はセル自身のプロパティから読む」).
    function tellWidth() {
        if (diffRow.banded)
            return
        if (diffRow.twoColumns) {
            const body = splitBody.item
            if (body)
                diffRow.rowDrawn(diffRow.index,
                                 Math.max(body.leftCell.lineCell.inkWidth, body.rightCell.lineCell.inkWidth))
        } else if (oneBody.item) {
            diffRow.rowDrawn(diffRow.index, oneBody.item.lineCell.inkWidth)
        }
    }
    /// Automation: how far right this row's ink stands inside `frame`, **asked of the placed item** — `codeInk` plus
    /// the send is built from what `codeMax` is, so checking that against `codeMax` checks a number against itself
    /// (verify-ui, `code-grow`). 0 on a heading; side by side, the further of the two lines.
    function inkRightIn(frame) {
        if (diffRow.banded)
            return 0
        const body = diffRow.twoColumns ? splitBody.item : oneBody.item
        return body ? body.inkRightIn(frame) : 0
    }
    /// A heading: a hunk's, or — where several commits' patches of one file stand one after another — the commit a
    /// block is of (デザイン規約 §複数のコミットを選ぶ). No number, no seat; it spans the row either way it is read.
    readonly property bool banded: diffRow.kind === "hunk" || diffRow.kind === "commit"
    /// Side by side, and a line of the file — a band or the binary note (`meta`) has no side and spans the row.
    readonly property bool twoColumns: diffRow.split && !diffRow.banded && diffRow.kind !== "meta"
    /// A changed line is bold, a fence is not (規約 §シンタックスハイライト). Named here because **the ruler has to be
    /// set in the weight the row is drawn in** — a bold face that advances differently puts every wash and press out
    /// (`DiffTextSelect` reads it off this row for the same reason).
    readonly property bool codeBold: !diffRow.fence && (diffRow.kind === "add" || diffRow.kind === "del")
    readonly property bool pairBold: !diffRow.pairFence && diffRow.pair_kind === "add"
    /// Automation: how far this row's own line is drawn — where the blank right of it begins
    /// (`PGG_AUTO_ACT=diff-blank`).
    readonly property real codeInk: diffRow.twoColumns
                                    ? (splitBody.item ? splitBody.item.ownInk : 0)
                                    : (oneBody.item ? oneBody.item.lineCell.inkWidth : 0)

    // ---- what the hand over the rows asks of a row ---------------------------------------------------------------
    /// One side's line, weight and kind, for the hand reading a press against the line's layout
    /// (`DiffTextSelect.byteAt`). Side 0 is the row's own line; side 1 is the right of a split row.
    function textOf(side) { return side === 1 ? diffRow.pair_text : diffRow.text }
    function boldOf(side) { return side === 1 ? diffRow.pairBold : diffRow.codeBold }
    function kindOf(side) { return side === 1 ? diffRow.pair_kind : diffRow.kind }
    /// Automation: whether one side's mark is out, read off the mark itself (`PGG_AUTO_ACT=split-tools`). As one
    /// column only side 0 has one.
    function markShown(side) {
        if (diffRow.twoColumns)
            return splitBody.item ? (side === 1 ? splitBody.item.rightMark : splitBody.item.leftMark) : false
        return side === 0 && oneBody.item ? oneBody.item.markShown : false
    }
    /// `[hunk, line]` under this row's `x`, for `DiffPane.settlePointedRow`. A heading is line -1, which lights its
    /// whole hunk; an empty seat across from a line is `[-1, -1]`, which lights nothing.
    function pointedAt(x) {
        if (diffRow.banded)
            return [diffRow.hunk, -1]
        if (!diffRow.twoColumns)
            return [diffRow.hunk, diffRow.line]
        const line = x < diffRow.halfW ? diffRow.line : diffRow.pair_line
        return line < 0 ? [-1, -1] : [diffRow.hunk, line]
    }

    color: diffRow.twoColumns ? "transparent" : diffRow.groundOf(diffRow.kind)
    function groundOf(kind) {
        return kind === "add" ? Theme.diffAddedBg
             : kind === "del" ? Theme.diffRemovedBg
             : kind === "hunk" ? Theme.diffHunkHeaderBg
             // The window's band ground (`PaneHeader`): the hunk's own would make a commit and its hunks read as one
             // kind of row.
             : kind === "commit" ? Theme.bgElevated
             // The empty seat of a split row (規約 §diff を 2 列で読む): a step up, so it does not read as a blank line.
             : kind === "" ? Theme.bgElevated
             : "transparent"
    }
    /// A heading is line -1, so a hunk named without a line means the heading.
    readonly property bool underPointer: diffRow.hoverHunk === diffRow.hunk && diffRow.hoverLine === diffRow.line
    readonly property bool pairUnderPointer: diffRow.pair_line >= 0 && diffRow.hoverHunk === diffRow.hunk
                                             && diffRow.hoverLine === diffRow.pair_line
    /// The pointer is on this hunk's heading, so the whole hunk lights — only the heading does it
    /// (デザイン規約 §diff の中のステージ).
    readonly property bool inAimedHunk: diffRow.partial && diffRow.hoverLine < 0 && diffRow.hoverHunk === diffRow.hunk
    /// The mark's seat between the numbers, none on a heading (`DiffTextMetrics.seatW`, デザイン規約 §diff の中のステージ).
    readonly property int stageSeatW: diffRow.banded ? 0 : diffRow.seatW
    /// The right edge a hunk heading gives up to its two words — git's `@@` line would otherwise run under them.
    readonly property real toolsRoom: hunkTools.item ? hunkTools.width + Theme.spaceSm * 2 : 0

    Rectangle {
        anchors.fill: parent
        color: Theme.bgHover
        visible: diffRow.inAimedHunk
        z: 1
    }

    // ---- one column ----------------------------------------------------------------------------------------------
    Loader {
        id: oneBody
        active: !diffRow.twoColumns
        anchors.fill: parent
        // Not the body's own `Component.onCompleted`: that runs before `item` is set, and finds no body to read.
        onLoaded: diffRow.tellWidth()
        sourceComponent: Item {
            /// The cell itself — `diffRow.tellWidth` reads it directly.
            readonly property alias lineCell: codeRoom
            function inkRightIn(frame) { return codeRoom.inkRightIn(frame) }
            readonly property bool markShown: markSeat.item ? markSeat.item.visible : false

            // Which side of a conflict this line came from, in that branch's graph colour — git paints both sides
            // the same green.
            Rectangle {
                visible: diffRow.sidesTold && diffRow.side !== ""
                width: Theme.spaceXs
                height: parent.height
                color: diffRow.side === "ours" ? diffRow.oursColor : diffRow.theirsColor
            }
            // The gutter stays put however far the code is sent — a `+` scrolled out of reach would take partial
            // staging with it.
            Row {
                id: gutter
                height: parent.height
                spacing: 0
                Label {
                    id: oldNoCol
                    // No padding toward the seat, which carries the whole gap between the numbers. Both columns
                    // close on a heading, which takes the row from its left edge.
                    width: diffRow.banded ? 0 : Theme.spaceXs + diffRow.numberW
                    leftPadding: Theme.spaceXs
                    height: parent.height
                    verticalAlignment: Text.AlignVCenter
                    text: diffRow.old_no >= 0 ? diffRow.old_no : ""
                    horizontalAlignment: Text.AlignRight
                    color: Theme.textMuted
                    font.family: Theme.monoFamily
                    font.pixelSize: Theme.fontSm
                }
                // The mark's seat, held open on lines without a mark too — closing it would walk the numbers
                // sideways from row to row.
                Item {
                    width: diffRow.stageSeatW
                    height: parent.height
                }
                Label {
                    width: diffRow.banded ? 0 : diffRow.numberW + Theme.spaceXs
                    height: parent.height
                    verticalAlignment: Text.AlignVCenter
                    text: diffRow.new_no >= 0 ? diffRow.new_no : ""
                    horizontalAlignment: Text.AlignRight
                    rightPadding: Theme.spaceXs
                    color: Theme.textMuted
                    font.family: Theme.monoFamily
                    font.pixelSize: Theme.fontSm
                }
            }
            DiffLineCell {
                id: codeRoom
                x: gutter.width
                width: Math.max(0, diffRow.rowWidth - gutter.width - diffRow.toolsRoom)
                height: parent.height
                text: diffRow.text
                kind: diffRow.kind
                emph: diffRow.emph
                sel: diffRow.sel
                fence: diffRow.fence
                noNewline: diffRow.noNewline
                codeBold: diffRow.codeBold
                codeX: diffRow.codeX
                charW: diffRow.charW
                ruler: diffRow.ruler
                banded: diffRow.banded
                // Again whenever the fallback resolves and the width moves; the first time is `onLoaded`.
                onInkWidthChanged: diffRow.tellWidth()
            }
            // The mark, built only on a changed line of a diff with pieces in it (the hunk tools' rule).
            Loader {
                id: markSeat
                active: diffRow.partial && (diffRow.kind === "add" || diffRow.kind === "del")
                x: oldNoCol.width + Theme.borderWidth
                anchors.verticalCenter: parent.verticalCenter
                sourceComponent: DiffStageMark {
                    shown: diffRow.underPointer
                    staged: diffRow.staged
                    busy: diffRow.busy
                    onPressed: diffRow.lineStageRequested(diffRow.hunk, diffRow.line)
                }
            }
        }
    }

    // ---- two columns ---------------------------------------------------------------------------------------------
    // Old side left, new right, each on its own ground; both travel by the one send (デザイン規約 §diff を 2 列で読む).
    Loader {
        id: splitBody
        active: diffRow.twoColumns
        anchors.fill: parent
        onLoaded: diffRow.tellWidth()
        sourceComponent: Item {
            readonly property alias leftCell: leftCell
            readonly property alias rightCell: rightCell
            readonly property real ownInk: leftCell.inkWidth
            function inkRightIn(frame) {
                return Math.max(leftCell.inkRightIn(frame), rightCell.inkRightIn(frame))
            }
            readonly property bool leftMark: leftCell.markShown
            readonly property bool rightMark: rightCell.markShown

            // One side: its one number and the seat after it, so the code stands the same step from its number as
            // in one column.
            component SideColumn: Rectangle {
                id: column
                required property string kindHere
                required property int number
                required property string textHere
                required property var emphHere
                required property var selHere
                required property bool fenceHere
                required property bool noNewlineHere
                required property bool boldHere
                required property string sideHere
                required property bool pointed
                required property int lineHere
                /// The cell itself — `diffRow.tellWidth` reads it directly.
                readonly property alias lineCell: cell
                readonly property real inkWidth: cell.inkWidth
                function inkRightIn(frame) { return cell.inkRightIn(frame) }
                readonly property bool markShown: markSeat.item ? markSeat.item.visible : false

                height: diffRow.height
                color: diffRow.groundOf(column.kindHere)
                Rectangle {
                    visible: diffRow.sidesTold && column.sideHere !== ""
                    width: Theme.spaceXs
                    height: parent.height
                    color: column.sideHere === "ours" ? diffRow.oursColor : diffRow.theirsColor
                }
                Label {
                    id: noCol
                    width: Theme.spaceXs + diffRow.numberW
                    leftPadding: Theme.spaceXs
                    height: parent.height
                    verticalAlignment: Text.AlignVCenter
                    text: column.number >= 0 ? column.number : ""
                    horizontalAlignment: Text.AlignRight
                    color: Theme.textMuted
                    font.family: Theme.monoFamily
                    font.pixelSize: Theme.fontSm
                }
                DiffLineCell {
                    id: cell
                    x: noCol.width + diffRow.stageSeatW
                    width: Math.max(0, column.width - x)
                    height: parent.height
                    text: column.textHere
                    kind: column.kindHere
                    emph: column.emphHere
                    sel: column.selHere
                    fence: column.fenceHere
                    noNewline: column.noNewlineHere
                    codeBold: column.boldHere
                    codeX: diffRow.codeX
                    charW: diffRow.charW
                    ruler: diffRow.ruler
                    banded: false
                    onInkWidthChanged: diffRow.tellWidth()
                }
                // The mark, built as in one column.
                Loader {
                    id: markSeat
                    active: diffRow.partial && (column.kindHere === "add" || column.kindHere === "del")
                    x: noCol.width + Theme.borderWidth
                    anchors.verticalCenter: parent.verticalCenter
                    sourceComponent: DiffStageMark {
                        shown: column.pointed
                        staged: diffRow.staged
                        busy: diffRow.busy
                        onPressed: diffRow.lineStageRequested(diffRow.hunk, column.lineHere)
                    }
                }
            }

            SideColumn {
                id: leftCell
                width: diffRow.halfW
                kindHere: diffRow.kind
                number: diffRow.old_no
                textHere: diffRow.text
                emphHere: diffRow.emph
                selHere: diffRow.sel
                fenceHere: diffRow.fence
                noNewlineHere: diffRow.noNewline
                boldHere: diffRow.codeBold
                sideHere: diffRow.side
                pointed: diffRow.underPointer
                lineHere: diffRow.line
            }
            Rectangle {
                x: diffRow.halfW
                width: Theme.borderWidth
                height: parent.height
                color: Theme.borderSubtle
            }
            SideColumn {
                id: rightCell
                x: diffRow.halfW + Theme.borderWidth
                width: Math.max(0, diffRow.rowWidth - x)
                kindHere: diffRow.pair_kind
                number: diffRow.new_no
                textHere: diffRow.pair_text
                emphHere: diffRow.pair_emph
                selHere: diffRow.pair_sel
                fenceHere: diffRow.pairFence
                noNewlineHere: diffRow.pairNoNewline
                boldHere: diffRow.pairBold
                sideHere: diffRow.pairSide
                pointed: diffRow.pairUnderPointer
                lineHere: diffRow.pair_line
            }
        }
    }

    // Hunk-level staging, built only on a heading of a diff with pieces in it
    // (rules-refs/app-ui.md「行のデリゲートが見せない部品は消す」).
    Loader {
        id: hunkTools
        active: diffRow.partial && diffRow.kind === "hunk"
        anchors.right: parent.right
        anchors.rightMargin: Theme.spaceSm
        anchors.verticalCenter: parent.verticalCenter
        z: 2
        sourceComponent: Row {
            readonly property alias discardButton: discardHunkButton
            /// Automation: how far apart the two words stand, baseline to baseline (`PGG_AUTO_ACT=hunk-tools`) — the
            /// hold mark makes one button deeper than the other, by an amount the face decides. Nothing on the staged
            /// side, which has the one word.
            function wordsApart() {
                return diffRow.staged ? 0
                    : Math.abs(discardHunkButton.wordBase(diffRow) - stageHunkButton.wordBase(diffRow))
            }
            spacing: Theme.spaceXs
            // Unstaged side only: on the staged side, unstaging puts the hunk back where it can be discarded. Held
            // (デザイン規約 §長押し), with no ask bar — it stands in the heading of what it takes. Shaped like a held menu
            // row: a frame around one word in a heading reads as a box come loose.
            ActionButton {
                id: discardHunkButton
                visible: !diffRow.staged
                text: qsTr("Discard hunk")
                font.pixelSize: Theme.fontSm
                // Asleep until the pointer is on this heading (デザイン規約 §diff の中のステージ); the hold mark still says
                // it is held.
                tone: diffRow.underPointer ? Theme.danger : Theme.textSecondary
                holdTone: Theme.danger
                holdMs: Metrics.holdMs
                enabled: !diffRow.busy
                // Which hunk of which reading (`HoldDriver.premise`): the pane re-reads itself behind every write, and
                // a delegate re-used across that read would discard whatever it was pointed at when the fill ran out.
                premise: diffRow.rowsGen + ":" + diffRow.hunk
                onHeld: diffRow.discardRequested(diffRow.hunk)
            }
            // The file rows' `+` / `−` colours, only under the pointer; at rest `textSecondary`, the `@@` line's own
            // (デザイン規約 §diff の中のステージ).
            ActionButton {
                id: stageHunkButton
                // As deep as the held button beside it, so the two words stand on one line: the hold mark makes that
                // one the deeper (by an amount the face decides), and a `Row` sets its children by their tops.
                height: discardHunkButton.visible ? discardHunkButton.height : stageHunkButton.implicitHeight
                text: diffRow.staged ? qsTr("Unstage hunk") : qsTr("Stage hunk")
                font.pixelSize: Theme.fontSm
                tone: !diffRow.underPointer ? Theme.textSecondary
                      : diffRow.staged ? Theme.diffRemovedFg : Theme.diffAddedFg
                enabled: !diffRow.busy
                onActivated: diffRow.stageHunkRequested(diffRow.hunk)
            }
        }
    }
    // No per-line discard: a bare `×` has no words to say what it takes (デザイン規約 §長押し「長押しの的は文字を持つ」).
}
