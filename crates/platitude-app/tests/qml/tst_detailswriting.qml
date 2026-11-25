import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The commit card shows a message as recorded and takes the cut a save writes once a caret comes in
// (`DetailsPane.startWriting`, デザイン規約 §リポジトリの文は原文で描く). Only the details model stands in: the
// Rust-backed one is not given to these runs (`xtask::qmltest`).
Item {
    id: root
    width: 420
    height: 640

    /// As much of `DetailsModel` as the card reads: one commit whose recorded message differs from its saved cut.
    QtObject {
        id: model
        property string shaHex: "a1"
        property string messageSubject: "  indented subject"
        property string messageBody: "The body.\n\n"
        property string editSubject: "indented subject"
        property string editBody: "The body."
        property string authorName: "Ada Lovelace"
        property string authorEmail: "ada@example.com"
        property string authorTime: "2026-11-25T00:54:00+09:00"
        property string sha8: "a1a1a1a1"
        property string parentHex: ""
        property var parentHexes: []
        property int avatar: 0
        property string avatarUrl: ""
        property var coAuthors: []
        property string committerName: "Ada Lovelace"
        property string committerEmail: "ada@example.com"
        property int selectionCount: 1
        property bool comparing: false
        property bool loading: false
        property int floorRows: 0
        property int fileTotal: 0
        property bool treeView: false
        function setTreeView(on) {}
        function toggleFolder(path) {}
        function rowOfFileIn(path, bucket) { return -1 }
    }

    DetailsPane {
        id: pane
        anchors.fill: parent
        details: model
        editable: true
    }

    TestCase {
        name: "DetailsWriting"
        when: windowShown

        /// Every item under `item` with the property `tell` — walked rather than reached through a handle the product
        /// would carry for this file alone.
        function drawn(item, tell) {
            let found = item[tell] === undefined ? [] : [item]
            const kids = item.children
            if (kids !== undefined)
                for (let i = 0; i < kids.length; i++)
                    found = found.concat(drawn(kids[i], tell))
            return found
        }
        function block() {
            const blocks = drawn(pane, "subjectText").filter(found => found.writing !== undefined)
            compare(blocks.length, 1, "one message block")
            return blocks[0]
        }
        /// The subject's own field, found by what it holds.
        function subjectField(text) {
            const fields = drawn(pane, "cursorPosition").filter(found => found.text === text)
            verify(fields.length > 0, "a field holding " + JSON.stringify(text))
            return fields[0]
        }

        /// The caret out, then the commit's own message back in the boxes, as a commit read anew puts it.
        function init() {
            pane.forceActiveFocus()
            tryVerify(() => !block().writing)
            pane.fillMessage(false)
        }

        function test_a_message_rests_as_recorded() {
            compare(block().subjectText, "  indented subject")
            compare(block().bodyText, "The body.\n\n")
            verify(!pane.messageDirty)
        }

        /// The caret coming in is what turns it: the boxes take the cut, and the difference is the edit's.
        function test_a_caret_coming_in_takes_the_cut_a_save_writes() {
            subjectField("  indented subject").forceActiveFocus()
            tryVerify(() => block().writing)
            compare(block().subjectText, "indented subject")
            compare(block().bodyText, "The body.")
            verify(pane.messageDirty, "the save row stands for the cut, nothing typed")
        }

        /// Typed words are the reader's: the caret going out and back leaves them where they are.
        function test_typed_words_are_not_cut_again() {
            subjectField("  indented subject").forceActiveFocus()
            tryVerify(() => block().writing)
            block().setTexts("  my own words", "kept\n")
            pane.forceActiveFocus()
            tryVerify(() => !block().writing)
            subjectField("  my own words").forceActiveFocus()
            tryVerify(() => block().writing)
            compare(block().subjectText, "  my own words")
            compare(block().bodyText, "kept\n")
        }
    }
}
