import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The numbers a diff's rows are laid out from — line number, gutter, one mono column, the longest line — read off
// rulers that are never drawn. Whole lines only: where a character stands is the laid-out line's (`LineRuler`).
//
// An `Item`: the rulers are Labels, and a `QtObject` has nowhere to put a child (rules-refs/structure.md).
Item {
    id: metrics

    required property var diffModel
    /// Whether the rows carry a per-line mark — the seat between the two
    /// numbers is that mark's, and a step of air where there is none.
    required property bool partial
    /// Whether the rows are read as two columns (デザイン規約 §diff を 2 列で読む); one column unless told otherwise.
    property bool split: false

    /// How wide a line number is: the widest one this diff carries (デザイン規約 §レイアウト初期値). Measured with a
    /// never-drawn Label: `TextMetrics` reports a few pixels tighter than the Label the number is set in. Whole
    /// pixels — the columns and the code are laid out from this one number.
    readonly property int numberW: Math.ceil(numberMeasure.implicitWidth)
    Label {
        id: numberMeasure
        visible: false
        text: metrics.diffModel.widestNo
        font.family: Theme.monoFamily
        font.pixelSize: Theme.fontSm
    }

    /// The mark's seat between the two numbers, and the gutter it sits in — per column: as two columns each holds
    /// one number and the seat after it (デザイン規約 §diff を 2 列で読む).
    readonly property int seatW: metrics.partial ? Theme.iconMd + 2 * Theme.borderWidth : Theme.spaceXs
    readonly property int gutterW: metrics.split
                                   ? Theme.spaceXs + metrics.numberW + metrics.seatW
                                   : 2 * (Theme.spaceXs + metrics.numberW) + metrics.seatW
    /// How wide the longest of the lines the model picked is drawn — **measured** (not columns times `charW`, see
    /// there), and **0 while none of them has been laid out** ("nothing measured yet"). A head start the rows correct
    /// as they are laid out (`DiffReach`).
    ///
    /// Pushed by `settleReach`: a binding takes a laid-out width once, before the font arrives (app-ui.md).
    property real codeW: 0

    /// Takes the widest of the rulers.
    function settleReach() {
        let reach = 0
        for (let i = 0; i < reachRulers.count; i++) {
            const ruler = reachRulers.itemAt(i)
            if (ruler)
                reach = Math.max(reach, ruler.implicitWidth)
        }
        metrics.codeW = reach
    }

    /// One never-drawn Label per line the reach could come from (`DiffModel.widestLines` — `{bold, line}`).
    Repeater {
        id: reachRulers
        model: metrics.diffModel.widestLines
        onCountChanged: metrics.settleReach()
        delegate: Label {
            required property var modelData

            visible: false
            text: modelData.line
            // The row's own format, family, size and weight: a ruler that reads the line any other way measures a
            // line this pane never draws. Left to itself a `Label` reads `AutoText` and guesses whether source is
            // markup.
            textFormat: Text.StyledText
            font.family: Theme.monoFamily
            font.pixelSize: Theme.fontCode
            font.bold: modelData.bold
            onImplicitWidthChanged: metrics.settleReach()
            Component.onCompleted: metrics.settleReach()
        }
    }
    /// One measured column of the mono font — only for the wash on an empty line (`DiffRowDelegate`). Where a line's
    /// characters are drawn is that line's layout (`LineRuler`): a column is not a width — the fallback a Latin-only
    /// mono family hands a wide glyph to is not monospaced, and a combining mark takes a column and no room.
    readonly property real charW: charMeasure.implicitWidth / charMeasure.text.length
    Label {
        id: charMeasure
        visible: false
        // Ten of them, so a single advance's rounding does not multiply up.
        text: "0000000000"
        font.family: Theme.monoFamily
        font.pixelSize: Theme.fontCode
    }
}
