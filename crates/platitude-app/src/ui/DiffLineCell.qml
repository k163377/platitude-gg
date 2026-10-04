pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// One line of the diff: the text, its two washes (what changed, what is selected) and the no-newline mark. One per
// row in one column, one per side in two (`DiffRowDelegate`). The cell clips and the line travels sideways under it;
// only a heading elides (デザイン規約 §diff を横へ送る).
Item {
    id: cell

    /// What this cell draws, as `Text.StyledText` reads it — every line, coloured or not (`encode::DiffRow`).
    required property string text
    /// `hunk` / `ctx` / `add` / `del` / `meta` / `commit` — what colour the words are, and whether they are a
    /// line of the file at all.
    required property string kind
    /// What changed inside this line, as runs of layout places (`[{ from, len }, …]`, `encode::DiffRow.emph`).
    required property var emph
    /// The reader's selection on this line (`DiffModel.sel` — `{whole, runs}`), empty on every line `Copy` would not
    /// take: the wash is the answer (デザイン規約 §diff の中身をコピーする).
    required property var sel
    /// One of git's conflict fences: the words drop their voice (デザイン規約 §シンタックスハイライト).
    required property bool fence
    /// This line ends the file without a newline (デザイン規約 §行末の改行が無いこと).
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
    function settleEmph() {
        cell.emphRects = cell.ruler.rectsOf(cell.text, cell.codeBold, cell.emph)
    }
    function settleSel() {
        cell.selRects = !cell.sel || cell.sel.whole ? [] : cell.ruler.rectsOf(cell.text, cell.codeBold, cell.sel.runs)
    }
    function settleWashes() {
        cell.settleEmph()
        cell.settleSel()
    }
    // A delegate reused for another row (`reuseItems`) arrives through these same handlers.
    onTextChanged: cell.settleWashes()
    onCodeBoldChanged: cell.settleWashes()
    onEmphChanged: cell.settleEmph()
    onSelChanged: cell.settleSel()
    Component.onCompleted: cell.settleWashes()

    // The stronger wash under what changed (デザイン規約 §シンタックスハイライト), under the Label and sent with it.
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
    // The selection, over the emphasis and under the text, so a selected line keeps its syntax colours.
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
    Label {
        id: codeLine
        x: cell.banded ? 0 : -cell.codeX
        width: cell.banded ? cell.width : implicitWidth
        elide: cell.banded ? Text.ElideRight : Text.ElideNone
        height: cell.height
        verticalAlignment: Text.AlignVCenter
        // A heading starts at the row's left edge, not past the empty number columns (デザイン規約 §diff の中のステージ).
        leftPadding: cell.banded ? Theme.spaceXs : 0
        text: cell.text
        // Every line is markup, coloured or not (`markup::styled`), so the format never changes under a row.
        textFormat: Text.StyledText
        font.family: Theme.monoFamily
        font.bold: cell.codeBold
        // `fontCode` for source (デザイン規約 §タイポグラフィ); a heading is smaller, or its `@@` line runs under the
        // row's two buttons.
        font.pixelSize: cell.banded ? Theme.fontSm : Theme.fontCode
        // The whole colour of a line the theme said nothing about; a changed line stays `textPrimary` and a fence
        // drops to muted (デザイン規約 §シンタックスハイライト).
        color: cell.fence ? Theme.textMuted
               : cell.kind === "hunk" ? Theme.diffHunkHeaderFg
               : cell.kind === "commit" ? Theme.textSecondary
               : cell.kind === "meta" ? Theme.textMuted
               : Theme.textPrimary
    }
    // git's `\ No newline at end of file` as a mark at the line's end (デザイン規約 §行末の改行が無いこと). Built only
    // on the line that has it (rules-refs/app-ui.md「行のデリゲートが見せない部品は消す」).
    Loader {
        id: noEolMark
        active: cell.noNewline
        // Seated by its ink, not its square (デザイン規約 §余白).
        x: noEolMark.item
           ? codeLine.x + codeLine.implicitWidth + Theme.spaceXs - (noEolMark.item.width - noEolMark.item.inkWidth) / 2
           : 0
        anchors.verticalCenter: parent.verticalCenter
        sourceComponent: NavIcon {
            kind: "no-entry"
            width: Theme.iconSm
            height: Theme.iconSm
            stroke: Metrics.iconStroke * Theme.iconSm / Theme.iconMd
            tint: Theme.danger
            /// On its own line, never over the lines either side; rightward, past the line's end and off its code
            /// (`SharedToolTip.tipRowSide`).
            readonly property string tipRowSide: "right"
            // Hover only: the press over the code belongs to `DiffTextSelect`.
            ToolTip.visible: noEolHover.containsMouse
            ToolTip.delay: Metrics.tipDelayMs
            ToolTip.text: qsTr("No newline at end of file")
            MouseArea {
                id: noEolHover
                anchors.fill: parent
                acceptedButtons: Qt.NoButton
                hoverEnabled: true
            }
        }
    }
}
