import QtQuick
import platitude.ui

// Where a place in a line is drawn, and which place a point along it is over. One line at a time, laid out where
// nobody sees it.
//
// **It exists because a `Text` cannot be asked.** The rows draw their lines in a `Label`, and a `Text` publishes
// `contentWidth`, `advance` and `linkAt` — nothing that maps an x to a place in the text (read off the item itself,
// Qt 6.10). So the panes asked a count of columns instead, and **a column is not a width and no arithmetic makes it
// one**: the fallback a Latin-only mono family hands a wide glyph to is not monospaced (13px, 11, 9 and 7 for four
// full-width glyphs against 8 for ASCII, Windows `Cascadia Mono` at `fontCode`), and a combining mark is a character
// the count sees and the font draws nothing for. Four hundred combining pairs stand 800 columns and are drawn in
// 3,200px, so a press at 3,600 came back as a byte in the middle of the line.
//
// **The pane that owns one sets the format and the size its own rows are drawn in** — a ruler reading the same line
// in another format, family, size or weight is measuring a line that pane never draws (`DiffTextMetrics`, the same
// rule), and neither of the two is right for both users: the diff's rows are the markup `markup::styled` writes, at
// `fontCode` (`DiffPane`); the command log's three columns are the characters themselves, at `fontSm`
// (`CommandsPane`). Whichever it is, **nothing here unescapes anything**: a `TextEdit` counts its places in the text
// its format says the string stands for, which is the text the row draws.
//
// Measured against the `Label` the rows are set in — same family, size and weight — on ASCII, combining marks, CJK,
// emoji, indentation, trailing spaces and escaped tags: every one agrees to the pixel, and agrees **cold**, in the
// same statement the line was put on it (`tests/qml/tst_diffhit.qml`, `tst_commandshit.qml`). That is what lets one
// ruler serve every row of a pane: the answer is a property of the line.
//
// **Nobody binds to it** — every function below sets the line it is asking about first, so two callers in one frame
// cannot read each other's answer.
TextEdit {
    id: ruler

    visible: false
    // A ruler: the press, the caret and the keyboard all stay with the pane's own sheet.
    readOnly: true
    activeFocusOnPress: false
    selectByMouse: false
    // The one thing both users have in common. `textFormat` and `font.pixelSize` are the owner's to write, and are
    // written at both call sites: there is no default here that could be right for both of them.
    font.family: Theme.monoFamily

    /// Puts one line on the ruler. A line already standing here is not laid out again.
    function hold(line, bold) {
        ruler.font.bold = bold
        ruler.text = line
    }
    /// Which place of the line a point `x` along it is over — a boundary between two characters, never inside one (a
    /// surrogate pair and a combining sequence are each one place to stop at). Past the last character it is the
    /// line's own end, which is what makes the blank right of a short row belong to that row's end and to no place
    /// inside it.
    function placeAt(line, bold, x) {
        ruler.hold(line, bold)
        return ruler.positionAt(x, 0)
    }
    /// The rectangles the runs (`[{ from, len }, …]`) cover, in the line's own coordinates — `[{ x, w }, …]`, and
    /// empty for nothing (`encode::markup::spelled_ranges` / `plain_ranges`).
    ///
    /// **A run is one rectangle only while the line reads one way.** Where a line carries a right-to-left run the
    /// places along it stop being in order (measured: three Latin letters, four of a right-to-left alphabet and
    /// three more Latin put their places at 32, 56, 48, 40, 64), and the two ends of a range say nothing about what
    /// is between them — taken as a span they can even name the same x twice and wash **nothing at all**. So such a
    /// line is walked place by place, each step is a piece of the row wherever it was drawn, and the pieces that
    /// touch are merged. What that buys is the guarantee the wash needs: **every place the run names is inside a
    /// piece**. Every other line — which is every line of source and every command either pane has been handed —
    /// costs two questions a run and is exact.
    ///
    /// **Known limit**: a run that begins or ends *inside* an opposite-direction run is covered approximately, and
    /// the approximation is on the generous side. A cursor sitting on a direction boundary has two places on screen
    /// and Qt's QML text API answers with one of them — `TextEdit` / `TextInput` publish `positionToRectangle` and
    /// `positionAt` and nothing that takes the direction the caller means, so there is no way from here to ask which
    /// (Qt 6.10). The pieces then reach past the characters the run names.
    function rectsOf(line, bold, runs) {
        if (!runs || runs.length === 0)
            return []
        ruler.hold(line, bold)
        const twoWay = ruler.twoWay(line)
        let out = []
        for (const run of runs) {
            const from = run.from
            const to = from + run.len
            // **A run past the end of the line belongs to another.** A row is handed its line and the runs on it
            // one property at a time (`DiffRowDelegate`, `reuseItems`), so in between them a run can stand against
            // a line that has none of its places — the row before's runs on the row after's blank line. Nothing is
            // drawn for it and nothing is asked of the layout: the half still to arrive settles the washes again,
            // and a place past the end is a rectangle at nowhere and a warning from Qt for each one asked.
            if (to > ruler.length)
                return []
            out = out.concat(twoWay ? ruler.walked(from, to)
                                    : [ruler.spanned(ruler.xOf(from), ruler.xOf(to))])
        }
        return out
    }
    /// Where one place is drawn. **Only ever a place the line has** — a place past the end is a rectangle at
    /// nowhere and a warning from Qt, so the runs are held against the line first (`rectsOf`).
    function xOf(at) { return ruler.positionToRectangle(at).x }
    /// Two places as the one rectangle between them, whichever way round they came.
    function spanned(a, b) { return { x: Math.min(a, b), w: Math.abs(b - a) } }
    /// One run, place by place: the step between two places is a piece of the line wherever it was drawn, and the
    /// pieces that touch are one rectangle.
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
    /// Whether the line can be drawn in both directions — the strong right-to-left blocks, the marks that turn a run
    /// on their own, their presentation forms, and the lead surrogates the right-to-left planes begin with. Said in
    /// code points: this is a rule, and letters of a script
    /// written out here read like the hardcoded wording the rules forbid (CLAUDE.md 絶対制約 — the same reason
    /// `DiffTextMetrics` builds its wide ruler out of `String.fromCharCode`).
    ///
    /// **Conservative on purpose**: a line this says yes about is walked, which is the right answer either way and
    /// costs only the walk.
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
