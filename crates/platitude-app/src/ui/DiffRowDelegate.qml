pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

// One row of the diff. Read as one column: its two numbers, the seat between them where a changed line puts its
// mark out, the line itself, and — on a hunk heading — the two words that act on the whole hunk. Read side by side
// (`split`): the old side's number, seat and line on the left, the new side's on the right, a hairline between
// them (デザイン規約 §diff を 2 列で読む). A heading spans both.
//
// **The mark is the only thing in a row that takes a press** (デザイン規約 §diff の中のステージ). The text stays free for
// the hand that wants to read or copy it — and whether the pointer is here is the pane's own question: it
// works it out and names the row (`hoverHunk` / `hoverLine`, see
// `DiffPane.settlePointedRow`).
//
// Everything the pane knows arrives as a property; everything the pane has to do about a press leaves as a signal.
// **The body is built for the reading it is in** — the two layouts are Loaders, since a delegate is built per row
// on screen and a second gutter and cell every row never shows is the heap this pane is measured by
// (rules-refs/app-ui.md, the Loader rule).
Rectangle {
    id: diffRow

    required property string kind
    required property int old_no
    required property int new_no
    /// What this row draws, as `Text.StyledText` reads it — every row, coloured or not (`encode::DiffRow`). One format:
    /// a row whose format arrived a moment after its text was measured with its own tags counted as
    /// letters, and the width that came off it set how far the whole diff could be sent (`DiffReach`).
    required property string text
    /// Which row this is, for the width it reports back (`DiffReach.noteRow`).
    required property int index
    /// Which reading of the rows this row's text is from (`DiffModel.rowsGen`). A new one makes every row on screen
    /// say its width again: the pane files widths per reading, and a row whose text came out the same width would
    /// otherwise never speak for the new one.
    required property int rowsGen
    /// Where what changed inside this row falls in the line as this row spells it — `"from:len,…"` in the places a
    /// layout counts, empty for nothing (`encode::DiffRow.emph`).
    required property string emph
    /// The small facts about the line as letters — `f` fence, `n` no newline at the end, `o` / `t` the side of a
    /// conflict — and after a `|` the same for the right side of a split row (`DiffLineItem::marks`). Decoded once
    /// below.
    required property string marks
    /// Where the reader's own selection falls on this row (`DiffModel.sel`): `"*"` for a line taken end to end —
    /// which is what almost every selected row is — and otherwise the same `from:len` runs `emph` carries, for the
    /// one or two rows a drag cuts through.
    ///
    /// Empty on every row the plain `Copy` does not take: outside the selection, and on the lines of the other
    /// side and hunk headings inside it. **The wash is the answer** — what is not washed is not copied
    /// (デザイン規約 §diff の中身をコピーする).
    required property string sel
    required property int hunk
    required property int line
    /// The right side of a split row: the same again for the line across from this one, `pair_kind` empty where
    /// there is none (`DiffLineItem`). Empty throughout as one column.
    required property string pair_kind
    required property string pair_text
    required property string pair_emph
    required property int pair_line
    required property string pair_sel

    required property real rowWidth
    /// How far the file's own text has been sent sideways (`DiffCodeScroll.offset`). The gutter and the hunk headings
    /// stay put: the numbers, the mark and the pane's own words are about the row
    /// (デザイン規約 §diff を横へ送る).
    required property real codeX
    /// One measured column of the mono font (`DiffPane.charMeasure`). It is how wide the
    /// wash on an **empty** line is, and that is the only place a width without characters behind it is needed.
    required property real charW
    /// Where the two washes go: this row's own line, laid out, asked where each run of it is drawn
    /// (`LineRuler`). The same ruler the hand over the rows reads a press against (`DiffTextSelect`).
    required property var ruler
    /// The room held beside a number for the mark, as the pane works it out once for every row (`DiffPane.seatW`).
    required property int seatW
    /// This diff has pieces worth naming (`DiffPane.partial`).
    required property bool partial
    /// Which way a write on this diff goes, and whether one is running.
    required property bool staged
    required property bool busy
    /// How wide a line number is (`DiffPane.numberW`).
    required property int numberW
    /// Whether the two sides are being told apart by colour, and the two colours themselves — held as properties, so
    /// the pane is never asked per row.
    required property bool sidesTold
    required property var oursColor
    required property var theirsColor
    /// Which hunk and line the pointer is on, as the pane works it out (`DiffPane.settlePointedRow`) — and as the
    /// automation names it, since hover cannot be injected (verify-ui). One pair, one answer, whichever way it was
    /// arrived at. Side by side, the line names the side too: the two lines of a paired row are two lines of the
    /// hunk.
    required property int hoverHunk
    required property int hoverLine
    /// Whether the rows are read as two columns (`DiffModel.split`), and how wide the left one is — the right
    /// starts a hairline after it (`DiffPane.halfW`).
    required property bool split
    required property real halfW

    /// How wide this row was laid out, as soon as that is known and again whenever it changes — a `Text` answers with
    /// every glyph at the family's own advance until the fallback carrying the wide ones is resolved, so the first
    /// number out of it is not the last (`DiffTextMetrics`). The pane files it under `index`, so a second answer
    /// replaces the first (`DiffReach`). Side by side it is the wider of the two lines: both travel by the one send.
    ///
    /// Headings do not send one: they stand at the pane's own left edge and never travel, so how wide their words are
    /// is not how far there is to go.
    signal rowDrawn(int row, real drawn)

    /// A press on a line's own mark: this line goes over to the other side now.
    signal lineStageRequested(int hunk, int line)
    signal discardRequested(int hunk)
    signal stageHunkRequested(int hunk)

    // The hunk's discard lives in its heading. Reached from outside for the smoke run, which holds it the way a hand
    // does. `null` on every row that is not a heading with tools on it — the tools are built only there (`hunkTools`).
    readonly property var discardButton: hunkTools.item ? hunkTools.item.discardButton : null

    // ---- the letters, read once ---------------------------------------------------------------------------------
    readonly property var marksOf: diffRow.marks.split("|")
    readonly property string ownMarks: diffRow.marksOf[0]
    readonly property string pairMarks: diffRow.marksOf.length > 1 ? diffRow.marksOf[1] : ""
    /// One of git's conflict fences (`encode::DiffRow`).
    readonly property bool fence: diffRow.ownMarks.indexOf("f") >= 0
    /// This line ends the file without a newline (デザイン規約 §行末の改行が無いこと).
    readonly property bool noNewline: diffRow.ownMarks.indexOf("n") >= 0
    /// "ours" / "theirs" / "" — which side of a conflict the line came from (`side_of_markers`).
    readonly property string side: diffRow.sideOf(diffRow.ownMarks)
    readonly property bool pairFence: diffRow.pairMarks.indexOf("f") >= 0
    readonly property bool pairNoNewline: diffRow.pairMarks.indexOf("n") >= 0
    readonly property string pairSide: diffRow.sideOf(diffRow.pairMarks)
    function sideOf(letters) {
        return letters.indexOf("o") >= 0 ? "ours" : letters.indexOf("t") >= 0 ? "theirs" : ""
    }

    width: diffRow.rowWidth
    height: Theme.rowHeight
    onRowsGenChanged: diffRow.tellWidth()
    /// Says what this row was laid out at. Guarded on the cells: a reading can turn over while
    /// this row is still being built, and the parts of a delegate exist only once the whole of it does.
    ///
    /// **Read off the cells, not off a binding over them.** This runs from a cell's own `onInkWidthChanged`,
    /// and a property derived from that width on the body between them is still one value behind at that
    /// moment (app-ui.md: `onXChanged` runs before the bindings drawn from `x`) — measured: the deep line's
    /// width was filed under the row its delegate was reused for next, and its own row never got it.
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
    /// Automation: how far right this row's ink stands inside `frame` — **asked of the item where it was placed**,
    /// which is the one reading of that edge owing the reach nothing. `codeInk` carried through the send describes
    /// the same edge, but that arithmetic runs through the gutter and the offset, which is what `codeMax` is built
    /// out of, so a run holding the result against `codeMax` would be checking a number against itself — and the
    /// far end of a send is the only place a reach measured past the end of every line can be seen at all
    /// (verify-ui, `code-grow`). A heading does not travel, so it answers 0; where the file's own text ends is a
    /// question for the lines — side by side, the further of the two.
    function inkRightIn(frame) {
        if (diffRow.banded)
            return 0
        const body = diffRow.twoColumns ? splitBody.item : oneBody.item
        return body ? body.inkRightIn(frame) : 0
    }
    /// A row that names something: the hunk's own heading, and — where several
    /// commits' patches of one file stand one after another — the commit each block is of
    /// (デザイン規約 §複数のコミットを選ぶ). Neither has a line number, a stage seat or a place in the gutter, and
    /// both span the row whichever way it is read.
    readonly property bool banded: diffRow.kind === "hunk" || diffRow.kind === "commit"
    /// Whether this row is laid out as two columns: side by side, and a line rather than a band.
    /// Whether this row is laid out as two columns: side by side, and a line of the file rather than a band or the
    /// binary note — those span the row whichever way it is read, having no side to be on.
    readonly property bool twoColumns: diffRow.split && !diffRow.banded && diffRow.kind !== "meta"
    /// A changed line is bold (規約 §シンタックスハイライト) — the wash says which side it is, the weight is what makes
    /// it stand off the context around it. The fences keep the plain weight: git's scaffolding is not a change to
    /// read. Named here, because **the ruler has to be set in the weight the row is drawn in**
    /// — a family whose bold face advances differently would otherwise put every wash and every press out by the
    /// difference (`DiffTextSelect` reads it off this row for the same reason).
    readonly property bool codeBold: !diffRow.fence && (diffRow.kind === "add" || diffRow.kind === "del")
    readonly property bool pairBold: !diffRow.pairFence && diffRow.pair_kind === "add"
    /// Automation: how far this row's own line is drawn, which is where the blank right of it begins
    /// (`PGG_AUTO_ACT=diff-blank`). The same number the row files under `rowDrawn`, as a property — a run presses
    /// beside one row, and no count of that row's characters can find this edge.
    readonly property real codeInk: diffRow.twoColumns
                                    ? (splitBody.item ? splitBody.item.ownInk : 0)
                                    : (oneBody.item ? oneBody.item.lineCell.inkWidth : 0)

    // ---- what the hand over the rows asks of a row ---------------------------------------------------------------
    /// The line one side of this row holds, the weight it is set in, and what kind of row it is there — for the hand
    /// that reads a press against the line's own layout (`DiffTextSelect.byteAt`). Side 0 is the row's own line,
    /// which as one column is the whole row; side 1 is the right of a split row.
    function textOf(side) { return side === 1 ? diffRow.pair_text : diffRow.text }
    function boldOf(side) { return side === 1 ? diffRow.pairBold : diffRow.codeBold }
    function kindOf(side) { return side === 1 ? diffRow.pair_kind : diffRow.kind }
    /// Automation: whether one side's mark is out — read off the mark itself, not off the pointer that would show
    /// it (`PGG_AUTO_ACT=split-tools`). As one column the row has one mark, and it is side 0's.
    function markShown(side) {
        if (diffRow.twoColumns)
            return splitBody.item ? (side === 1 ? splitBody.item.rightMark : splitBody.item.leftMark) : false
        return side === 0 && oneBody.item ? oneBody.item.markShown : false
    }
    /// Which hunk and line the pointer over this row's `x` is on, as `[hunk, line]` — the pair the pane names
    /// (`DiffPane.settlePointedRow`). A heading is named as itself (line -1), which is what lights its whole hunk;
    /// an empty seat across from a line is nothing at all (`[-1, -1]`), which lights nothing.
    function pointedAt(x) {
        if (diffRow.banded)
            return [diffRow.hunk, -1]
        if (!diffRow.twoColumns)
            return [diffRow.hunk, diffRow.line]
        const line = x < diffRow.halfW ? diffRow.line : diffRow.pair_line
        return line < 0 ? [-1, -1] : [diffRow.hunk, line]
    }

    color: diffRow.twoColumns ? "transparent" : diffRow.groundOf(diffRow.kind)
    /// The ground a line of each kind stands on: the diff's pair of colours, the heading's band, and nothing.
    function groundOf(kind) {
        return kind === "add" ? Theme.diffAddedBg
             : kind === "del" ? Theme.diffRemovedBg
             : kind === "hunk" ? Theme.diffHunkHeaderBg
             // The window's own band ground, the one every heading in it wears (`PaneHeader`): a commit is a step
             // above the hunks it brought, and wearing the hunk's own would make the two read as one kind of row.
             : kind === "commit" ? Theme.bgElevated
             // The empty seat across from a line nothing replaced (規約 §diff を 2 列で読む): a step up from the
             // pane's ground, so it reads as a place with nothing in it rather than as a blank line of the file.
             : kind === "" ? Theme.bgElevated
             : "transparent"
    }
    /// Under the pointer. A heading's own row is line -1, so a hunk named without a line means the heading.
    readonly property bool underPointer: diffRow.hoverHunk === diffRow.hunk && diffRow.hoverLine === diffRow.line
    /// The same for the right side of a split row, whose line is its own.
    readonly property bool pairUnderPointer: diffRow.pair_line >= 0 && diffRow.hoverHunk === diffRow.hunk
                                             && diffRow.hoverLine === diffRow.pair_line
    /// The pointer is on this hunk's heading, so the whole hunk lights: the heading's two words act on exactly these
    /// rows, and this is what says so (デザイン規約 §diff の中のステージ). Only the heading does it — a pointer resting on a line is
    /// reading.
    readonly property bool inAimedHunk: diffRow.partial && diffRow.hoverLine < 0 && diffRow.hoverHunk === diffRow.hunk
    /// The room held beside a line number for the mark this line puts out for the hand. A hairline of air on each
    /// side of it: the mark belongs to neither number, and anything wider reads as the new number having drifted
    /// off its own column.
    ///
    /// Where no line can be staged on its own the seat closes to the plain gap — a diff with no pieces in it never puts
    /// a mark out, and holding the room open would leave a hole nothing ever stands in.
    readonly property int stageSeatW: diffRow.banded ? 0 : diffRow.seatW
    /// How much of the row's right edge the hunk heading has to give up to the two words that act on the hunk. git's
    /// `@@` line is as long as the enclosing signature and would otherwise run under them.
    readonly property real toolsRoom: hunkTools.item ? hunkTools.width + Theme.spaceSm * 2 : 0

    // The hunk under the pointer wears the wash a row anywhere else in the app wears under one.
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
        // The body's own `Component.onCompleted` runs before `item` is set on this loader, so a width said from
        // inside it finds no body to read; the loader says it once the body is its.
        onLoaded: diffRow.tellWidth()
        sourceComponent: Item {
            /// The cell itself, for the width the row files (`diffRow.tellWidth` reads it there and then) and
            /// where its ink ends (`inkRightIn`).
            readonly property alias lineCell: codeRoom
            function inkRightIn(frame) { return codeRoom.inkRightIn(frame) }
            /// Whether the mark is out (`diffRow.markShown`).
            readonly property bool markShown: markSeat.item ? markSeat.item.visible : false

            // Which side this line came from, in the colour that branch wears in the graph. The diff's own green
            // cannot say it — git paints our side and theirs the same, because each is in the file and in neither
            // of the other's — so the head of the row says it instead.
            Rectangle {
                visible: diffRow.sidesTold && diffRow.side !== ""
                width: Theme.spaceXs
                height: parent.height
                color: diffRow.side === "ours" ? diffRow.oursColor : diffRow.theirsColor
            }
            // The gutter: two numbers with the mark's seat between them. It stays where it is however far the code
            // is sent sideways — a number belongs to the row, and a `+` that scrolled out of reach would take
            // partial staging with it.
            Row {
                id: gutter
                height: parent.height
                spacing: 0
                Label {
                    id: oldNoCol
                    // The pane's edge and the code stand one `spaceXs` from the numbers; what stands between the
                    // two numbers is the line's own mark (`stageSeatW`), so this column keeps no padding on that
                    // side — the seat carries the whole gap.
                    //
                    // Both columns close on a hunk heading, which has no line to number: the heading takes the row
                    // from its left edge.
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
                // Where the mark below stands. Empty, and held open on every row of a diff that can be taken apart:
                // a context line has no mark, and a seat that closed on the rows without one would walk the numbers
                // left and right under the pointer.
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
            // The room the file's own text is read in (`DiffLineCell`). Cut to what is left of the row.
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
                // How far this row is drawn, filed under its own row number (`diffRow.rowDrawn`) — the first time
                // by the loader (`onLoaded`), and again whenever the fallback resolves and the width moves.
                onInkWidthChanged: diffRow.tellWidth()
            }
            // The mark a changed line puts out for the hand, in the seat between the two numbers (`stageSeatW`).
            //
            // **Built only on a line that can be staged on its own** — a changed line of a diff with pieces in it.
            // The rows of a commit's diff never carry one, and the context lines of any diff do not either, so on
            // those it is left unbuilt (the hunk tools' rule).
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
    // The old side on the left, the new on the right, each on its own ground with its one number, its seat and its
    // line (デザイン規約 §diff を 2 列で読む). Both lines travel by the one send.
    Loader {
        id: splitBody
        active: diffRow.twoColumns
        anchors.fill: parent
        onLoaded: diffRow.tellWidth()
        sourceComponent: Item {
            /// The two columns, for the widths the row files (`diffRow.tellWidth`).
            readonly property alias leftCell: leftCell
            readonly property alias rightCell: rightCell
            /// The left line's own ink, for the run that presses beside a row (`diffRow.codeInk`).
            readonly property real ownInk: leftCell.inkWidth
            function inkRightIn(frame) {
                return Math.max(leftCell.inkRightIn(frame), rightCell.inkRightIn(frame))
            }
            /// Whether each side's mark is out (`diffRow.markShown`).
            readonly property bool leftMark: leftCell.markShown
            readonly property bool rightMark: rightCell.markShown

            // One column, either side of the hairline. Its ground is its own line's; the gutter is one number
            // and the seat after it, so the code stands the same step from its number as it does in one column.
            component SideColumn: Rectangle {
                id: column
                /// What stands on this side: the line's kind, number, text, washes and letters, the weight it is
                /// set in, whether the pointer is on it, and which line of the hunk it is.
                required property string kindHere
                required property int number
                required property string textHere
                required property string emphHere
                required property string selHere
                required property bool fenceHere
                required property bool noNewlineHere
                required property bool boldHere
                required property string sideHere
                required property bool pointed
                required property int lineHere
                /// The cell itself (`diffRow.tellWidth` reads its width there and then), what the row files and
                /// where the ink ends (`inkWidth` / `inkRightIn` above).
                readonly property alias lineCell: cell
                readonly property real inkWidth: cell.inkWidth
                function inkRightIn(frame) { return cell.inkRightIn(frame) }
                /// Whether this column's mark is out.
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
                // The mark, in the seat after this column's number — built only on a changed line of a diff with
                // pieces in it, as in one column.
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
            // The hairline between the two, the one every pane edge in the window is drawn with.
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

    // Hunk-level staging. The row carries the hunk index the patch builder needs, so what is staged is exactly what is
    // shown — and so is what is thrown away. Absent on a diff with no pieces in it (see `partial`).
    //
    // **Built only on a heading that has them.** A delegate is built per line on
    // screen, and each `ActionButton` is a label with its rulers, a hold and its timers — on the lines that are not
    // headings the pair was the heaviest thing in the row while drawing nothing — tens of megabytes of heap on a diff
    // of a few dozen lines, most of it in parts the rows never showed (ci/baseline/code-costs-windows-x64.md §メモリの形).
    // Same rule as the dialogs behind `WindowDialogSeat` (rules-refs/app-ui.md).
    Loader {
        id: hunkTools
        active: diffRow.partial && diffRow.kind === "hunk"
        anchors.right: parent.right
        anchors.rightMargin: Theme.spaceSm
        anchors.verticalCenter: parent.verticalCenter
        z: 2
        sourceComponent: Row {
            /// The smoke run's handle on the discard, read through the loader (`diffRow.discardButton`).
            readonly property alias discardButton: discardHunkButton
            spacing: Theme.spaceXs
            // Only the unstaged side has a piece to throw away: on the staged side the button beside this one puts the
            // hunk back where it can be.
            //
            // Held (デザイン規約 §長押し): the button sits in the hunk's own heading, so what it takes is the
            // thing it is standing on, and a bar coming down over the diff to say so is machinery a hunk does not need.
            //
            // Shaped like the held row of a right-click menu: this one sits in a
            // line of other words, and a frame around one word in a heading reads
            // as a box that has come loose. The mark says it is held, and the hold fills the words' own ground edge to
            // edge.
            ActionButton {
                id: discardHunkButton
                visible: !diffRow.staged
                text: qsTr("Discard hunk")
                font.pixelSize: Theme.fontSm
                // Asleep until the pointer is on this heading (デザイン規約 §diff の中のステージ). The mark is what still says this
                // one is held — the colour is saying something else.
                tone: diffRow.underPointer ? Theme.danger : Theme.textSecondary
                holdTone: Theme.danger
                holdMs: Metrics.holdMs
                enabled: !diffRow.busy
                // Which hunk of which reading (`HoldDriver.premise`): the pane re-reads itself behind every write, and
                // a delegate re-used across that read would discard whatever it was pointed at when the fill ran out.
                premise: diffRow.rowsGen + ":" + diffRow.hunk
                onHeld: diffRow.discardRequested(diffRow.hunk)
            }
            // The same pair of colours the file rows put on their own `+` and `−`: staging is the green half of the
            // gesture and unstaging the red one, and the heading names them in the list's own
            // voice — but it says it at the volume of a heading that is not being pointed at. At rest both words
            // wear `textSecondary`, which is the colour the `@@` beside them already has, so the whole heading reads
            // as one grey line until the pointer arrives (デザイン規約 §diff の中のステージ). A file diff carries 2 hunks at the
            // middle and 8 at the ninetieth percentile — measured over 52 files — so leaving them all lit puts 2 to 4
            // coloured words on screen against the header's one.
            ActionButton {
                text: diffRow.staged ? qsTr("Unstage hunk") : qsTr("Stage hunk")
                font.pixelSize: Theme.fontSm
                tone: !diffRow.underPointer ? Theme.textSecondary
                      : diffRow.staged ? Theme.diffRemovedFg : Theme.diffAddedFg
                enabled: !diffRow.busy
                onActivated: diffRow.stageHunkRequested(diffRow.hunk)
            }
        }
    }
    // Throwing away starts at the hunk (デザイン規約 §その他の操作). A line can be staged on its own because staging loses
    // nothing — the line stays on disk either way — but a bare `×`
    // has no words to say what it takes, and a control that must be held has to say it.
}
