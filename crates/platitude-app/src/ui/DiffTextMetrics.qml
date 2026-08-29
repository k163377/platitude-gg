import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The numbers a diff's rows are laid out from: how wide a line number is, how
// wide the gutter that holds two of them is, how wide one column of the mono
// font comes out, and how far a wide glyph runs past the two columns it counts
// as. Three rulers that are never drawn, and the six numbers read off them.
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
    /// How wide the longest line of this diff is drawn. The model counts the columns (`encode::widest_columns`); one
    /// measured character turns them into pixels, and the font is mono so that one character speaks for all of them.
    /// Measured with a never-drawn Label: a metric read off a method is taken once, before the font arrives (app-ui.md).
    readonly property real codeW: metrics.diffModel.widestColumns * metrics.charW + Theme.spaceSm
    /// One measured column of the mono font. The divisor is however many characters `charMeasure` holds, so the two
    /// cannot drift apart; everything column-addressed — the code width above, the emphasis wash in the rows —
    /// multiplies this one number. Measured at regular weight: changed rows draw bold, which JetBrains Mono advances
    /// identically — a mono family whose bold face advances differently would drift the wash, so a swap of
    /// `Theme.monoFamily` re-checks that.
    readonly property real charW: charMeasure.implicitWidth / charMeasure.text.length
    Label {
        id: charMeasure
        visible: false
        // Ten of them, so a single advance's rounding does not multiply up over a two-hundred-column line.
        text: "0000000000"
        font.family: Theme.monoFamily
        font.pixelSize: Theme.fontCode
    }
    /// What a wide glyph costs on top of the two columns `encode::columns::step_of` counts it as. Zero wherever the
    /// mono family carries the wide glyphs itself at two of its own advances; where it is Latin-only they arrive from a
    /// fallback that advances one em instead (Windows, Cascadia Mono at `fontCode`: charW 8px against a wide advance of
    /// 13px, so −3px a glyph, which slid the wash that far right of the characters it names). Measured rather than
    /// assumed, because it is a property of whichever fallback this OS hands the glyphs to.
    ///
    /// **And measured only where a wide glyph is on screen** (`DiffModel.hasWide`, from `encode::has_wide`). Setting
    /// one is what loads that fallback, and the font is 52.6MB of working set — a third of what the whole app is
    /// allowed — which every window used to pay, one with no repository open included (2026-08-29 実測: 198.7–199.4MB
    /// against 146.3–146.7). Zero is not a stand-in for the unmeasured number: every place this is read multiplies it
    /// by a count of wide glyphs, so a diff that has none never asks what one would have cost.
    readonly property real wideDelta: wideRuler.item
        ? wideRuler.item.implicitWidth / wideRuler.item.text.length - 2 * metrics.charW
        : 0
    Loader {
        id: wideRuler
        active: metrics.diffModel.hasWide
        sourceComponent: Label {
            visible: false
            // Ten U+65E5, for the same reason charMeasure holds ten. Built from the code point rather than written as
            // the glyph: this is a ruler and not a word, and a line of Japanese sitting in a `text:` reads like the
            // hardcoded wording the rules forbid (CLAUDE.md 絶対制約).
            text: String.fromCharCode(0x65e5).repeat(10)
            font.family: Theme.monoFamily
            font.pixelSize: Theme.fontCode
        }
    }
}
