import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// Where a press lands in a row's line, and where the washes on it go (`DiffLineRuler`).
//
// **Only a laid-out line can answer**: a column is not a width — the fallback a Latin-only mono family hands a wide
// glyph to is not monospaced, and a combining mark is a character a walk counts and the font draws nothing for. No
// Rust test reaches it (that side maps bytes to places and back, `encode::markup`), and the headless screenshots
// cannot judge it either — a wash a few pixels off the characters it names photographs like one on them.
//
// **The expected place is taken from a second instrument, never from the one under test**: the ruler is a `TextEdit`
// reading the row's markup through `QTextDocument`; every case here is held against a `TextInput` carrying the
// characters that markup stands for, which is a different QML type on Qt's other text path, and against the `Label`
// the rows are actually drawn in. A ruler that agreed only with itself would pass on any rule at all.
//
// **`cargo xtask qmltest` runs this on `-platform offscreen`, where the answer is easier than the real one** —
// offscreen resolves every full-width glyph through one font at one advance, so a run there cannot speak for how a
// real window advances them (rules-refs/app-ui.md). So what is judged here is the *agreement* rather than any
// particular number, which is a claim both platforms can carry; the numbers themselves are read by running
// `qmltestrunner` on this file without `-platform offscreen`.
Item {
    id: root
    width: 600
    height: 200

    /// Lines built from code points rather than written out: these are rulers, not words (`DiffTextMetrics`).
    readonly property string acute: String.fromCharCode(0x301)
    readonly property string combining: ("e" + root.acute).repeat(40)
    readonly property string cjk: (String.fromCharCode(0x306e) + String.fromCharCode(0x65e5)
                                   + "abc" + String.fromCharCode(0x3002)).repeat(6)
    readonly property string emoji: ("a" + String.fromCodePoint(0x1f600)).repeat(6)
    readonly property string fullwidth: String.fromCharCode(0xff21, 0xff22, 0xff23) + "de"

    /// What a row holds and what it draws: `markup::styled` for a line with nothing to colour, and the characters
    /// that markup stands for. The tab is spelled as the spaces that reach the stop at four, both times.
    function markupOf(line) {
        let out = ""
        let col = 0
        for (const ch of line) {
            const step = ch === "\t" ? 4 - (col % 4) : 1
            out += ch === "&" ? "&amp;"
                 : ch === "<" ? "&lt;"
                 : ch === ">" ? "&gt;"
                 : ch === " " ? "&nbsp;"
                 : ch === "\t" ? "&nbsp;".repeat(step)
                 : ch
            col += step
        }
        return out
    }
    /// One line's markup cut into the pieces the theme's runs would wrap, without cutting an entity in half —
    /// `&nbsp;` is one character of the drawn line and five of this string.
    function pieces(markup) {
        const out = []
        let piece = ""
        let entity = false
        for (const ch of markup) {
            piece += ch
            if (ch === "&")
                entity = true
            else if (ch === ";")
                entity = false
            if (!entity && piece.length >= 4) {
                out.push(piece)
                piece = ""
            }
        }
        if (piece !== "")
            out.push(piece)
        return out
    }
    function spelledOf(line) {
        let out = ""
        let col = 0
        for (const ch of line) {
            const step = ch === "\t" ? 4 - (col % 4) : 1
            out += ch === "\t" ? " ".repeat(step) : ch
            col += step
        }
        return out
    }

    /// The one under test, wired the way `DiffPane` wires it.
    DiffLineRuler {
        id: ruler
    }
    /// The instrument its answers are held against: the characters themselves, in the family, size and weight the
    /// rows are set in, on Qt's other text path.
    TextInput {
        id: reference
        visible: false
        font.family: Theme.monoFamily
        font.pixelSize: Theme.fontCode
    }
    /// And the row itself — what the pane really draws, for the width the two have to end at.
    Label {
        id: drawn
        visible: false
        textFormat: Text.StyledText
        font.family: Theme.monoFamily
        font.pixelSize: Theme.fontCode
    }

    TestCase {
        name: "DiffLineHit"
        when: windowShown

        function lines() {
            return [
                { tag: "ascii", line: "let value = compute(4);" },
                { tag: "combining", line: root.combining },
                { tag: "cjk", line: root.cjk },
                { tag: "emoji", line: root.emoji },
                { tag: "fullwidth", line: root.fullwidth },
                { tag: "tabs", line: "\ta\tbc\tdef" },
                { tag: "indent", line: "        if (x) {" },
                { tag: "trailing", line: "code   " },
                { tag: "markup", line: "a<T>& b" },
                { tag: "empty", line: "" },
                { tag: "blank", line: "    " },
            ]
        }

        /// Puts one line on all three and waits for them to settle on one width. **Never read at a moment of this
        /// test's choosing**: a `Text` answers at the family's own advance until the fallback carrying the other
        /// glyphs is resolved, and a rendered frame is not something a test window is promised
        /// (`waitForRendering` answers false on a real platform, `tst_diffreach`). What is waited for is the two
        /// sides agreeing, which is only true once both have settled.
        function shown(line) {
            const markup = root.markupOf(line)
            const spelled = root.spelledOf(line)
            drawn.text = markup
            reference.text = spelled
            ruler.hold(markup, false)
            tryVerify(function () { return reference.contentWidth === drawn.implicitWidth },
                      undefined, "reference " + reference.contentWidth + " against drawn " + drawn.implicitWidth)
            return spelled
        }

        function test_a_place_is_drawn_where_the_line_draws_it_data() { return lines() }

        /// Every place of the line, against the same place read off the other instrument. This is the whole of what
        /// the washes stand on: `DiffRowDelegate` asks for the runs `encode::markup::spelled_ranges` names and lays
        /// its rectangles between the answers.
        function test_a_place_is_drawn_where_the_line_draws_it(data) {
            const spelled = shown(data.line)
            for (let at = 0; at <= spelled.length; at++)
                compare(ruler.xOf(at), reference.positionToRectangle(at).x, "place " + at)
        }

        function test_a_point_along_the_line_names_the_place_under_it_data() { return lines() }

        /// And the way back, at every pixel across the row: the press the reader makes.
        function test_a_point_along_the_line_names_the_place_under_it(data) {
            const spelled = shown(data.line)
            const markup = root.markupOf(data.line)
            for (let x = 0; x <= drawn.implicitWidth; x++)
                compare(ruler.placeAt(markup, false, x), reference.positionAt(x, 0), "x " + x)
        }

        function test_the_blank_right_of_the_line_is_the_line_s_own_end_data() { return lines() }

        /// **The reported bug** (2026-09-08): a point past the last character of the row is the row's end and no
        /// place inside it, however far past. The walk of columns it used to be read by went on counting characters
        /// that were never drawn — four hundred combining pairs end at 3,200px and were still answering with places
        /// in the middle of the line at 4,000.
        function test_the_blank_right_of_the_line_is_the_line_s_own_end(data) {
            const spelled = shown(data.line)
            const markup = root.markupOf(data.line)
            const ink = drawn.implicitWidth
            for (const past of [1, 8, 40, 400, 4000]) {
                compare(ruler.placeAt(markup, false, ink + past), spelled.length,
                        "the blank " + past + "px past the line")
            }
            // And the end is where the line was drawn to, not where a count of its characters would put it.
            compare(ruler.xOf(spelled.length), ink)
        }

        /// A press inside spaces the file really holds is a press on them. The blank right of the line and the blank
        /// inside it are the same colour and different places (デザイン規約 §diff の中身をコピーする: what the reader
        /// can see they are getting is what they get).
        function test_the_spaces_a_line_really_holds_are_places_to_stop_at() {
            const line = "code   "
            const spelled = shown(line)
            const markup = root.markupOf(line)
            const ends = ruler.xOf(spelled.length)
            const code = ruler.xOf(4)
            verify(ends > code, "the trailing spaces are drawn: " + code + " to " + ends)
            const inside = ruler.placeAt(markup, false, (code + ends) / 2)
            verify(inside > 4 && inside < spelled.length, "a press between them named place " + inside)
        }

        /// A run is one rectangle where the line reads one way, and it is the run's own two places — not a prefix
        /// measured on its own, which is a different layout of a different string.
        function test_a_run_is_the_rectangle_between_its_two_places_data() { return lines() }

        function test_a_run_is_the_rectangle_between_its_two_places(data) {
            const spelled = shown(data.line)
            const markup = root.markupOf(data.line)
            if (spelled.length < 2)
                return
            const from = 1
            const len = spelled.length - 1
            const rects = ruler.rectsOf(markup, false, from + ":" + len)
            compare(rects.length, 1)
            compare(rects[0].x, reference.positionToRectangle(from).x)
            compare(rects[0].w, reference.positionToRectangle(from + len).x
                                - reference.positionToRectangle(from).x)
            // Two runs come back as two rectangles, in the order they were named.
            const pair = ruler.rectsOf(markup, false, "0:1," + from + ":" + len)
            compare(pair.length, 2)
            compare(pair[1].x, rects[0].x)
        }

        /// **The colours arrive after the rows** (`SessionEvent::DiffColoured`), so the same line reaches the ruler
        /// twice: once escaped and plain, once wrapped in the theme's `<font …>` runs. A place has to be the same
        /// place both times — a reader who pressed before the colours landed and dragged after would otherwise see
        /// the selection jump under the pointer.
        function test_the_colours_arriving_move_no_place_data() { return lines() }

        function test_the_colours_arriving_move_no_place(data) {
            const spelled = shown(data.line)
            const plain = root.markupOf(data.line)
            // What `markup::styled` writes once the theme has something to say: the runs split the line, and each
            // one carries the same characters it did before.
            let coloured = ""
            let at = 0
            const colours = ["#aabbcc", "#112233", "#445566"]
            for (const piece of root.pieces(plain)) {
                coloured += "<font color=\"" + colours[at % colours.length] + "\">" + piece + "</font>"
                at++
            }
            for (let p = 0; p <= spelled.length; p++)
                compare(ruler.xOf(p), reference.positionToRectangle(p).x, "plain, place " + p)
            ruler.hold(coloured, false)
            for (let p = 0; p <= spelled.length; p++)
                compare(ruler.xOf(p), reference.positionToRectangle(p).x, "coloured, place " + p)
            if (spelled.length >= 2) {
                const run = "1:" + (spelled.length - 1)
                const before = ruler.rectsOf(plain, false, run)
                const after = ruler.rectsOf(coloured, false, run)
                compare(after.length, before.length)
                compare(after[0].x, before[0].x)
                compare(after[0].w, before[0].w)
            }
        }

        /// Nothing to wash asks nothing of the ruler.
        function test_a_row_with_no_runs_has_no_rectangles() {
            compare(ruler.rectsOf(root.markupOf("abc"), false, "").length, 0)
        }

        /// A line that reads both ways has no order along it, so a run over one cannot be read off its two ends.
        /// **What the walk guarantees is that every place the run names is inside a piece** — the two ends can name
        /// the same x and would wash nothing at all, which is the failure this is here to stop.
        function test_a_run_across_a_word_that_reads_the_other_way_still_covers_its_places() {
            // Four letters of an alphabet that reads the other way, between Latin ones. Built from code points:
            // this is a fixture and not a word (`DiffTextMetrics`).
            const rtl = String.fromCharCode(0x05d0, 0x05d1, 0x05d2, 0x05d3)
            const line = "abc " + rtl + " def"
            const markup = root.markupOf(line)
            shown(line)
            verify(ruler.twoWay(markup), "the line was not seen as reading both ways")
            // The places of that word are out of order, which is the whole reason for the walk.
            verify(ruler.xOf(5) > ruler.xOf(6), "the letters were not laid out the other way round")
            for (const run of ["2:5", "4:4", "0:12", "5:2"]) {
                const cut = run.split(":")
                const from = Number(cut[0])
                const to = from + Number(cut[1])
                const pieces = ruler.rectsOf(markup, false, run)
                verify(pieces.length > 0, run + " washed nothing")
                for (let at = from; at <= to; at++) {
                    const x = ruler.xOf(at)
                    let held = false
                    for (const piece of pieces)
                        held = held || (x >= piece.x - 0.01 && x <= piece.x + piece.w + 0.01)
                    verify(held, run + ": place " + at + " at " + x + " is outside every piece")
                }
            }
        }

        /// And the pieces are merged rather than laid one on another: a run of a line that reads one way comes back
        /// as one rectangle even when it is walked.
        function test_the_pieces_that_touch_are_one_rectangle() {
            const line = "let value = compute(4);"
            shown(line)
            const walked = ruler.walked(2, line.length)
            compare(walked.length, 1)
            compare(walked[0].x, ruler.xOf(2))
            compare(walked[0].w, ruler.xOf(line.length) - ruler.xOf(2))
        }

        /// A line with none of those letters in it is never walked — the walk is exact either way, and this is the
        /// clause that keeps a two-thousand-column row to two questions a run.
        function test_a_line_that_reads_one_way_is_never_walked_data() { return lines() }

        function test_a_line_that_reads_one_way_is_never_walked(data) {
            verify(!ruler.twoWay(root.markupOf(data.line)))
        }

        /// The ruler is set in the row's own weight, because the two washes and the press all read the same layout
        /// (`DiffRowDelegate.codeBold`). A family whose bold face advanced differently would otherwise put every one
        /// of them out by the difference — this is what says the weight reaches the ruler at all.
        function test_the_weight_the_row_is_set_in_reaches_the_ruler() {
            const line = "let value = compute(4);"
            const markup = root.markupOf(line)
            drawn.text = markup
            drawn.font.bold = true
            reference.text = root.spelledOf(line)
            reference.font.bold = true
            ruler.hold(markup, true)
            tryVerify(function () { return reference.contentWidth === drawn.implicitWidth })
            compare(ruler.xOf(line.length), drawn.implicitWidth)
            drawn.font.bold = false
            reference.font.bold = false
        }
    }
}
