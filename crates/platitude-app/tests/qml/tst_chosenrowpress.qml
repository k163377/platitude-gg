import QtQuick
import QtTest
import platitude.ui

// A row of the choice's list (`ChosenCommitRow`) under a real pointer. A press goes to one of four hands — the drop
// hand over everything takes a Ctrl one, the hash plate keeps its own, the hand over the summary and the pad under the
// row take a plain one — and only a pointer can say which one Qt hands it to: the headless verbs enter the hands'
// functions. A plain press let go where it went down, on the words or in the air, finds the commit on the graph; one
// that travels drags over the words and finds nothing; a Ctrl click takes the commit out
// (デザイン規約 §複数のコミットを選ぶ / §右のペインの字は掴める).
Item {
    id: root
    width: 520
    height: 200

    property int shows: 0
    property string shown: ""
    property int drops: 0
    property int copies: 0
    property string copied: ""
    readonly property var record: ({ "oid": "5e1f0a2bccccddddeeeeffff0000111122223333", "sha8": "5e1f0a2b",
                                     "subject": "feat: a summary the hand clicks on", "body": "",
                                     "author": "someone", "authorTime": 1700000000, "avatar": 0, "avatarUrl": "",
                                     "mates": "", "laneAbove": "", "laneBelow": "solid" })

    // A list as the pane's (`DetailsPane.chosenList`): the presses reach the row through a view, as they do there. The
    // row goes in once the window is up, as a choice puts it in the app: built in the window's own creation, the hash
    // plate's arrow says a binding loop on its width (`HashPlate.parentBack`).
    ListModel {
        id: rows
    }
    ListView {
        id: list
        x: 20
        y: 30
        width: 480
        height: Theme.graphRowHeight * 3
        model: rows
        delegate: ChosenCommitRow {
            onShowRequested: oidHex => {
                root.shows++
                root.shown = oidHex
            }
            onDropRequested: oidHex => root.drops++
            onCopyRequested: text => {
                root.copies++
                root.copied = text
            }
        }
    }

    TestCase {
        name: "ChosenRowPress"
        when: windowShown

        function row() {
            return list.itemAtIndex(0)
        }
        /// The row's value field showing `text`.
        function field(text) {
            const fields = row().sweep.fields()
            for (let i = 0; i < fields.length; i++) {
                if (fields[i].text === text)
                    return fields[i]
            }
            return null
        }
        /// A point of `item` in the root's coordinates, in whole pixels, as a pointer delivers them.
        function inRoot(item, x, y) {
            const p = item.mapToItem(root, x, y)
            return Qt.point(Math.round(p.x), Math.round(p.y))
        }
        /// On the summary's words, `x` along them.
        function onWords(x) {
            const words = field(root.record.subject)
            return inRoot(words, x, words.height / 2)
        }
        /// The row's air under the summary: below its box, above the row's foot.
        function underWords(x) {
            const words = field(root.record.subject)
            const foot = words.mapToItem(row(), 0, words.height).y
            return inRoot(row(), words.mapToItem(row(), x, 0).x, (foot + row().height) / 2)
        }

        function initTestCase() {
            rows.append(root.record)
        }

        function init() {
            tryVerify(() => row() !== null && field(root.record.subject) !== null
                      && field(root.record.subject).width > 0, undefined, "the row stands with its words")
            row().sweep.dropValues()
            root.shows = 0
            root.shown = ""
            root.drops = 0
            root.copies = 0
            root.copied = ""
        }

        function test_a_click_on_the_summary_finds_the_commit() {
            const p = onWords(20)
            mouseClick(root, p.x, p.y)
            compare(root.shows, 1, "the click on the words is the row's")
            compare(root.shown, root.record.oid)
            compare(root.drops, 0)
            compare(field(root.record.subject).selected, "", "and it took no words")
        }

        function test_a_click_in_the_air_finds_the_commit() {
            const below = underWords(20)
            verify(below.y > onWords(20).y, "the point is under the words")
            mouseClick(root, below.x, below.y)
            const left = inRoot(row(), 1, row().height / 2)
            mouseClick(root, left.x, left.y)
            compare(root.shows, 2, "a click anywhere in the row's air is the row's")
            compare(root.drops, 0)
        }

        // Jitter inside the platform's drag distance is still a click, or a shaky hand would find nothing.
        function test_a_press_that_barely_moves_is_still_a_click() {
            const p = underWords(20)
            mousePress(root, p.x, p.y)
            mouseMove(root, p.x + 1, p.y + 1)
            mouseRelease(root, p.x + 1, p.y + 1)
            compare(root.shows, 1)
            compare(field(root.record.subject).selected, "")
        }

        function test_a_drag_over_the_summary_takes_its_words_and_finds_nothing() {
            const from = onWords(1)
            const to = onWords(field(root.record.subject).width - 1)
            mousePress(root, from.x, from.y)
            mouseMove(root, (from.x + to.x) / 2, from.y)
            mouseMove(root, to.x, to.y)
            mouseRelease(root, to.x, to.y)
            compare(field(root.record.subject).selected, root.record.subject, "the drag took the words it crossed")
            compare(root.shows, 0, "a drag is not a click")
        }

        function test_a_drag_from_the_air_takes_the_words_and_finds_nothing() {
            const from = underWords(1)
            const to = underWords(field(root.record.subject).width - 1)
            mousePress(root, from.x, from.y)
            mouseMove(root, (from.x + to.x) / 2, from.y)
            mouseMove(root, to.x, to.y)
            mouseRelease(root, to.x, to.y)
            compare(field(root.record.subject).selected, root.record.subject, "the drag took the nearest words")
            compare(root.shows, 0, "a drag is not a click")
        }

        // A click after a drag drops what the drag took: one selection in the window, and a click takes no words.
        function test_a_click_after_a_drag_drops_the_words() {
            test_a_drag_over_the_summary_takes_its_words_and_finds_nothing()
            const p = underWords(20)
            mouseClick(root, p.x, p.y)
            compare(field(root.record.subject).selected, "")
            compare(root.shows, 1)
        }

        function test_a_ctrl_click_on_the_summary_takes_the_commit_out_and_finds_nothing() {
            const p = onWords(20)
            mouseClick(root, p.x, p.y, Qt.LeftButton, Qt.ControlModifier)
            compare(root.drops, 1)
            compare(root.shows, 0)
        }

        function test_a_ctrl_click_in_the_air_takes_the_commit_out_and_finds_nothing() {
            const p = underWords(20)
            mouseClick(root, p.x, p.y, Qt.LeftButton, Qt.ControlModifier)
            compare(root.drops, 1)
            compare(root.shows, 0)
        }

        // The press decides which hand has it, and so what its click is: a key going down or up before the release
        // changes neither.
        function test_ctrl_held_down_after_a_plain_press_still_finds_the_commit() {
            const p = underWords(20)
            mousePress(root, p.x, p.y)
            mouseRelease(root, p.x, p.y, Qt.LeftButton, Qt.ControlModifier)
            compare(root.shows, 1)
            compare(root.drops, 0)
        }

        function test_ctrl_let_go_before_the_release_still_takes_the_commit_out() {
            const p = onWords(20)
            mousePress(root, p.x, p.y, Qt.LeftButton, Qt.ControlModifier)
            mouseRelease(root, p.x, p.y, Qt.LeftButton, Qt.NoModifier)
            compare(root.drops, 1)
            compare(root.shows, 0)
        }

        function test_a_click_on_the_hash_copies_it_and_finds_nothing() {
            const hash = field(root.record.sha8)
            const p = inRoot(hash, hash.width / 2, hash.height / 2)
            mouseClick(root, p.x, p.y)
            compare(root.copies, 1, "the plate keeps its own click")
            compare(root.copied, root.record.oid)
            compare(root.shows, 0)
        }
    }
}
