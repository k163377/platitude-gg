pragma Singleton

import QtQuick
import platitude.ui

// The lines of the window's face that a mark set beside words is drawn to (デザイン規約 §アイコンの席): the capitals'
// height, which every set mark is drawn to, and the face's own `+` — where a sign stands and the room it takes. Read
// off the face at run time, so each OS's face answers for itself; in shares of the pixel size, measured large, where
// hinting does not round them.
QtObject {
    id: face

    readonly property FontMetrics large: FontMetrics {
        font.family: Theme.uiFamily
        font.pixelSize: 1000
    }
    /// The capitals' height.
    readonly property real capital: face.large.capitalHeight / 1000
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

    function share(rect) {
        return { y: rect.y / 1000, width: rect.width / 1000, height: rect.height / 1000 }
    }
    /// How far above the baseline the capitals' middle stands, at `size` — where a picture beside words (a face)
    /// sets its own middle.
    function capitalMiddle(size) {
        return face.capital * size / 2
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
    /// The same to the face's `+`'s middle: where a sign that turns about its own middle (the ring) stands.
    function signDrop(words) {
        return Math.round(words.baselineOffset - face.signMiddle(words.font.pixelSize) - words.height / 2)
    }
}
