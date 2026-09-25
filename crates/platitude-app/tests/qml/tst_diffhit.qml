import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// Where a press lands in a row's line, and where the washes on it go (`LineRuler`).
//
// Only a laid-out line can answer: wide-glyph fallbacks are not monospaced and combining marks draw nothing, and a
// wash a few pixels off photographs like one on. Each place is held against a `TextInput` carrying the characters
// the markup stands for (Qt's other text path) and the drawn `Label` — a ruler agreeing only with itself would pass
// any rule.
//
// `cargo xtask qmltest` runs offscreen, which advances every full-width glyph alike, so only the agreement is judged
// here; real advances need `qmltestrunner` without `-platform offscreen` (rules-refs/app-ui.md).
Item {
    id: root
    width: 600
    height: 200

    /// Built from code points: written as glyphs they would read like non-English wording (CLAUDE.md 絶対制約).
    readonly property string acute: String.fromCharCode(0x301)
    readonly property string combining: ("e" + root.acute).repeat(40)
    readonly property string cjk: (String.fromCharCode(0x306e) + String.fromCharCode(0x65e5)
                                   + "abc" + String.fromCharCode(0x3002)).repeat(6)
    readonly property string emoji: ("a" + String.fromCodePoint(0x1f600)).repeat(6)
    readonly property string fullwidth: String.fromCharCode(0xff21, 0xff22, 0xff23) + "de"

    /// `markup::styled` for a line with nothing to colour. Tabs are spelled as the spaces to the stop at four, here
    /// and in `spelledOf`.
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
    /// Markup cut into the pieces the theme's runs would wrap, every entity kept whole.
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

    /// Wired as `DiffPane` wires it: a ruler in another format or size measures a line the diff never draws.
    LineRuler {
        id: ruler
        textFormat: TextEdit.RichText
        font.pixelSize: Theme.fontCode
    }
    /// The second instrument: the characters themselves in the rows' family and size, on Qt's other text path.
    TextInput {
        id: reference
        visible: false
        font.family: Theme.monoFamily
        font.pixelSize: Theme.fontCode
    }
    /// What the pane really draws — the width the two have to end at.
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

        /// Puts one line on all three and waits until two sides agree: a `Text` answers at the family's own
        /// advance until the fallback is resolved, and `waitForRendering` answers false on a real platform.
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

        /// What the washes stand on: `DiffLineCell` lays its rectangles between the places of the runs
        /// `encode::markup::spelled_ranges` names.
        function test_a_place_is_drawn_where_the_line_draws_it(data) {
            const spelled = shown(data.line)
            for (let at = 0; at <= spelled.length; at++)
                compare(ruler.xOf(at), reference.positionToRectangle(at).x, "place " + at)
        }

        function test_a_point_along_the_line_names_the_place_under_it_data() { return lines() }

        /// The way back, at every pixel: the press.
        function test_a_point_along_the_line_names_the_place_under_it(data) {
            const spelled = shown(data.line)
            const markup = root.markupOf(data.line)
            for (let x = 0; x <= drawn.implicitWidth; x++)
                compare(ruler.placeAt(markup, false, x), reference.positionAt(x, 0), "x " + x)
        }

        function test_the_blank_right_of_the_line_is_the_line_s_own_end_data() { return lines() }

        /// However far past: a walk of columns counts characters never drawn, and would answer mid-line far right of
        /// the ink.
        function test_the_blank_right_of_the_line_is_the_line_s_own_end(data) {
            const spelled = shown(data.line)
            const markup = root.markupOf(data.line)
            const ink = drawn.implicitWidth
            for (const past of [1, 8, 40, 400, 4000]) {
                compare(ruler.placeAt(markup, false, ink + past), spelled.length,
                        "the blank " + past + "px past the line")
            }
            compare(ruler.xOf(spelled.length), ink)
        }

        /// Trailing spaces inside the line and the blank right of it look alike and are different places
        /// (デザイン規約 §diff の中身をコピーする).
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

        function test_a_run_is_the_rectangle_between_its_two_places_data() { return lines() }

        function test_a_run_is_the_rectangle_between_its_two_places(data) {
            const spelled = shown(data.line)
            const markup = root.markupOf(data.line)
            if (spelled.length < 2)
                return
            const from = 1
            const len = spelled.length - 1
            const rects = ruler.rectsOf(markup, false, [{ from: from, len: len }])
            compare(rects.length, 1)
            compare(rects[0].x, reference.positionToRectangle(from).x)
            compare(rects[0].w, reference.positionToRectangle(from + len).x
                                - reference.positionToRectangle(from).x)
            // Two runs come back as two rectangles, in the order they were named.
            const pair = ruler.rectsOf(markup, false, [{ from: 0, len: 1 }, { from: from, len: len }])
            compare(pair.length, 2)
            compare(pair[1].x, rects[0].x)
        }

        /// The colours arrive after the rows (`SessionEvent::DiffColoured`), so a line reaches the ruler plain and
        /// then in `<font …>` runs; a place that moved would jump a selection dragged across the arrival.
        function test_the_colours_arriving_move_no_place_data() { return lines() }

        function test_the_colours_arriving_move_no_place(data) {
            const spelled = shown(data.line)
            const plain = root.markupOf(data.line)
            // What `markup::styled` writes once the theme has colours: runs splitting the same characters.
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
                const run = [{ from: 1, len: spelled.length - 1 }]
                const before = ruler.rectsOf(plain, false, run)
                const after = ruler.rectsOf(coloured, false, run)
                compare(after.length, before.length)
                compare(after[0].x, before[0].x)
                compare(after[0].w, before[0].w)
            }
        }

        function test_a_row_with_no_runs_has_no_rectangles() {
            compare(ruler.rectsOf(root.markupOf("abc"), false, []).length, 0)
        }

        /// A row is handed its line and its runs one property at a time (`DiffRowDelegate`), so a run can stand
        /// against a line without its places. It washes nothing: each place past the end would be a rectangle at
        /// nowhere and a Qt warning.
        function test_a_run_the_line_has_no_places_for_washes_nothing() {
            compare(ruler.rectsOf("", false, [{ from: 0, len: 33 }]).length, 0)
            compare(ruler.rectsOf(root.markupOf("abc"), false, [{ from: 0, len: 33 }]).length, 0)
            compare(ruler.rectsOf(root.markupOf("abc"), false, [{ from: 2, len: 2 }]).length, 0)
            // The run that ends exactly at the line's end is the line's own, and is drawn.
            compare(ruler.rectsOf(root.markupOf("abc"), false, [{ from: 0, len: 3 }]).length, 1)
        }

        /// On a line reading both ways a run's two ends can name the same x and wash nothing; the walk puts every
        /// place the run names inside a piece.
        function test_a_run_across_a_word_that_reads_the_other_way_still_covers_its_places() {
            // Four right-to-left letters between Latin ones, built from code points like the lines at the top.
            const rtl = String.fromCharCode(0x05d0, 0x05d1, 0x05d2, 0x05d3)
            const line = "abc " + rtl + " def"
            const markup = root.markupOf(line)
            shown(line)
            verify(ruler.twoWay(markup), "the line was not seen as reading both ways")
            // Out of order, which is why the walk exists.
            verify(ruler.xOf(5) > ruler.xOf(6), "the letters were not laid out the other way round")
            for (const run of [{ from: 2, len: 5 }, { from: 4, len: 4 }, { from: 0, len: 12 }, { from: 5, len: 2 }]) {
                const said = run.from + ":" + run.len
                const to = run.from + run.len
                const pieces = ruler.rectsOf(markup, false, [run])
                verify(pieces.length > 0, said + " washed nothing")
                for (let at = run.from; at <= to; at++) {
                    const x = ruler.xOf(at)
                    let held = false
                    for (const piece of pieces)
                        held = held || (x >= piece.x - 0.01 && x <= piece.x + piece.w + 0.01)
                    verify(held, said + ": place " + at + " at " + x + " is outside every piece")
                }
            }
        }

        function test_the_pieces_that_touch_are_one_rectangle() {
            const line = "let value = compute(4);"
            shown(line)
            const walked = ruler.walked(2, line.length)
            compare(walked.length, 1)
            compare(walked[0].x, ruler.xOf(2))
            compare(walked[0].w, ruler.xOf(line.length) - ruler.xOf(2))
        }

        /// The walk is exact either way; skipping it keeps a long row to two questions a run.
        function test_a_line_that_reads_one_way_is_never_walked_data() { return lines() }

        function test_a_line_that_reads_one_way_is_never_walked(data) {
            verify(!ruler.twoWay(root.markupOf(data.line)))
        }

        /// The row's weight reaches the ruler (`DiffRowDelegate.codeBold`): a bold face that advanced differently
        /// would put the washes and the press out.
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
