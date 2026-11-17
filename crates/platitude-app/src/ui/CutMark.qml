import QtQuick
import platitude.ui
import platitude.ui.face

// The cut mark — the one every cut in the window draws (デザイン規約 §寸法「省略の印は描く」): three dots drawn in the
// place of a `…`, a character stood in for, not an icon (デザイン規約 §アイコンの席 の字の代替). Typed, the `…` is the
// font's and the locale's: Noto Sans JP / Noto Sans CJK JP set it centred in a Japanese locale, and outside one centred
// after kana, kanji or digits and on the baseline after Latin letters, while Yu Gothic UI, Cascadia Mono and Menlo set
// it on the baseline always — so one window showed both (user decision — ask before changing: no cut mark is the
// font's). A `…` in words a sentence or a value spells stays a character; this is the mark a cut leaves.
//
// Drawn in the room of the glyph it stands for (user decision — ask before changing: never wider than that glyph, and
// no air beside it the glyph would not leave): as wide as the words' face's own `…` laid out alone at the words' size
// (`ruler`) — the room an elide keeps for it, so a cut keeps the letters the typed glyph kept — its dots at an even
// third of that room, centred in it, on whole pixels of the holder (`WordFace.cutDotX`), standing on the baseline (the
// English text's mark). Laid over a clear `…` in rich text (`Words.codeMarkup`), it takes that glyph's face and so its
// width. `base` is where that baseline is in this item's box — its foot by default, so a holder sets the item's
// baseline (`y = baseline - baselineOffset`, `anchors.baseline`); a holder that centres a line box instead gives the
// item that box and its baseline.
Item {
    id: mark

    /// The font of the words the mark cuts: their face, size, weight and tracking.
    property font font
    property color color: Theme.textPrimary
    /// The baseline, in this item's y.
    property real base: mark.implicitHeight
    /// The words' face's own `…`, laid out alone: its advance is the mark's room, the dots spread in it. Nothing writes
    /// it but the font, so a binding reads it (`WordFace.groundEndAtMark`).
    readonly property TextMetrics ruler: TextMetrics {
        font: mark.font
        text: WordFace.cutGlyph
    }

    implicitWidth: mark.ruler.advanceWidth
    implicitHeight: WordFace.cutDotSize
    baselineOffset: mark.base

    Rectangle {
        x: WordFace.cutDotX(mark.ruler, 0, mark.x) - mark.x
        y: Math.round(mark.base) - height
        width: WordFace.cutDotSize
        height: width
        radius: width / 2
        color: mark.color
    }
    Rectangle {
        x: WordFace.cutDotX(mark.ruler, 1, mark.x) - mark.x
        y: Math.round(mark.base) - height
        width: WordFace.cutDotSize
        height: width
        radius: width / 2
        color: mark.color
    }
    Rectangle {
        x: WordFace.cutDotX(mark.ruler, 2, mark.x) - mark.x
        y: Math.round(mark.base) - height
        width: WordFace.cutDotSize
        height: width
        radius: width / 2
        color: mark.color
    }
}
