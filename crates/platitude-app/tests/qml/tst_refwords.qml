import QtQuick
import QtTest
import platitude.ui

// The markup that sets a name in a sentence as code (`CardText.namedMarkup`), held against the rich-text
// engine that draws it. It owes two opposite things: a run of spaces survives rich text's HTML fold, and a single space
// stays a place the line may break (`NoticeLine` wraps, and a sentence pinned at every space breaks mid-word). Neither
// shows in the markup, so both are read off laid-out lines.
Item {
    id: root
    width: 400
    height: 200

    readonly property string sentence: "The first commit will start %1"
    readonly property string spaced: "The first  commit will start %1"
    /// The name in the sentence in a colour of its own, with no mark (no room): the markup alone.
    function named(sentence, name) {
        return [[sentence.indexOf("%1"), "", Theme.textLink, name.length]]
    }

    CardText {
        id: colouredLine
        visible: false
        text: root.sentence.arg("main")
        marks: root.named(root.sentence, "main")
    }
    CardText {
        id: spacedLine
        visible: false
        text: root.spaced.arg("main")
        marks: root.named(root.spaced, "main")
    }
    CardText {
        id: markupLike
        visible: false
        text: root.sentence.arg("<b>&x")
        marks: root.named(root.sentence, "<b>&x")
    }
    CardText {
        id: unnamed
        visible: false
        text: "nothing to name here"
    }

    // Wide enough for most of the sentence, too narrow for all of it.
    TextEdit {
        id: coloured
        width: 130
        wrapMode: Text.Wrap
        textFormat: TextEdit.RichText
        text: colouredLine.namedMarkup
    }
    TextEdit {
        id: plain
        width: 130
        wrapMode: Text.Wrap
        textFormat: TextEdit.PlainText
        text: root.sentence.arg("main")
    }
    // Unwrapped: the markup with its two spaces, and the same markup unescaped, which folds them to one.
    TextEdit {
        id: runKept
        wrapMode: Text.NoWrap
        textFormat: TextEdit.RichText
        text: spacedLine.namedMarkup
    }
    TextEdit {
        id: runFolded
        wrapMode: Text.NoWrap
        textFormat: TextEdit.RichText
        text: "The first  commit will start " + Words.codeMarkup("main", Theme.textLink)
    }

    TestCase {
        name: "RefWords"
        when: windowShown

        /// The words on the first line, in the order the engine laid them out.
        function firstLineOf(field) {
            const step = field.contentHeight / field.lineCount
            return field.getText(0, field.positionAt(0, step * 1.5))
        }

        // The line count is the machine's — the font database is not the window's (verify-ui windows.md
        // §Windows での実行・デバッグの罠) — so what is asked is where the break landed.
        function test_the_coloured_line_breaks_where_the_plain_one_does() {
            verify(colouredLine.namedMarkup !== "", "the name is drawn apart")
            verify(coloured.lineCount > 1)
            compare(firstLineOf(coloured), firstLineOf(plain))
            verify(firstLineOf(coloured).endsWith(" "))
        }

        function test_a_run_of_spaces_keeps_its_width() {
            verify(runKept.implicitWidth > runFolded.implicitWidth)
        }

        function test_a_name_that_looks_like_markup_stays_a_name() {
            verify(markupLike.namedMarkup.indexOf("&lt;b&gt;&amp;x") >= 0, markupLike.namedMarkup)
        }

        function test_a_line_with_no_name_asks_for_no_markup() {
            compare(unnamed.namedMarkup, "")
        }

        /// The numbers come with the kind (rules-refs/app-ui.md「Rust に文言を置かない」) and must land in the right
        /// seats — a wrong index reads as `undefined` in the card, which nothing else catches.
        function test_an_avatar_refusal_takes_its_numbers_in_the_order_it_was_given_them() {
            compare(Words.avatarFailure("too-large", ["10"], ""), "This file is larger than 10 MB.")
            compare(Words.avatarFailure("too-many-pixels", ["8000x6000", "32"], ""),
                    "This file is 8000x6000, past the 32 megapixels this can take.")
            verify(Words.avatarFailure("unreadable", [], "").indexOf("undefined") < 0)
            compare(Words.avatarFailure("", [], ""), "")
        }

        /// What the operating system said goes on its own line under ours; the rest quote nobody — the same division a
        /// report makes (デザイン規約 §長さ).
        function test_the_failures_the_system_made_quote_it_and_the_rest_do_not() {
            const said = Words.avatarFailure("read", [], "The system cannot find the file specified. (os error 2)")
            verify(said.indexOf("could not be read") >= 0, said)
            verify(said.indexOf("os error 2") >= 0, said)
            compare(said.split("\n").length, 2, said)
            // The same kind with nothing said leaves no empty second line behind.
            compare(Words.avatarFailure("read", [], "").split("\n").length, 1)
            compare(Words.avatarFailure("no-store", [], "").split("\n").length, 1)
        }
    }
}
