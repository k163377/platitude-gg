pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// One line of the diff: the text, its two washes (what changed, what is selected) and the no-newline mark. One per
// row in one column, one per side in two (`DiffRowDelegate`). The cell clips and the line travels sideways under it:
// a line runs to its end and is sent to be read — a `…` for its tail would pass "cannot be read" off as "read to the
// end". Only a heading elides (デザイン規約 §diff を横へ送る): it is the pane's own words, giving its right edge to its
// two buttons. **The line's own `x` moves, never the list's `contentX`**: the
// `ListView` keeps its own `contentWidth`, and the two would fight over it.
Item {
    id: cell

    /// What this cell draws, as `Text.StyledText` reads it — every line, coloured or not (`encode::DiffRow`).
    required property string text
    /// `hunk` / `ctx` / `add` / `del` / `meta` / `commit` / `mode` — what colour the words are, and whether they are
    /// a line of the file at all.
    required property string kind
    /// What changed inside this line, as runs of layout places (`[{ from, len }, …]`, `encode::DiffRow.emph`).
    required property var emph
    /// The reader's selection on this line (`DiffModel.sel` — `{whole, runs}`), empty on every line `Copy` would not
    /// take: the wash is the answer (デザイン規約 §diff の中身をコピーする) — removed lines and hunk headings stay plain.
    /// Not a faint wash on removed lines to say "in the range": over the line's ground and the syntax emphasis it would
    /// be a third tier answering the one question whether a line is taken.
    required property var sel
    /// The find bar's hits on this line, and the one it stands on, as runs over the drawn text (`LineMarks`).
    required property var found
    required property var foundNow
    /// One of git's conflict fences (`<<<<<<<` / `|||||||` / `=======` / `>>>>>>>`), the scaffolding git stands round
    /// the two sides: the words drop to `textMuted` — in the added line's colours, the part nobody reads would be the
    /// loudest thing in the pane. The ground stays the added line's (`DiffRowDelegate.groundOf`): the line is really
    /// in the working-tree file, and staged as it is it goes into the commit. Not bold (`DiffRowDelegate.codeBold`).
    required property bool fence
    /// This line ends the file without a newline (the mark below).
    required property bool noNewline
    /// The weight this line is set in, decided by the row: the ruler must use the same weight, or a bold face that
    /// advances differently puts every wash and press out.
    required property bool codeBold
    /// How far the file's own text has been sent sideways (`DiffCodeScroll.offset`).
    required property real codeX
    /// One measured mono column (`DiffTextMetrics.charW`) — only the wash on an empty line needs it.
    required property real charW
    /// Places the washes by laying this line out (`LineRuler`) — the same ruler presses are read against
    /// (`DiffTextSelect`).
    required property var ruler
    /// A row that names something rather than being a line of the file: the hunk's heading, or the commit each block
    /// is of (デザイン規約 §複数のコミットを選ぶ). It stands at the cell's left edge and elides.
    required property bool banded

    /// How far the line is drawn — what the row files under `rowDrawn` (`DiffReach`). Read off the Label and
    /// followed: the first answer comes before the font fallback resolves.
    readonly property real inkWidth: codeLine.implicitWidth
    /// The line's right edge in `frame`'s coordinates, asked of the placed item (`DiffPane.codeInkRight`); 0 for a
    /// heading.
    function inkRightIn(frame) {
        return cell.banded ? 0 : codeLine.mapToItem(frame, codeLine.implicitWidth, 0).x
    }

    clip: true

    /// The two washes as `[{ x, w }, …]` in the code's coordinates, before the send (`LineRuler.rectsOf`).
    ///
    /// Pushed, not bound: asking the ruler writes this line onto it, and a binding that writes is a binding loop that
    /// keeps the previous row's wash. The handlers below say when to ask.
    property var emphRects: []
    property var selRects: []
    property var foundRects: []
    property var foundNowRects: []
    function settleEmph() {
        cell.emphRects = cell.ruler.rectsOf(cell.text, cell.codeBold, cell.emph)
    }
    function settleSel() {
        cell.selRects = !cell.sel || cell.sel.whole ? [] : cell.ruler.rectsOf(cell.text, cell.codeBold, cell.sel.runs)
    }
    function settleFound() {
        cell.foundRects = cell.ruler.rectsOf(cell.text, cell.codeBold, cell.found)
    }
    function settleFoundNow() {
        cell.foundNowRects = cell.ruler.rectsOf(cell.text, cell.codeBold, cell.foundNow)
    }
    function settleWashes() {
        cell.settleEmph()
        cell.settleSel()
        cell.settleFound()
        cell.settleFoundNow()
    }
    // A delegate reused for another row (`reuseItems`) arrives through these same handlers.
    onTextChanged: cell.settleWashes()
    onCodeBoldChanged: cell.settleWashes()
    onEmphChanged: cell.settleEmph()
    onSelChanged: cell.settleSel()
    onFoundChanged: cell.settleFound()
    onFoundNowChanged: cell.settleFoundNow()
    Component.onCompleted: cell.settleWashes()

    // The stronger wash under what changed (where: `platitude_core::intraline`), under the Label and sent with it.
    Repeater {
        model: cell.emphRects
        delegate: Rectangle {
            required property var modelData
            x: -cell.codeX + modelData.x
            width: modelData.w
            height: cell.height
            color: cell.kind === "add" ? Theme.diffAddedEmphBg : Theme.diffRemovedEmphBg
        }
    }
    // The selection, over the emphasis and under the text, so a selected line keeps its syntax colours (the comment
    // colour stands on `bgSelected` at 5.73:1, the band `diffAddedBg` / `diffRemovedBg` are in).
    Repeater {
        model: cell.sel && cell.sel.whole ? [{ "whole": true }] : cell.selRects
        delegate: Rectangle {
            required property var modelData
            readonly property bool whole: modelData.whole === true
            x: -cell.codeX + (whole ? 0 : modelData.x)
            // A whole line is washed to its ink, and at least one column: an empty line is copied too, and a
            // zero-width wash would leave a hole in the selection.
            width: whole ? Math.max(codeLine.implicitWidth, cell.charW) : modelData.w
            height: parent.height
            color: Theme.bgSelected
        }
    }
    // The find bar's hits, over the selection and under the text: every hit in the lighter wash, the one the bar
    // stands on in the denser (`DiffFindBar`).
    Repeater {
        model: cell.foundRects
        delegate: Rectangle {
            required property var modelData
            x: -cell.codeX + modelData.x
            width: modelData.w
            height: cell.height
            color: Theme.diffFindBg
        }
    }
    Repeater {
        model: cell.foundNowRects
        delegate: Rectangle {
            required property var modelData
            x: -cell.codeX + modelData.x
            width: modelData.w
            height: cell.height
            color: Theme.diffFindCurrentBg
        }
    }
    Label {
        id: codeLine
        x: cell.banded ? 0 : -cell.codeX
        width: cell.banded ? cell.width : implicitWidth
        elide: cell.banded ? Text.ElideRight : Text.ElideNone
        height: cell.height
        verticalAlignment: Text.AlignVCenter
        // A heading starts at the row's left edge (`spaceXs`), not past the number columns, which are empty on it (user
        // decision — ask before changing): indented, the pane's own words would line up with the file's lines across
        // a groove that says nothing. The file's lines start right of the columns.
        leftPadding: cell.banded ? Theme.spaceXs : 0
        // A commit's mode change under its band (one per commit when several are read stacked, so each mode reads as
        // its commit's) carries the two modes alone; the words are the pane's (`Words.modeLine`).
        text: cell.kind === "mode" ? Words.inked(Words.modeLineOf(cell.text)) : cell.text
        // Every line is markup, coloured or not (`markup::styled`), so the format never changes under a row.
        textFormat: Text.StyledText
        font.family: Theme.monoFamily
        font.bold: cell.codeBold
        // `fontCode` for source (デザイン規約 §タイポグラフィ); a heading is smaller, or its `@@` line runs under the
        // row's two buttons.
        font.pixelSize: cell.banded ? Theme.fontSm : Theme.fontCode
        // The whole colour of a line the theme said nothing about; a changed line stays `textPrimary` and a fence
        // drops to muted (`fence`). A row that is not a line of the file takes its kind's colour, never the syntax's.
        color: cell.fence ? Theme.textMuted
               : cell.kind === "hunk" ? Theme.diffHunkHeaderFg
               : cell.kind === "commit" ? Theme.textSecondary
               : cell.kind === "meta" ? Theme.textMuted
               : Theme.textPrimary
    }
    // git's `\ No newline at end of file` as a mark at the line's end (user decision — ask before changing; GitHub's
    // way). The note is git's about the line above it; as a row of its own it stands between the `-` and the `+` and
    // reads as about the added line below. A mark says which line by the line it ends, and the line's ground already
    // names the side. Where both sides lack the newline git prints a note per side, so two marks stand; on an
    // unchanged line the one note serves both. A combined diff never has one (git does not print it there).
    //
    // `no-entry` (`NavIcon`), a sign after the line's code. In `danger`, an exception kept to match other GUIs: the
    // colour rules alone give `textMuted` (outside the three states — nothing is stopped and little spreads,
    // 規約 §状態). It travels with the code (規約 §diff を横へ送る): it names the line's end, so it is out of view when
    // that is. The words are the hover's (`No newline at end of file`, 規約 §hover のツールチップ). Built only
    // on the line that has it (rules-refs/app-ui.md「行のデリゲートが見せない部品は消す」). Never selected or copied: the
    // note is folded onto its line (`encode::flatten_patches`) and is no row. Headless: `diff-file` on `--preset eol`'s
    // `unstaged:src/bare.kt` — old side without a final newline, new side CRLF, the one shape whose note stands between
    // the `-` and the `+`.
    Loader {
        id: noEolMark
        active: cell.noNewline
        // Its seat straight after the code, its box the code's line: the seat's own air is the gap.
        x: codeLine.x + codeLine.implicitWidth
        y: codeLine.y
        // A sign after the line's code (デザイン規約 §アイコンの席 の修飾): the capitals' scale, on the face's own `+`'s
        // middle, in the middle of its seat.
        sourceComponent: NavIcon {
            id: noEolSign
            kind: "no-entry"
            set: "sign"
            size: codeLine.font.pixelSize
            wordBase: codeLine.baselineOffset
            width: noEolSign.wordCell
            height: codeLine.height
            tint: Theme.danger
            readonly property TipAsk noEolTip: TipAsk {
                target: noEolSign
                // Hover only: the press over the code belongs to `DiffTextSelect`.
                asked: noEolHover.containsMouse
                words: qsTr("No newline at end of file")
                // On its own line, never over the lines either side; rightward, past the line's end and off its
                // code.
                rowSide: "right"
            }
            MouseArea {
                id: noEolHover
                anchors.fill: parent
                acceptedButtons: Qt.NoButton
                hoverEnabled: true
            }
        }
    }
}
