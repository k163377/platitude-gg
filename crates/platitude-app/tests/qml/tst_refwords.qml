import QtQuick
import QtTest
import platitude.ui

// The escape that puts a ref name in a sentence (`Words.nameInSentence`), held against the engine that draws it.
//
// **Only a laid-out line can answer this.** The two things the escape owes are opposite: a run of spaces has to
// survive a fold rich text takes from HTML, and a single space has to stay a place the line may break — one of the
// fields this is drawn in wraps rather than cutting (`NoticeLine`), and a sentence pinned at every space breaks
// through the middle of a word instead. Neither is visible in the markup itself, and no headless verb photographs the
// empty column at the width where it wraps, so both are read off the same rich-text engine the fields use.
Item {
    id: root
    width: 400
    height: 200

    readonly property string sentence: "The first commit will start %1"
    readonly property string spaced: "The first  commit will start %1"

    // The width is the measurement: wide enough to hold most of the sentence, too narrow to hold all of it.
    TextEdit {
        id: coloured
        width: 130
        wrapMode: Text.Wrap
        textFormat: TextEdit.RichText
        text: Words.nameInSentence(root.sentence, "main", "#60A5FA")
    }
    TextEdit {
        id: plain
        width: 130
        wrapMode: Text.Wrap
        textFormat: TextEdit.PlainText
        text: root.sentence.arg("main")
    }
    // The pair the fold is read from, both unwrapped: what the sentence asks for with its two spaces kept, and what
    // the same sentence asks for with one.
    TextEdit {
        id: runKept
        wrapMode: Text.NoWrap
        textFormat: TextEdit.RichText
        text: Words.nameInSentence(root.spaced, "main", "#60A5FA")
    }
    TextEdit {
        id: runFolded
        wrapMode: Text.NoWrap
        textFormat: TextEdit.RichText
        text: "The first  commit will start <font color=\"#60A5FA\">main</font>"
    }

    TestCase {
        name: "RefWords"
        when: windowShown

        /// The words on the first line, in the order the engine laid them out.
        function firstLineOf(field) {
            const step = field.contentHeight / field.lineCount
            return field.getText(0, field.positionAt(0, step * 1.5))
        }

        // How many lines it takes is the machine's answer — this runs on a font database that is not the window's
        // (verify-ui §Windows の罠), so what is asked is where the break landed and not how many there were.
        function test_the_coloured_line_breaks_where_the_plain_one_does() {
            verify(coloured.lineCount > 1)
            compare(firstLineOf(coloured), firstLineOf(plain))
            verify(firstLineOf(coloured).endsWith(" "))
        }

        function test_a_run_of_spaces_keeps_its_width() {
            verify(runKept.implicitWidth > runFolded.implicitWidth)
        }

        function test_a_name_that_looks_like_markup_stays_a_name() {
            const words = Words.nameInSentence(root.sentence, "<b>&x", "#60A5FA")
            verify(words.indexOf("&lt;b&gt;&amp;x") >= 0)
        }

        function test_a_sentence_with_no_seat_asks_for_no_markup() {
            compare(Words.nameInSentence("nothing to name here", "main", "#60A5FA"), "")
        }
    }
}
