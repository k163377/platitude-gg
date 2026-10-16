import QtQuick
import QtTest
import platitude.ui

// What a copy out of a card's line carries where the markup opened a room for a mark (`CardText.roomAt`,
// `Words.roomInSentence`): the words as the sentence reads them and nothing for the mark — not the spaces that make
// its room (デザイン規約 §アイコンの席「コピー」). Selections are made through the card's own drag, as a hand makes them.
Item {
    id: root
    width: 700
    height: 100

    readonly property string sentence: "Checked out in another worktree — topic"
    readonly property int at: root.sentence.indexOf("topic")
    readonly property int room: 3

    CardText {
        id: card
        width: 600
        text: root.sentence
        markup: Words.roomInSentence(root.sentence, root.at, root.room)
        roomAt: root.at
        roomLength: root.room
    }
    /// Where the clipboard is read back.
    TextEdit {
        id: sink
        y: 50
        width: 600
    }

    TestCase {
        name: "CardCopy"
        when: windowShown

        /// Selects from one of the field's characters to another, as a drag between them would.
        function select(from, to) {
            const a = card.charRect(from)
            const b = card.charRect(to)
            card.anchorFrom(card, a.x, a.y + a.height / 2)
            card.extendFrom(card, b.x, b.y + b.height / 2)
        }

        function test_a_whole_line_goes_out_as_its_words() {
            card.selectAll()
            compare(card.copiedText(), root.sentence)
        }

        function test_a_selection_across_the_room_leaves_it_out() {
            const from = root.sentence.indexOf("another")
            select(from, root.sentence.length + root.room)
            compare(card.copiedText(), root.sentence.substring(from))
        }

        function test_the_name_after_the_room_goes_out_alone() {
            select(root.at + root.room, root.sentence.length + root.room)
            compare(card.copiedText(), "topic")
        }

        function test_a_selection_inside_the_room_carries_nothing() {
            select(root.at + 1, root.at + root.room)
            compare(card.copiedText(), "")
        }

        /// The key a reader presses goes the same way: what lands on the clipboard is the sentence, not its spaces.
        function test_ctrl_c_puts_the_words_on_the_clipboard() {
            select(0, root.sentence.length + root.room)
            keySequence(StandardKey.Copy)
            sink.text = ""
            sink.paste()
            compare(sink.text, root.sentence)
        }
    }
}
