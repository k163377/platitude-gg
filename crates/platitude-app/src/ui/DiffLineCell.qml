pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The room one line of the diff is read in: the line itself, the two washes under it — what changed inside the
// line, and what the reader has selected of it — and the mark at its end where the file ends without a newline.
// One per row while the diff is read as one column, one per side while it is read as two (`DiffRowDelegate`).
//
// It is cut to the room it is given, and the line inside it is not: the text is as wide as it is and travels under
// this window, so a long line is read by sending it (デザイン規約 §diff を横へ送る). A heading does not travel: it is
// the pane's own words about the rows below, and elides into the room instead.
Item {
    id: cell

    /// What this cell draws, as `Text.StyledText` reads it — every line, coloured or not (`encode::DiffRow`).
    required property string text
    /// `hunk` / `ctx` / `add` / `del` / `meta` / `commit` — what colour the words are, and whether they are a
    /// line of the file at all.
    required property string kind
    /// Where what changed inside this line falls in it — runs of places a layout counts (`[{ from, len }, …]`),
    /// empty for nothing (`encode::DiffRow.emph`). Drawn as the stronger wash under the text; the quiet parts keep
    /// the line's own background (デザイン規約 §シンタックスハイライト).
    required property var emph
    /// Where the reader's own selection falls on this line (`DiffModel.sel` — `{whole, runs}`): the line taken end
    /// to end, or the same runs `emph` carries. Nothing on every line the plain `Copy` does not take —
    /// **the wash is the answer** (デザイン規約 §diff の中身をコピーする).
    required property var sel
    /// One of git's conflict fences: the words drop their voice (デザイン規約 §シンタックスハイライト).
    required property bool fence
    /// This line ends the file without a newline: git's note, said as a mark at the end of the line it was about
    /// (デザイン規約 §行末の改行が無いこと).
    required property bool noNewline
    /// The weight this line is set in — a changed line is bold (規約 §シンタックスハイライト). Decided by the row,
    /// because **the ruler has to be set in the weight the line is drawn in**: a family whose bold face advances
    /// differently would otherwise put every wash and every press out by the difference.
    required property bool codeBold
    /// How far the file's own text has been sent sideways (`DiffCodeScroll.offset`).
    required property real codeX
    /// One measured column of the mono font (`DiffTextMetrics.charW`): how wide the wash on an **empty** line is,
    /// the only place a width without characters behind it is needed.
    required property real charW
    /// Where the two washes go: this line, laid out, asked where each run of it is drawn (`LineRuler`). The same
    /// ruler the hand over the rows reads a press against (`DiffTextSelect`).
    required property var ruler
    /// A row that names something rather than being a line of the file: the hunk's own heading, or the commit
    /// each block is of (デザイン規約 §複数のコミットを選ぶ). It stands at the cell's own left edge and elides.
    required property bool banded

    /// How far the line is drawn — where the blank right of it begins, and what the row files under
    /// `rowDrawn` (`DiffReach`). Read off the Label: a `Text` answers with every glyph at the family's own advance
    /// until the fallback carrying the wide ones is resolved, so the first number out of it is not the last.
    readonly property real inkWidth: codeLine.implicitWidth
    /// Where the line's right edge stands in `frame`'s coordinates — **asked of the item where it was placed**
    /// (`DiffPane.codeInkRight`). A heading does not travel, so it answers 0.
    function inkRightIn(frame) {
        return cell.banded ? 0 : codeLine.mapToItem(frame, codeLine.implicitWidth, 0).x
    }

    clip: true

    /// Where this line's two washes are drawn, taken from the line laid out — `[{ x, w }, …]` in the code's own
    /// coordinates, before the send (`LineRuler.rectsOf`). A line taken end to end says so (`sel.whole`) and needs
    /// no ruler at all; a heading has neither wash.
    ///
    /// **Pushed** (the rule `DiffTextMetrics.codeW` is written under). Asking the ruler means putting this line on
    /// it, and a binding that writes while it is being evaluated is a binding loop — Qt says so by name and then
    /// holds whatever it had, which is a wash left on the row before it. The four properties the answer is made of
    /// say when to ask instead.
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
    // The line itself and the weight it is set in move both washes; each run moves its own. A delegate handed to
    // another row (`reuseItems`) arrives through these same three.
    onTextChanged: cell.settleWashes()
    onCodeBoldChanged: cell.settleWashes()
    onEmphChanged: cell.settleEmph()
    onSelChanged: cell.settleSel()
    Component.onCompleted: cell.settleWashes()

    // The stronger wash under what actually changed inside the line (`emph`), while the quiet parts keep the line's
    // own background — the strong/weak split is these rectangles (デザイン規約 §シンタックスハイライト). Under the
    // Label, travelling with the same send.
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
    // The reader's own selection, over the emphasis and under the text — the same place the line's other wash
    // stands, and for the same reason: the strong/weak split of a diff is rectangles
    // (デザイン規約 §シンタックスハイライト), so a selected line keeps its syntax colours.
    Repeater {
        model: cell.sel && cell.sel.whole ? [{ "whole": true }] : cell.selRects
        delegate: Rectangle {
            required property var modelData
            readonly property bool whole: modelData.whole === true
            x: -cell.codeX + (whole ? 0 : modelData.x)
            // A line taken whole is washed to its own end — the ink it was drawn in, which is the one number no
            // walk of it can produce — and a column at least: an empty line is a line the copy takes, and a wash
            // of no width would leave a hole in the middle of a selection.
            width: whole ? Math.max(codeLine.implicitWidth, cell.charW) : modelData.w
            height: parent.height
            color: Theme.bgSelected
        }
    }
    Label {
        id: codeLine
        // A hunk heading does not travel: it is the pane's own words about the rows below, and words that slid off
        // the left while the code was read would take with them the only thing saying which hunk this is. It gives
        // up the right of the row to the two buttons and elides into what is left.
        x: cell.banded ? 0 : -cell.codeX
        width: cell.banded ? cell.width : implicitWidth
        elide: cell.banded ? Text.ElideRight : Text.ElideNone
        height: cell.height
        verticalAlignment: Text.AlignVCenter
        // A hunk heading starts at the row's own left edge: the two columns beside it are empty
        // — a heading has no line to number — so indenting it by them lines the pane's own words up with the
        // file's, behind a gutter that says nothing. One `spaceXs`, the same gap everything else in the gutter
        // stands at.
        leftPadding: cell.banded ? Theme.spaceXs : 0
        text: cell.text
        // Every line is markup, coloured or not (`markup::styled`), so a line of source full of `<T>` arrives
        // escaped and this never changes under a row (規約 §シンタックスハイライト).
        textFormat: Text.StyledText
        font.family: Theme.monoFamily
        // The weight this line is set in, said once for the Label and for the ruler both washes are placed with.
        font.bold: cell.codeBold
        // The size an editor puts source at — `fontCode`, matched to IntelliJ's default (デザイン規約 §タイポグラフィ).
        //
        // The hunk heading is smaller still: it is the pane's own words, and at the file's size its `@@` line runs
        // under the two words sitting at the right of the same row.
        font.pixelSize: cell.banded ? Theme.fontSm : Theme.fontCode
        // Where the theme said nothing — an uncoloured language, a row past the lexer's budget, the moment before
        // the colours land — this is still the whole of the line's colour. A changed line reads in the window's own
        // words (規約 §シンタックスハイライト): the wash and the weight already name it, and green-on-green said the
        // same thing twice while reading worse.
        //
        // A fence drops its voice: `<<<<<<<` is git's scaffolding round the two sides, and painting it the
        // added-line green puts the loudest thing in the pane on the part nobody is reading
        // (デザイン規約 §シンタックスハイライト). It keeps its background — it really is in the file.
        color: cell.fence ? Theme.textMuted
               : cell.kind === "hunk" ? Theme.diffHunkHeaderFg
               // The band's own voice, the one every heading in this window speaks in (`PaneHeader`).
               : cell.kind === "commit" ? Theme.textSecondary
               : cell.kind === "meta" ? Theme.textMuted
               : Theme.textPrimary
    }
    // git's `\ No newline at end of file`, said where the thing it is about is: at the end of this line
    // (デザイン規約 §行末の改行が無いこと). As a row it stood between the removed line and the added one and parted the
    // pair the eye reads as one change; as a mark it travels with the text, so it is at the end of the line
    // whichever way the pane has been sent.
    //
    // **Built only on the line that has it.** A delegate is built per line on screen, and the mark is a canvas
    // with a hover and a tip of its own — on every other line it would be built and hidden, which is the heap
    // this pane is measured by (the rule the hunk tools follow; rules-refs/app-ui.md, the Loader rule).
    Loader {
        id: noEolMark
        active: cell.noNewline
        // Off the ink: the mark's square holds air past its ring, and a whole `spaceXs` on top of
        // that would leave the last character further from this than any other pair in the row (§余白).
        x: noEolMark.item
           ? codeLine.x + codeLine.implicitWidth + Theme.spaceXs - (noEolMark.item.width - noEolMark.item.inkWidth) / 2
           : 0
        anchors.verticalCenter: parent.verticalCenter
        sourceComponent: NavIcon {
            kind: "no-entry"
            // A mark that stands at the end of a line of words (デザイン規約 §寸法), with the stroke dropped to that
            // step so it carries the weight of the letters beside it.
            width: Theme.iconSm
            height: Theme.iconSm
            stroke: Metrics.iconStroke * Theme.iconSm / Theme.iconMd
            tint: Theme.danger
            // The words the note used to carry, kept where a mark can still hand them over. It takes no button:
            // the press over the code belongs to the hand picking text out of the rows (`DiffTextSelect`).
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
