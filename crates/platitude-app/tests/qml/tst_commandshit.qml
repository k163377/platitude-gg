import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// Where a press lands in one of the command log's three columns, and where the wash on it goes (`LineRuler` as
// `CommandsPane` sets it up).
//
// **Only a laid-out column can answer**: a column of characters is not a count of columns — the fallback a Latin-only
// mono family hands a wide glyph to is not monospaced, and a combining mark is a character a walk counts and the font
// draws nothing for. No Rust test reaches it (that side maps bytes to places and back, `encode::markup`), and the
// headless screenshots cannot judge it either — a wash a few pixels off the characters it names photographs like one
// on them.
//
// **The expected place is taken from a second instrument, never from the one under test**: the ruler is a `TextEdit`
// reading the column through `QTextDocument`; every case here is held against a `TextInput` carrying the same
// characters, which is a different QML type on Qt's other text path, and against the `Label` the rows are actually
// drawn in. A ruler that agreed only with itself would pass on any rule at all.
//
// **The log's columns are plain and the diff's rows are markup**, which is the one thing this fixes that
// `tst_diffhit.qml` does not: the ruler is set in `TextEdit.PlainText` at `fontSm`, and the rows name the same
// format rather than leaving Qt to guess it from what a command happens to hold (`CommandRowDelegate`). The last
// test measures what naming it buys — the same column read as markup is a different column.
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

    /// Columns built from code points rather than written out: these are rulers, not words (`DiffTextMetrics`).
    readonly property string acute: String.fromCharCode(0x301)
    readonly property string kanji: String.fromCharCode(0x65e5, 0x672c, 0x8a9e)
    readonly property string cjkPath: "git add -- " + root.kanji + ".txt"
    readonly property string combining: "git commit -m 'e" + root.acute + "e" + root.acute + "e" + root.acute + "'"
    readonly property string emoji: "git commit -m '" + String.fromCodePoint(0x1f600) + " done'"

    /// The one under test, wired the way `CommandsPane` wires it — the format and the size are the pane's to set, and
    /// a ruler set in another of either is measuring a column this log never draws.
    LineRuler {
        id: ruler
        textFormat: TextEdit.PlainText
        font.pixelSize: Theme.fontSm
    }
    /// The instrument its answers are held against: the same characters, in the family and size the rows are set in,
    /// on Qt's other text path.
    TextInput {
        id: reference
        visible: false
        font.family: Theme.monoFamily
        font.pixelSize: Theme.fontSm
    }
    /// And the column itself — what a row really draws, for the width the two have to end at.
    Label {
        id: drawn
        visible: false
        textFormat: Text.PlainText
        font.family: Theme.monoFamily
        font.pixelSize: Theme.fontSm
    }
    /// The same ruler set in the diff's format, for the one test that measures what naming the format buys.
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

        /// Puts one column on all three and waits for them to settle on one width. **Never read at a moment of this
        /// test's choosing**: a `Text` answers at the family's own advance until the fallback carrying the other
        /// glyphs is resolved, and a rendered frame is not something a test window is promised
        /// (`waitForRendering` answers false on a real platform, `tst_diffreach`). What is waited for is the two
        /// sides agreeing, which is only true once both have settled.
        function shown(text) {
            drawn.text = text
            reference.text = text
            ruler.hold(text, false)
            tryVerify(function () { return reference.contentWidth === drawn.implicitWidth },
                      undefined, "reference " + reference.contentWidth + " against drawn " + drawn.implicitWidth)
        }

        function test_a_place_is_drawn_where_the_column_draws_it_data() { return columns() }

        /// Every place of the column, against the same place read off the other instrument. This is the whole of what
        /// the wash stands on: `CommandRowDelegate` asks for the runs `encode::markup::plain_ranges` names and lays
        /// its rectangle between the answers.
        function test_a_place_is_drawn_where_the_column_draws_it(data) {
            shown(data.text)
            for (let at = 0; at <= data.text.length; at++)
                compare(ruler.xOf(at), reference.positionToRectangle(at).x, "place " + at)
        }

        function test_a_point_along_the_column_names_the_place_under_it_data() { return columns() }

        /// And the way back, at every pixel across the column: the press the reader makes
        /// (`CommandsTextSelect.hit`).
        function test_a_point_along_the_column_names_the_place_under_it(data) {
            shown(data.text)
            for (let x = 0; x <= drawn.implicitWidth; x++)
                compare(ruler.placeAt(data.text, false, x), reference.positionAt(x, 0), "x " + x)
        }

        function test_the_blank_right_of_the_column_is_its_own_end_data() { return columns() }

        /// **The reported bug**: a point past the last character of the column is the column's end and no place
        /// inside it, however far past. The walk of columns it used to be read by went on counting characters that
        /// were never drawn, so a drag out to the right of the panel selected a byte in the middle of the line.
        function test_the_blank_right_of_the_column_is_its_own_end(data) {
            shown(data.text)
            const ink = drawn.implicitWidth
            for (const past of [1, 8, 40, 400, 4000]) {
                compare(ruler.placeAt(data.text, false, ink + past), data.text.length,
                        "the blank " + past + "px past the column")
            }
            // And the end is where the column was drawn to, not where a count of its characters would put it.
            compare(ruler.xOf(data.text.length), ink)
        }

        function test_a_run_is_the_rectangle_between_its_two_places_data() { return columns() }

        /// One run comes back as one rectangle, and it is that run's own two places — the wash the delegate lays is
        /// the bounding box of what comes back (`CommandRowDelegate.bounds`).
        function test_a_run_is_the_rectangle_between_its_two_places(data) {
            shown(data.text)
            if (data.text.length < 2)
                return
            const from = 1
            const len = data.text.length - 1
            const rects = ruler.rectsOf(data.text, false, from + ":" + len)
            compare(rects.length, 1)
            compare(rects[0].x, reference.positionToRectangle(from).x)
            compare(rects[0].w, reference.positionToRectangle(from + len).x
                                - reference.positionToRectangle(from).x)
        }

        /// A press inside spaces the command really holds is a press on them. A `-m` message ending in spaces is the
        /// case: the blank right of the column and the blank inside it are the same colour and different places.
        function test_the_spaces_a_column_really_holds_are_places_to_stop_at() {
            const text = "git commit -m 'ends   '"
            shown(text)
            const before = ruler.xOf(19)
            const ends = ruler.xOf(text.length)
            verify(ends > before, "the trailing spaces are drawn: " + before + " to " + ends)
            const inside = ruler.placeAt(text, false, (before + ends) / 2)
            verify(inside > 19 && inside < text.length, "a press between them named place " + inside)
        }

        /// Nothing to wash asks nothing of the ruler.
        function test_a_column_with_no_run_has_no_rectangle() {
            compare(ruler.rectsOf("git add --all", false, "").length, 0)
        }

        /// **Why the pane names the format its ruler is set in.** A command line can carry anything a shell would
        /// quote, and the same characters read as markup are a different line: the tags are dropped and the `&`
        /// begins an entity. The log's ruler reads them as themselves because `CommandsPane` says so — a ruler left
        /// in the diff's format would answer about a column no row here draws, and the wash and the press would both
        /// sit left of the characters they name.
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
