import QtQuick
import platitude.ui

// Where a place in a line is drawn, and which place a point along it is over — one line at a time, on a hidden
// `TextEdit`, because a `Text` cannot be asked and a column count is not a width (wide-glyph fallbacks are not
// monospaced; a combining mark is a character with no advance). The owning pane sets `textFormat` and
// `font.pixelSize` to what its rows are drawn in, and nothing here unescapes: places are counted in the text the
// format says the string stands for. Every function puts its line on first, so nobody binds to it and one ruler
// serves a pane, answering cold (`tests/qml/tst_diffhit.qml`, `tst_commandshit.qml`;
// rules-refs/app-ui.md「diff の当たり判定と 2 つの帯は、その行のレイアウトそのものに訊く」).
TextEdit {
    id: ruler

    visible: false
    // The press, the caret and the keyboard stay with the pane's own sheet.
    readOnly: true
    activeFocusOnPress: false
    selectByMouse: false
    // The only setting both owners share.
    font.family: Theme.monoFamily

    /// Puts one line on the ruler. A line already standing here is not laid out again.
    function hold(line, bold) {
        ruler.font.bold = bold
        ruler.text = line
    }
    /// The place `x` is over — a character boundary, never inside a surrogate pair or combining sequence. Past the last
    /// character, the line's end.
    function placeAt(line, bold, x) {
        ruler.hold(line, bold)
        return ruler.positionAt(x, 0)
    }
    /// The rectangles the runs (`[{ from, len }, …]`) cover, in the line's own coordinates — `[{ x, w }, …]`, and
    /// empty for nothing (`encode::markup::spelled_ranges` / `plain_ranges`).
    ///
    /// On a line that can read both ways (`twoWay`) places stop being in order, so a run's two ends say nothing about
    /// what lies between them; such a line is walked place by place and touching pieces merged, so every place the run
    /// names is inside a piece. **Known limit**: a run beginning or ending inside an opposite-direction run is covered
    /// generously — Qt answers only one of a direction boundary's two carets.
    function rectsOf(line, bold, runs) {
        if (!runs || runs.length === 0)
            return []
        ruler.hold(line, bold)
        const twoWay = ruler.twoWay(line)
        let out = []
        for (const run of runs) {
            const from = run.from
            const to = from + run.len
            // A run past the end belongs to the previous line: a reused row gets its line and its runs one property
            // at a time, and the other half's arrival settles the washes again.
            if (to > ruler.length)
                return []
            out = out.concat(twoWay ? ruler.walked(from, to)
                                    : [ruler.spanned(ruler.xOf(from), ruler.xOf(to))])
        }
        return out
    }
    /// Where one place is drawn. Only a place the line has — past the end, Qt answers a rectangle at nowhere and a
    /// warning (hence the check in `rectsOf`).
    function xOf(at) { return ruler.positionToRectangle(at).x }
    function spanned(a, b) { return { x: Math.min(a, b), w: Math.abs(b - a) } }
    /// One run, place by place, with touching pieces merged.
    function walked(from, to) {
        const parts = []
        let last = ruler.xOf(from)
        for (let at = from + 1; at <= to; at++) {
            const x = ruler.xOf(at)
            parts.push(ruler.spanned(last, x))
            last = x
        }
        parts.sort((a, b) => a.x - b.x)
        const out = []
        for (const part of parts) {
            const grown = out.length > 0 ? out[out.length - 1] : null
            if (grown && part.x <= grown.x + grown.w)
                grown.w = Math.max(grown.w, part.x + part.w - grown.x)
            else
                out.push(part)
        }
        return out
    }
    /// Whether the line may draw in both directions: the right-to-left blocks, direction marks and embeddings,
    /// presentation forms, and the lead surrogates of the right-to-left planes. In code points, since script letters
    /// here would read as hardcoded wording (CLAUDE.md 絶対制約). Over-inclusive on purpose — a false yes costs only
    /// the walk.
    function twoWay(line) {
        for (let i = 0; i < line.length; i++) {
            const code = line.charCodeAt(i)
            if ((code >= 0x0590 && code <= 0x08ff)
                    || (code >= 0x200e && code <= 0x200f)
                    || (code >= 0x202a && code <= 0x202e)
                    || (code >= 0x2066 && code <= 0x2069)
                    || (code >= 0xfb1d && code <= 0xfdff)
                    || (code >= 0xfe70 && code <= 0xfeff)
                    || (code >= 0xd802 && code <= 0xd803)
                    || code === 0xd83a)
                return true
        }
        return false
    }
}
