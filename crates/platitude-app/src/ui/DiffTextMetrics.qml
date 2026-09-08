import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The numbers a diff's rows are laid out from: how wide a line number is, how
// wide the gutter that holds two of them is, how wide one column of the mono
// font comes out, and how far the longest line is drawn. Rulers that are never
// drawn, and the numbers read off them.
//
// **Nothing here says where a character is.** That is a question for the line
// that was laid out (`DiffLineRuler`), and the rulers below hold whole lines
// or nothing to do with the text at all.
//
// An `Item`, not a `QtObject`: the rulers are Labels, and a `QtObject` has
// nowhere to put a child (rules-refs/structure.md).
Item {
    id: metrics

    required property var diffModel
    /// Whether the rows carry a per-line mark — the seat between the two
    /// numbers is that mark's, and a step of air where there is none.
    required property bool partial

    /// How wide a line number is: the widest one this diff carries (デザイン規約 §レイアウト初期値). The columns are cut to it
    /// rather than fixed, so the row reads `gap 140 gap 153 gap }`; a fixed column leaves the slack of the numbers it
    /// is *not* holding between the two numbers, while the code — which has none — sits one gap away.
    ///
    /// Measured with a Label that is never drawn, the way `ActionButton` and `TopBar` measure: `TextMetrics` reports a
    /// few pixels tighter than the Label the number is actually set in. Whole pixels — two columns and the code's run
    /// are laid out from this one number.
    readonly property int numberW: Math.ceil(numberMeasure.implicitWidth)
    Label {
        id: numberMeasure
        visible: false
        text: metrics.diffModel.widestNo
        font.family: Theme.monoFamily
        font.pixelSize: Theme.fontSm
    }

    /// The room between the two numbers where a changed line puts its mark out, and the whole gutter that room sits in.
    /// Worked out once here: the rows lay themselves out from it, and the pane subtracts it to know how much shows.
    readonly property int seatW: metrics.partial ? Theme.iconMd + 2 * Theme.borderWidth : Theme.spaceXs
    readonly property int gutterW: 2 * (Theme.spaceXs + metrics.numberW) + metrics.seatW
    /// How wide the longest of the lines the model picked is drawn — **measured, not counted**, and **0 while none of
    /// them has been laid out**, which is not the same answer as a diff with nowhere to go: what reads this holds the
    /// reader's place through that gap rather than clamping it back to the left edge (`DiffReach`).
    ///
    /// A head start and not the whole answer: the pick is by column count and the rows correct it as they are laid
    /// out (`DiffReach`). The rulers below set each line the way a row sets it — same family, size, weight and text
    /// format, on the row's own markup — so what comes back is the row's width and not a second opinion about it.
    ///
    /// **A column is not a width, and no arithmetic makes it one.** A wide glyph counts as two columns and is drawn at
    /// whatever carries it — and the fallback a Latin-only mono family hands it to is not monospaced at all (measured
    /// on Windows at `fontCode`, against 8px for the font's own columns: `日` 13, `の` 11, `。` 9, `「` 7, an emoji
    /// 18). Columns times `charW` ran a diff of this repository's own documents nearly four screens past the end of
    /// its longest line; charging every wide glyph the one difference measured off `日` still left a screenful.
    ///
    /// Pushed by `settleReach` rather than bound: a width read off a laid-out item is a measurement, and a binding
    /// takes it once, before the font arrives (app-ui.md).
    property real codeW: 0

    /// Takes the widest of the rulers. Called by each of them as it settles, and by the Repeater as the set changes.
    function settleReach() {
        let reach = 0
        for (let i = 0; i < reachRulers.count; i++) {
            const ruler = reachRulers.itemAt(i)
            if (ruler)
                reach = Math.max(reach, ruler.implicitWidth)
        }
        metrics.codeW = reach
    }

    /// The packed records taken apart: `<length>:<bold><text>`, one after another, the length in the units this side
    /// counts a string in. **Cut out by that length rather than parted by a mark** — the text is a line of somebody's
    /// file, so there is no character it cannot hold.
    function reachRecords(packed) {
        const out = []
        let at = 0
        while (at < packed.length) {
            const cut = packed.indexOf(":", at)
            if (cut < 0)
                return out
            const from = cut + 2
            out.push({ bold: packed.charAt(cut + 1) === "1",
                       line: packed.substring(from, from + Number(packed.substring(at, cut))) })
            at = from + Number(packed.substring(at, cut))
        }
        return out
    }

    /// One never-drawn Label per line the reach could come from, in the font and the weight that line is drawn in —
    /// the same instrument the number column is measured with, for the same reason (`TabStrip.settleTitleCap`).
    Repeater {
        id: reachRulers
        model: metrics.reachRecords(metrics.diffModel.widestLines)
        onCountChanged: metrics.settleReach()
        delegate: Label {
            required property var modelData

            visible: false
            text: modelData.line
            // The row's own settings, all four of them: a ruler that read the same line in another format or another
            // weight is measuring a line this pane never draws. Left to itself a `Label` reads `AutoText` and decides
            // for itself whether a line of source is markup — which is the guess the rows were taken off (`DiffRow`).
            textFormat: Text.StyledText
            font.family: Theme.monoFamily
            font.pixelSize: Theme.fontCode
            // A changed line is set bold (`DiffRowDelegate`), and a bold face need not advance like its regular.
            font.bold: modelData.bold
            onImplicitWidthChanged: metrics.settleReach()
            Component.onCompleted: metrics.settleReach()
        }
    }
    /// One measured column of the mono font. The divisor is however many characters `charMeasure` holds, so the two
    /// cannot drift apart.
    ///
    /// **The rows draw nothing at it.** Where a line's characters are drawn is a question for that line's own layout
    /// (`DiffLineRuler`), because a column is not a width and no arithmetic makes it one — the fallback a Latin-only
    /// mono family hands a wide glyph to is not monospaced, and a combining mark takes a column and no room at all.
    /// What is left for one column to be is the width of a wash on an **empty** line (`DiffRowDelegate`): the one
    /// place a width is wanted where there are no characters to ask about.
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
