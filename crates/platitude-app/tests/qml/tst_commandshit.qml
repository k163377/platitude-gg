import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// Where a press lands in one of the command log's three columns, and where the wash on it goes (`LineRuler` as
// `CommandsPane` sets it up) — `tst_diffhit.qml`'s questions, asked of plain columns at `fontSm` instead of markup.
//
// Only a laid-out column can answer: wide-glyph fallbacks are not monospaced and combining marks draw nothing, and a
// wash a few pixels off photographs like one on. Each place is held against a `TextInput` (Qt's other text path) and
// the drawn `Label` — a ruler agreeing only with itself would pass any rule.
//
// `cargo xtask qmltest` runs offscreen, which advances every full-width glyph alike, so only the agreement is judged
// here; real advances need `qmltestrunner` without `-platform offscreen` (rules-refs/app-ui.md).
Item {
    id: root
    width: 600
    height: 200

    /// Built from code points: written as glyphs they would read like non-English wording (CLAUDE.md 絶対制約).
    readonly property string acute: String.fromCharCode(0x301)
    readonly property string kanji: String.fromCharCode(0x65e5, 0x672c, 0x8a9e)
    readonly property string cjkPath: "git add -- " + root.kanji + ".txt"
    readonly property string combining: "git commit -m 'e" + root.acute + "e" + root.acute + "e" + root.acute + "'"
    readonly property string emoji: "git commit -m '" + String.fromCodePoint(0x1f600) + " done'"

    /// Wired as `CommandsPane` wires it: a ruler in another format or size measures a column this log never draws.
    LineRuler {
        id: ruler
        textFormat: TextEdit.PlainText
        font.pixelSize: Theme.fontSm
    }
    /// The second instrument: the rows' family and size, on Qt's other text path.
    TextInput {
        id: reference
        visible: false
        font.family: Theme.monoFamily
        font.pixelSize: Theme.fontSm
    }
    /// What a row really draws — the width the two have to end at.
    Label {
        id: drawn
        visible: false
        textFormat: Text.PlainText
        font.family: Theme.monoFamily
        font.pixelSize: Theme.fontSm
    }
    /// The same ruler in the diff's format, for the last test.
    LineRuler {
        id: asMarkup
        textFormat: TextEdit.RichText
        font.pixelSize: Theme.fontSm
    }

    TestCase {
        name: "CommandsLineHit"
        when: windowShown

        /// One of each of the three the model cuts a row into (`selection::Line`), and the shapes a command line can
        /// carry into the middle one.
        function columns() {
            return [
                { tag: "clock", text: "12:03:17" },
                { tag: "command", text: "git switch -- 3.2" },
                { tag: "outcome", text: "exit 128 12 ms" },
                { tag: "cjk", text: root.cjkPath },
                { tag: "combining", text: root.combining },
                { tag: "emoji", text: root.emoji },
                { tag: "tags", text: "git commit -m '<n> & <m>'" },
                { tag: "spaces", text: "git commit -m 'ends   '" },
                { tag: "empty", text: "" },
            ]
        }

        /// Puts one column on all three and waits until two sides agree: a `Text` answers at the family's own
        /// advance until the fallback is resolved, and `waitForRendering` answers false on a real platform.
        function shown(text) {
            drawn.text = text
            reference.text = text
            ruler.hold(text, false)
            tryVerify(function () { return reference.contentWidth === drawn.implicitWidth },
                      undefined, "reference " + reference.contentWidth + " against drawn " + drawn.implicitWidth)
        }

        function test_a_place_is_drawn_where_the_column_draws_it_data() { return columns() }

        /// What the wash stands on: `CommandRowDelegate` lays its rectangle between the places of the runs
        /// `encode::markup::plain_ranges` names.
        function test_a_place_is_drawn_where_the_column_draws_it(data) {
            shown(data.text)
            for (let at = 0; at <= data.text.length; at++)
                compare(ruler.xOf(at), reference.positionToRectangle(at).x, "place " + at)
        }

        function test_a_point_along_the_column_names_the_place_under_it_data() { return columns() }

        /// The way back, at every pixel: the press (`CommandsTextSelect.hit`).
        function test_a_point_along_the_column_names_the_place_under_it(data) {
            shown(data.text)
            for (let x = 0; x <= drawn.implicitWidth; x++)
                compare(ruler.placeAt(data.text, false, x), reference.positionAt(x, 0), "x " + x)
        }

        function test_the_blank_right_of_the_column_is_its_own_end_data() { return columns() }

        /// However far past: a walk of columns counts characters never drawn, and a drag out to the right of the
        /// panel would select a byte mid-line.
        function test_the_blank_right_of_the_column_is_its_own_end(data) {
            shown(data.text)
            const ink = drawn.implicitWidth
            for (const past of [1, 8, 40, 400, 4000]) {
                compare(ruler.placeAt(data.text, false, ink + past), data.text.length,
                        "the blank " + past + "px past the column")
            }
            compare(ruler.xOf(data.text.length), ink)
        }

        function test_a_run_is_the_rectangle_between_its_two_places_data() { return columns() }

        /// The delegate's wash is the bounding box of what comes back (`CommandRowDelegate.bounds`).
        function test_a_run_is_the_rectangle_between_its_two_places(data) {
            shown(data.text)
            if (data.text.length < 2)
                return
            const from = 1
            const len = data.text.length - 1
            const rects = ruler.rectsOf(data.text, false, [{ from: from, len: len }])
            compare(rects.length, 1)
            compare(rects[0].x, reference.positionToRectangle(from).x)
            compare(rects[0].w, reference.positionToRectangle(from + len).x
                                - reference.positionToRectangle(from).x)
        }

        /// Trailing spaces inside the column and the blank right of it look alike and are different places.
        function test_the_spaces_a_column_really_holds_are_places_to_stop_at() {
            const text = "git commit -m 'ends   '"
            shown(text)
            const before = ruler.xOf(19)
            const ends = ruler.xOf(text.length)
            verify(ends > before, "the trailing spaces are drawn: " + before + " to " + ends)
            const inside = ruler.placeAt(text, false, (before + ends) / 2)
            verify(inside > 19 && inside < text.length, "a press between them named place " + inside)
        }

        function test_a_column_with_no_run_has_no_rectangle() {
            compare(ruler.rectsOf("git add --all", false, []).length, 0)
        }

        /// Why the pane names its ruler's format: read as markup, a command line's tags drop and `&` begins an
        /// entity, so the wash and the press would sit left of the characters they name.
        function test_the_format_the_ruler_is_set_in_is_the_row_s_own() {
            const text = "git commit -m '<n> & <m>'"
            shown(text)
            compare(ruler.xOf(text.length), drawn.implicitWidth)
            asMarkup.hold(text, false)
            verify(asMarkup.xOf(text.length) < drawn.implicitWidth,
                   "the same column read as markup ended at " + asMarkup.xOf(text.length)
                   + ", the same place the row drew it to — so this says nothing about the format")
        }
    }
}
