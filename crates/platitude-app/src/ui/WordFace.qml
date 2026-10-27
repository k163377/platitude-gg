pragma Singleton

import QtQuick
import platitude.ui

// The lines of the window's face that a mark set beside words is drawn to (デザイン規約 §アイコンの席): the capitals'
// height, which every set mark is drawn to, and the face's own `+` — where a sign stands and the room it takes. Read
// off the face at run time, so each OS's face answers for itself; in shares of the pixel size, measured large, where
// hinting does not round them.
QtObject {
    id: face

    /// The face's `H`, large.
    readonly property TextMetrics capitalInk: TextMetrics {
        font.family: Theme.uiFamily
        font.pixelSize: 1000
        text: "H"
    }
    /// The capitals' height: an `H`'s ink. Not the face's own table (`capitalHeight`): Yu Gothic UI's sets the capitals
    /// a tenth taller than the `H` it draws (0.772 of an em against 0.700); Noto Sans JP's and its `H` agree.
    readonly property real capital: face.capitalInk.tightBoundingRect.height / 1000
    /// The face's `+`, measured by a `TextMetrics`: its answers are properties, which the bindings below follow when
    /// the face changes — a `FontMetrics` method answers once (rules/app-ui.md「メソッドはバインディングが依存を取らない」).
    readonly property TextMetrics plusInk: TextMetrics {
        font.family: Theme.uiFamily
        font.pixelSize: 1000
        text: "+"
    }
    /// The face's `+`: `y` its top above the baseline (negative), `width` and `height` its ink. Qt's tight box never
    /// ends above the baseline (`QFontEngine::tightBoundingBox` starts its bottom at 0, every engine and OS), so a
    /// glyph standing clear of it — the plus — comes back with its foot at the baseline: the top and width are the
    /// face's, and the plus is taken as tall as it is wide (`tst_setmark.qml`).
    readonly property var plus: {
        const rect = face.share(face.plusInk.tightBoundingRect)
        return { y: rect.y, width: rect.width, height: rect.width }
    }
    /// The air the face's `+` holds either side of its ink: a sign's room in a run of words.
    readonly property real signSide: (face.plusInk.advanceWidth / 1000 - face.plus.width) / 2
    /// The code face large, in the two weights a name is set in, for where a glyph's ink ends and starts (`groundEnd`,
    /// `nameAir`): a glyph's box comes back rounded out to whole pixels where the face is FreeType's (offscreen, and
    /// Linux — `QFontEngineFT::boundingBox`), up to a pixel past the ink at a name's size; here the rounding is a
    /// thousandth.
    readonly property FontMetrics codeLarge: FontMetrics {
        font.family: Theme.monoFamily
        font.pixelSize: 1000
    }
    readonly property FontMetrics codeLargeStrong: FontMetrics {
        font.family: Theme.monoFamily
        font.pixelSize: 1000
        font.weight: Theme.fontWeightStrong
    }
    /// The code face's letters and digits and the `/` that cuts a name, large: how far a name's ink reaches over its
    /// baseline and under it (`codeInkShare`). `f`, `j` and `/` the tallest and `y`, `g` and `j` the deepest, by face.
    readonly property TextMetrics codeInk: TextMetrics {
        font.family: Theme.monoFamily
        font.pixelSize: 1000
        text: "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789/"
    }
    /// How far above the baseline the middle of that ink stands, in shares of the pixel size. Read once here: the
    /// metrics lay the line out again at every read, and every code ground asks.
    readonly property real codeInkShare: {
        const ink = face.codeInk.tightBoundingRect
        return -(ink.y + ink.height / 2) / 1000
    }
    /// The window's face large in the same two weights, for where a name set in it starts its ink (`nameAir`).
    readonly property FontMetrics wordLarge: FontMetrics {
        font.family: Theme.uiFamily
        font.pixelSize: 1000
    }
    readonly property FontMetrics wordLargeStrong: FontMetrics {
        font.family: Theme.uiFamily
        font.pixelSize: 1000
        font.weight: Theme.fontWeightStrong
    }

    /// Where a code ground starts in front of a word or a name with no mark (デザイン規約 §git 用語のコード表記): half a
    /// gap before its first glyph's `head` — the air it keeps after the last glyph too (`groundEnd`). A name behind its
    /// mark starts its ground at the head of the mark's seat instead: the seat holds the mark's air. Not rounded: a
    /// ground drawn to a whole pixel moves each end by up to half a pixel, a different amount name by name.
    function groundStart(head) {
        return head - Theme.spaceXs / 2
    }
    /// …and where it ends: half a gap past the ink of the name's last glyph. `advanceEnd` is where the name's
    /// advance ends, laid out in `code`, the code face at the drawn size; the glyph's ink is the outline's, read off
    /// the face large (`codeLarge`). Its font is read here so a binding calling this follows it (rules/app-ui.md
    /// 「メソッドはバインディングが依存を取らない」).
    function groundEnd(code, name, advanceEnd) {
        const font_ = code.font
        const last = name.slice(-1)
        if (last === "")
            return advanceEnd + Theme.spaceXs / 2
        const large = code.font.weight === Theme.fontWeightStrong ? face.codeLargeStrong : face.codeLarge
        const ink = large.tightBoundingRect(last)
        const inkEnd = (ink.x + ink.width) * code.font.pixelSize / 1000
        return advanceEnd - (code.advanceWidth(last) - inkEnd) + Theme.spaceXs / 2
    }
    /// The air `name`'s first glyph holds ahead of its ink, set in the font of `words` (what draws the name, or metrics
    /// in its font): the outline read off the face large, as `groundEnd` reads the last glyph's — the code face's for
    /// a name in the code face, the window's for any other, the faces the app sets names in. A mark seated in front of
    /// the name stands in the middle between its seat's head and that ink (`NavIcon.nameAir`). Its font is read here so
    /// a binding calling this follows it.
    function nameAir(words, name) {
        const font_ = words.font
        const first = name.charAt(0)
        if (first === "")
            return 0
        const strong = words.font.weight === Theme.fontWeightStrong
        const large = words.font.family === Theme.monoFamily ? (strong ? face.codeLargeStrong : face.codeLarge)
                                                             : (strong ? face.wordLargeStrong : face.wordLarge)
        return large.tightBoundingRect(first).x * words.font.pixelSize / 1000
    }

    function share(rect) {
        return { y: rect.y / 1000, width: rect.width / 1000, height: rect.height / 1000 }
    }
    /// The capitals' height at `size` as the face draws them, to whole pixels and rounded up: hinting stands their
    /// edges on whole pixels, mostly a pixel above `capital` at a name's size, and how far depends on the weight and
    /// on where the baseline falls — no measure taken off the face foretells it. A mark drawn to the height unrounded
    /// stands no taller than the letters beside it; rounded up, it stands past them at every size a name is set in
    /// (`tst_setmark.qml`).
    function capitalsAt(size) {
        return Math.ceil(face.capital * size - 1e-6)
    }
    /// How far above the baseline the capitals' middle stands, at `size` — where a picture beside words (a face)
    /// sets its own middle.
    function capitalMiddle(size) {
        return face.capital * size / 2
    }
    /// How far above the baseline the middle of the code face's ink stands, at `size` — where a code ground sets its
    /// own (`CodeGround`).
    function codeInkMiddle(size) {
        return face.codeInkShare * size
    }
    /// How far above the baseline the face's `+` has its middle, at `size` — where a sign drawn beside words, a dot
    /// included, sets its own (`DotMark.middle`).
    function signMiddle(size) {
        return -(face.plus.y + face.plus.height / 2) * size
    }
    /// How far a box centred where `words`' box is centred moves its middle down to the capitals' middle.
    function capitalDrop(words) {
        return face.capitalDropOf(words.baselineOffset, words.font.pixelSize, words.height)
    }
    /// The same for words given by their baseline, pixel size and height (a field, which names its size itself).
    function capitalDropOf(base, size, height) {
        return Math.round(base - face.capitalMiddle(size) - height / 2)
    }
    /// How far above the baseline the middle of the capitals as the face draws them (`capitalsAt`) stands, at `size`:
    /// where a hold mark and a ring heading the words centre, as the marks among the words do (デザイン規約
    /// §アイコンの席「顔と回数押しの印」).
    function drawnCapitalMiddle(size) {
        return face.capitalsAt(size) / 2
    }
    /// How far a box centred where `words`' box is centred moves its middle down to `drawnCapitalMiddle`.
    function drawnCapitalDrop(words) {
        return Math.round(words.baselineOffset - face.drawnCapitalMiddle(words.font.pixelSize) - words.height / 2)
    }
    /// The same to the face's `+`'s middle: where a sign that turns about its own middle (the ring) stands.
    function signDrop(words) {
        return Math.round(words.baselineOffset - face.signMiddle(words.font.pixelSize) - words.height / 2)
    }
}
