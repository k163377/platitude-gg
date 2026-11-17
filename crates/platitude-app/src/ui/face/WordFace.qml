pragma Singleton

import QtQuick
import platitude.ui

// The lines of the window's face that a mark set beside words is drawn to (デザイン規約 §アイコンの席): the capitals'
// height, which every set mark is drawn to, and the face's own `+` — where a sign stands and the room it takes. Read
// off the face at run time, so each OS's face answers for itself; in shares of the pixel size, measured large, where
// hinting does not round them. And the cut: where an elide put its `…`, the drawn cut mark's dots and the grounds that
// end on it (`CutMark`).
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
    /// Linux — `QFontEngineFT::boundingBox` takes FLOOR / CEIL: a `t` inked over 0.48..7.22 answers 0..8), up to a
    /// pixel past the ink at a name's size; here the rounding is a thousandth. A real Windows window's DirectWrite
    /// answers the design box. Offscreen also draws a ground's ends rounded out to whole pixels
    /// (`QSGSoftwareInternalRectangleNode::setRect`, `toAlignedRect`) where a real window antialiases the fraction, so
    /// a verify-ui / qmltest picture shows up to a pixel more air at either end: tests read a ground's ends off the
    /// item's geometry and the ink off the picture (`tst_codenames.drawnInkEnd`).
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
    /// The face's own `…`: what an elide puts in a cut, and the glyph the cut mark takes its room and its dots' reach
    /// from (`CutMark.ruler`) — measured, never drawn as a mark.
    readonly property string cutGlyph: "\u2026"
    /// The cut mark's dots (`CutMark`): the drawn middle dot's (`DotMark`), so the two marks drawn in a character's
    /// place are one dot.
    readonly property real cutDotSize: Theme.borderWidth * 2

    /// Where an elide put its `…` in `elided`, cut from `whole` — searched for, since a name may hold its own `…` (a
    /// stash is named by its message): the first `…` with a real head and tail of the whole around it; -1 for none.
    /// What a cut drawn in pieces splits the elide's answer at, to draw the cut mark in place of the face's glyph.
    function cutPoint(whole, elided) {
        for (let i = elided.indexOf("\u2026"); i >= 0; i = elided.indexOf("\u2026", i + 1)) {
            if (whole.startsWith(elided.substring(0, i)) && whole.endsWith(elided.substring(i + 1)))
                return i
        }
        return -1
    }
    /// Where the cut mark's dot `i` (0–2) starts (`CutMark`), set by `ruler` — the words' face's own `…` laid out alone
    /// (`CutMark.ruler`) — for a mark set at `at` in a holder on whole pixels, in the holder's x: the three at an even
    /// pitch of a third of that glyph's advance, the middle one at its middle, each on a whole pixel. The pitch is a
    /// dot and a pixel at least, so the dots stay apart: three take 8px, and a `…` narrower than that (a code face's
    /// at 12px, 7px) has them reach a pixel past its left edge. Off the advance alone: a face's report of the glyph's ink
    /// is rounded out, by a pixel on Noto Sans CJK JP, and dots set by it touch the letter before. Read off the ruler's
    /// properties, so a binding calling it follows the face.
    function cutDotX(ruler, i, at) {
        const advance = ruler.advanceWidth
        const pitch = Math.max(face.cutDotSize + 1, Math.round(advance / 3))
        return Math.floor(at + advance / 2 + (i - 1) * pitch - face.cutDotSize / 2)
    }

    /// Where a code ground starts in front of a word or a name with no mark (デザイン規約 §git 用語のコード表記): half a
    /// gap before its first glyph's `head` — the air it keeps after the last glyph too (`groundEnd`). A name behind its
    /// mark starts its ground at the head of the mark's seat instead, with no overhang (user decision — ask before
    /// changing): the seat is one letter of the run and holds the mark's air, and ground air laid over it widens the
    /// chip's left inside, the more the thinner the mark. Measured from the seat, not the mark's ink, so a chip's outer
    /// edge lines up whatever mark it carries — row heads row by row, a column's chips down the column: the mark stands
    /// midway between the seat's head and the name's first ink, so ground-to-mark equals mark-to-name; from the ink,
    /// each mark's leftover seat would move the start. Not rounded: a ground drawn to a whole pixel moves each end by
    /// up to half a pixel, a different amount name by name.
    function groundStart(head) {
        return head - Theme.spaceXs / 2
    }
    /// …and where it ends: half a gap past the ink of the name's last glyph (user decision) — from the advance it would
    /// change with every last letter; from the ink it never passes half a gap beyond the advance, as a command's chip,
    /// and never reaches the next word. `advanceEnd` is where the name's
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
    /// …and where it ends behind a name cut at its end, whose last glyph is the cut mark (`CutMark`), set at `markX` by
    /// `ruler` (`CutMark.ruler`, the name's face): half a gap past the last dot, as that mark draws it.
    function groundEndAtMark(markX, ruler) {
        return face.cutDotX(ruler, 2, markX) + face.cutDotSize + Theme.spaceXs / 2
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

    /// `text` cut to fill `room`, each piece measured by `measure` (a string's width as the field lays it out) and the
    /// cut mark `markWidth` wide between them (`CutMark`, drawn by `CutPieces`): at its end, or in its middle, the head
    /// given the odd letter — as many letters kept as the room holds, the mark included. The answer holds a `…` where
    /// the mark goes (`cutPoint` finds it), so it reads as the cut value. Not Qt's own elide: its middle cut takes
    /// letters off both ends at once and stops up to two letters short, a gap before whatever stands at the room's
    /// edge (a combo's arrow). The measure is a `TextMetrics`'s, not a `FontMetrics.advanceWidth`, which answers the
    /// hinted advance where the field lays out with design metrics (`qquicktextmetrics.cpp`) and cuts a letter early.
    /// An empty text has nothing to cut, room or none: a field not laid out yet has none, and a mark there would widen
    /// the field it is sized by and flip it back (`tst_appcombo`).
    function cutToFit(measure, text, room, atEnd, markWidth) {
        if (text === "" || measure(text) <= room)
            return text
        const fits = (head, tail) => head + tail < text.length
            && measure(text.slice(0, head)) + markWidth + measure(text.slice(text.length - tail)) <= room
        if (atEnd) {
            let kept = 0
            while (fits(kept + 1, 0))
                kept++
            return text.slice(0, kept) + "\u2026"
        }
        let head = 0
        let tail = 0
        for (;;) {
            if (head <= tail && fits(head + 1, tail))
                head++
            else if (fits(head, tail + 1))
                tail++
            else if (fits(head + 1, tail))
                head++
            else
                break
        }
        return text.slice(0, head) + "\u2026" + text.slice(text.length - tail)
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
    /// §アイコンの席「顔と回数押しの印」). Not `capitalMiddle`: a mark drawn to the rounded-up height but centred on the
    /// measured middle stands up to half a pixel off the name marks in the same column.
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
