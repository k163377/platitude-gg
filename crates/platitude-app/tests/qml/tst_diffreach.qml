import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// How far a diff reaches sideways: what the picked lines measure at, and what the rows on screen make of that.
//
// **Only a laid-out Label can answer the first half**: the model counts a line in columns, and what the pane needs is
// pixels — the fallback a Latin-only mono family hands a wide glyph to is not monospaced, and a combining mark is a
// character the count sees and the font draws nothing for. No Rust test reaches it, and the headless screenshots
// cannot judge it either (a diff sent to its end and one with nowhere to go photograph alike).
//
// **The expected width is taken from what the pane is supposed to draw**: the rulers read the row's markup as
// `StyledText`, and every case here is held against a `PlainText` Label carrying the characters that markup
// stands for. A ruler left on `AutoText` agrees with itself about a line of source that looks like markup; it
// does not agree with this.
Item {
    id: root
    width: 400
    height: 200

    /// Lines built from code points: these are rulers (`DiffTextMetrics`).
    readonly property string wideLine: String.fromCharCode(0x65e5).repeat(400)
    readonly property string narrowLine: "0".repeat(700)
    readonly property string mixedLine:
        (String.fromCharCode(0x306e) + String.fromCharCode(0x65e5) + "abc"
         + String.fromCharCode(0x3002)).repeat(120)

    /// What `markup::styled` writes for one line with nothing to colour, and the characters it stands for. The two
    /// are the same row: one is how it is stored and drawn, the other is what it draws.
    function escaped(line) {
        let out = ""
        let col = 0
        for (const ch of line) {
            const step = ch === "\t" ? 4 - (col % 4) : 1
            if (ch === "&")
                out += "&amp;"
            else if (ch === "<")
                out += "&lt;"
            else if (ch === ">")
                out += "&gt;"
            else if (ch === " ")
                out += "&nbsp;"
            else if (ch === "\t")
                out += "&nbsp;".repeat(step)
            else
                out += ch
            col += step
        }
        return out
    }
    function spelled(line) {
        let out = ""
        let col = 0
        for (const ch of line) {
            const step = ch === "\t" ? 4 - (col % 4) : 1
            out += ch === "\t" ? " ".repeat(step) : ch
            col += step
        }
        return out
    }

    /// The candidates as the model hands them over (`DiffModel.widestLines` — `{bold, line}`, the line in the
    /// markup the row draws).
    function candidates(lines) {
        return lines.map(line => ({ bold: true, line: root.escaped(line) }))
    }

    QtObject {
        id: diffModel
        property int widestNo: 1234
        property var widestLines: []
    }

    DiffTextMetrics {
        id: metrics
        diffModel: diffModel
        partial: false
    }

    /// The instrument the pane's answer is judged against: the characters themselves, in the family, size and weight
    /// the rows are set in, read as the plain text they are.
    property var shown: []
    Repeater {
        id: references
        model: root.shown
        Label {
            required property string modelData
            visible: false
            text: root.spelled(modelData)
            textFormat: Text.PlainText
            font.family: Theme.monoFamily
            font.pixelSize: Theme.fontCode
            font.bold: true
        }
    }
    /// How far the furthest of them is drawn, as far as they have settled. **The rows name the moment they are
    /// read at**: a `Text` answers with every glyph at the family's own advance until the fallback carrying the wide
    /// ones is resolved, and a rendered frame is not something a test window is promised
    /// (`waitForRendering` answers false on a real platform). What the cases below wait for is the two sides
    /// agreeing, which is only true once both have settled.
    function drawnWidth() {
        let widest = 0
        for (let i = 0; i < references.count; i++) {
            const label = references.itemAt(i)
            if (label)
                widest = Math.max(widest, label.implicitWidth)
        }
        return widest
    }

    /// The reach as the pane keeps it: what was picked and measured, and what the rows turned out to be.
    DiffReach {
        id: reach
        rowsGen: 1
        measured: 0
        file: "a.txt"
    }

    TestCase {
        name: "DiffReach"
        when: windowShown

        /// The pane's answer and the characters it stands for, once both have settled. Held as one condition:
        /// neither side is finished at a moment this test can name, and a zero on both sides means two things
        /// still to be measured.
        function agreed() {
            return metrics.codeW > 0 && metrics.codeW === root.drawnWidth()
        }

        function test_a_picked_line_measures_at_what_the_row_draws_data() {
            return [
                { tag: "wide", line: root.wideLine },
                { tag: "narrow", line: root.narrowLine },
                { tag: "mixed", line: root.mixedLine },
                // A line of source that looks like markup. Measured as the row reads it — escaped — against the
                // characters it stands for. A ruler left to guess the format reads the tags and comes out short.
                { tag: "markup", line: "<b>" + "0".repeat(100) + "</b> & <i>x</i>" },
                // Tabs: the row spells them as the spaces that reach the stop at four, and so does the ruler. A raw
                // tab handed to a Label is drawn at Qt's own stop, which is nowhere near.
                { tag: "tabs", line: "\ta\tbc\td" + "0".repeat(40) },
            ]
        }

        function test_a_picked_line_measures_at_what_the_row_draws(data) {
            root.shown = [data.line]
            diffModel.widestLines = root.candidates([data.line])
            tryVerify(agreed)
        }

        function test_the_picked_answer_is_the_furthest_of_them() {
            const lines = [root.wideLine, root.narrowLine, root.mixedLine]
            root.shown = lines
            diffModel.widestLines = root.candidates(lines)
            tryVerify(agreed)
        }

        /// Nothing picked measures at nothing, and holding the width through that gap is `DiffReach`'s to do —
        /// the cases below are where it is held against.
        function test_nothing_picked_measures_at_nothing() {
            root.shown = []
            diffModel.widestLines = []
            tryCompare(metrics, "codeW", 0)
        }

        function init() {
            reach.file = "a.txt"
            reach.rowsGen = 1
            reach.measured = 0
            reach.forgetFile()
        }

        /// A row that was never picked still decides the reach — the pick is by column count, and a column is not a
        /// width. This is the whole reason the rows are read at all.
        function test_a_row_reaches_past_what_was_picked() {
            reach.measured = 300
            compare(reach.width, 300)
            reach.noteRow(41, 900)
            compare(reach.width, 900)
        }

        /// Filed under the row, so a second look replaces the first. A running maximum could only ever grow, and a
        /// width measured before a fallback resolved — or before a row's text was — never came back off it.
        function test_a_row_that_came_back_narrower_takes_the_reach_with_it() {
            reach.noteRow(2, 900)
            reach.noteRow(7, 400)
            compare(reach.width, 900)
            reach.noteRow(2, 500)
            compare(reach.width, 500)
            reach.noteRow(7, 100)
            compare(reach.width, 500)
        }

        /// A new reading of the rows: the widths were measured on lines that may be gone, so they go. The width
        /// itself stays until this reading has measured something — publishing a zero would clamp the reader's place
        /// back to the left edge of a file they are still reading.
        function test_a_new_reading_forgets_the_rows_and_holds_the_width() {
            reach.measured = 200
            reach.noteRow(3, 800)
            compare(reach.width, 800)
            reach.rowsGen = 2
            compare(reach.width, 800)
            reach.noteRow(3, 350)
            compare(reach.width, 350)
        }

        /// Another file starts over: its rows are not this one's, and the place along it is reset beside this
        /// (`DiffCodeScroll.file`).
        function test_another_file_starts_over() {
            reach.noteRow(1, 800)
            compare(reach.width, 800)
            reach.file = "b.txt"
            compare(reach.width, 0)
        }
    }
}
