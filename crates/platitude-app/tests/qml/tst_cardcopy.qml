import QtQuick
import QtTest
import platitude.ui

// What a copy out of a card's line carries where a name wears its mark in a room the field opens (`CardText.marks`):
// the words as the sentence reads them and nothing for the mark — not the spaces that make its room (デザイン規約
// §アイコンの席「コピー」) — and a name the heading cut (`holdsNames`) as it is drawn. Selections are made through the
// card's own drag, as a hand makes them.
Item {
    id: root
    width: 1400
    height: 200

    readonly property string sentence: "Checked out in another worktree — topic"
    readonly property int at: root.sentence.indexOf("topic")
    /// How many pinned spaces the mark's room takes, once its drawing has said how wide it is.
    readonly property int room: card.markRooms.length > 0 ? card.markRooms[0] : 0
    readonly property string farName:
        "bugfix/keep-the-long-branch-names-readable-in-every-menu-card-tooltip-question-bar-notice-and-the-button"

    CardText {
        id: card
        width: 600
        text: root.sentence
        marks: [[root.at, "tree", undefined, "topic".length]]
    }
    CardText {
        id: held
        y: 60
        width: 1300
        text: "Deleted " + root.farName
        marks: Words.marksOf(name => "Deleted " + name, [[root.farName, "branch"]])
        holdsNames: true
    }
    /// Where the clipboard is read back.
    TextEdit {
        id: sink
        y: 150
        width: 600
    }

    TestCase {
        name: "CardCopy"
        when: windowShown

        function init() {
            tryVerify(() => root.room > 0, undefined, "the mark's room is open")
        }

        /// Selects from one of the field's characters to another, as a drag between them would.
        function select(field, from, to) {
            const a = field.charRect(from)
            const b = field.charRect(to)
            field.anchorFrom(field, a.x, a.y + a.height / 2)
            field.extendFrom(field, b.x, b.y + b.height / 2)
        }

        function test_a_whole_line_goes_out_as_its_words() {
            card.selectAll()
            compare(card.copiedText(), root.sentence)
        }

        function test_a_selection_across_the_room_leaves_it_out() {
            const from = root.sentence.indexOf("another")
            select(card, from, root.sentence.length + root.room)
            compare(card.copiedText(), root.sentence.substring(from))
        }

        function test_the_name_after_the_room_goes_out_alone() {
            select(card, root.at + root.room, root.sentence.length + root.room)
            compare(card.copiedText(), "topic")
        }

        function test_a_selection_inside_the_room_carries_nothing() {
            select(card, root.at + 1, root.at + root.room)
            compare(card.copiedText(), "")
        }

        /// The key a reader presses goes the same way: what lands on the clipboard is the sentence, not its spaces.
        function test_ctrl_c_puts_the_words_on_the_clipboard() {
            select(card, 0, root.sentence.length + root.room)
            keySequence(StandardKey.Copy)
            sink.text = ""
            sink.paste()
            compare(sink.text, root.sentence)
        }

        /// A heading's cut name goes out as it is drawn — the whole one is the tip's, under the hand.
        function test_a_cut_name_goes_out_as_drawn() {
            tryVerify(() => held.held.cuts.length === 1, undefined, "the name is cut")
            held.selectAll()
            compare(held.copiedText(), held.held.text)
            verify(held.copiedText().indexOf("…") > 0, "cut: " + held.copiedText())
        }
    }
}
