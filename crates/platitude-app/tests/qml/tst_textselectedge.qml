import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// A drag of the diff's or the log's text near the frame's edge sends the rows after it the way a browser sends a
// selection (デザイン規約 §右のペインの字は掴める): from a belt `Metrics.selectScrollBelt` inside each edge, the
// distance past the belt's line at `Metrics.selectScrollGain` — so a frame standing against the screen's edge, which
// the hand cannot get past, still sends. Each animation frame of the edge's ticker asks the pane for the milliseconds
// since the frame before. Pressed and dragged the way a hand does, through Qt's delivery, so the frame's own
// `MouseArea` is what follows the hand.
//
// The ticker is found in the hand's own tree (`tickerOf`): the amount is per frame, and the frame's length is the one
// the ticker read (`frameTime`), not one the test assumes.
Item {
    id: root
    width: 900
    height: 900

    /// The rows a hand stands over, many frames of them: a press finds a row, and a drag to either edge has rows to
    /// send once the list is in the middle.
    readonly property int rows: 200
    readonly property int rowH: 20

    /// What the diff's model answers a hand: every press lands, nothing it is asked about is a hunk's heading.
    Component {
        id: diffModelStub
        QtObject {
            property bool selHasNew: true
            property int selRemoved: 0
            function beginSelect(side, row, at) {}
            function extendSelect(side, row, at) {}
            function clearSelect() {}
            function sourceByteAt(side, row, place) { return place }
            function selectionHolds(side, row, at) { return false }
            function selectRow(side, row) {}
            function selectionEnd() { return undefined }
        }
    }
    /// What the log's model answers a hand: one column's words on every row.
    Component {
        id: commandsModelStub
        QtObject {
            function beginSelect(row, at) {}
            function extendSelect(row, at) {}
            function selectionText() { return "words" }
            function clearSelect() {}
            function hitAt(row, at, place) { return place }
            function columnText(row, at) { return "words" }
        }
    }

    /// The diff's hand over its rows, as `DiffPane` lays it: over the list, past the gutter, one column.
    Component {
        id: diffScene
        Item {
            id: scene
            property alias list: list
            property alias pick: pick
            anchors.fill: parent
            LineRuler {
                id: ruler
                font.pixelSize: Theme.fontSm
            }
            ListView {
                id: list
                x: 250
                y: 300
                width: 360
                height: 240
                clip: true
                model: root.rows
                delegate: Item {
                    required property int index
                    readonly property string kind: "line"
                    readonly property bool banded: false
                    width: list.width
                    height: root.rowH
                    function textOf(side) { return "the words of a line of the diff" }
                    function boldOf(side) { return false }
                }
            }
            DiffTextSelect {
                id: pick
                view: list
                diffModel: diffModelStub.createObject(scene)
                gutterW: 40
                split: false
                halfW: list.width
                barRoom: 0
                codeX: 0
                ruler: ruler
            }
        }
    }
    /// The log's hand over its rows, as `CommandsPane` lays it.
    Component {
        id: commandsScene
        Item {
            id: scene
            property alias list: list
            property alias pick: pick
            anchors.fill: parent
            LineRuler {
                id: ruler
                textFormat: TextEdit.PlainText
                font.pixelSize: Theme.fontSm
            }
            ListView {
                id: list
                x: 250
                y: 300
                width: 360
                height: 240
                clip: true
                model: root.rows
                delegate: Item {
                    required property int index
                    readonly property real lineHeight: root.rowH
                    readonly property real clockX: 0
                    readonly property real clockEnd: 60
                    readonly property real cmdX: 70
                    readonly property real cmdEnd: 220
                    readonly property real outX: 230
                    width: list.width
                    height: root.rowH
                }
            }
            CommandsTextSelect {
                id: pick
                view: list
                commandsModel: commandsModelStub.createObject(scene)
                barRoom: 0
                ruler: ruler
            }
        }
    }

    SignalSpy {
        id: sent
        signalName: "scrollWanted"
    }

    TestCase {
        name: "TextSelectEdge"
        when: windowShown

        /// The hand's edge ticker, read out of its tree.
        function tickerOf(pick) {
            for (let i = 0; i < pick.data.length; i++) {
                if (pick.data[i] instanceof FrameAnimation)
                    return pick.data[i]
            }
            return null
        }

        /// One of the two hands, its list sent to the middle of its rows so both edges have rows past them.
        function built(scene) {
            const made = createTemporaryObject(scene, root)
            verify(made !== null, "the scene was built")
            made.list.contentY = (root.rows * root.rowH - made.list.height) / 2
            sent.clear()
            sent.target = made.pick
            verify(tickerOf(made.pick) !== null, "the hand has an edge ticker")
            return made
        }

        /// Where the hand starts: a row in the middle of the frame, left of the middle of its words, so every move
        /// below lands on another place.
        function start(pick) {
            return Qt.point(pick.width / 4, pick.height / 2 + root.rowH / 2)
        }

        /// Pressed on the text and taken to `at` (the hand's own coordinates), held there until the ticker has asked
        /// twice — the first ask could be one the move itself raced. The last ask, with the milliseconds of the frame
        /// that asked it (read in the same turn, before another frame can come).
        function heldAt(pick, at) {
            const from = start(pick)
            mousePress(pick, from.x, from.y, Qt.LeftButton)
            verify(pick.dragging, "the press on the text started a drag")
            mouseMove(pick, at.x, at.y, -1, Qt.LeftButton)
            tryVerify(function () { return sent.count >= 2 }, undefined, "the ticker asked " + sent.count + " times")
            const asked = sent.signalArguments[sent.count - 1]
            const ms = tickerOf(pick).frameTime * 1000
            mouseRelease(pick, at.x, at.y, Qt.LeftButton)
            return { "y": asked[0], "x": asked[1], "ms": ms }
        }

        /// How far past the belt's line the hand is held, signed toward the edge it is near: inside the belt, on the
        /// frame's edge, and past it by the room a 48px taskbar leaves under a maximised window.
        function reaches() {
            const belt = Metrics.selectScrollBelt
            return [
                { tag: "inside the belt", past: belt - 15 },
                { tag: "on the edge", past: belt },
                { tag: "past the edge", past: belt + 47 },
            ]
        }
        /// Each reach at both ends of an axis.
        function bothEnds() {
            const out = []
            for (const reach of reaches()) {
                out.push({ tag: "after, " + reach.tag, past: reach.past })
                out.push({ tag: "before, " + reach.tag, past: -reach.past })
            }
            return out
        }
        /// The point `past` beyond one of the belt's lines of a frame `length` long, along one axis.
        function along(length, past) {
            const belt = Metrics.selectScrollBelt
            return past > 0 ? length - belt + past : belt + past
        }
        /// What `ms` milliseconds send a hand `past` beyond the belt's line.
        function sentIn(past, ms) {
            return past * Metrics.selectScrollGain * ms
        }

        function test_the_log_is_sent_up_and_down_from_its_belts_data() { return bothEnds() }

        function test_the_log_is_sent_up_and_down_from_its_belts(data) {
            const made = built(commandsScene)
            const asked = heldAt(made.pick, Qt.point(start(made.pick).x + 40, along(made.pick.height, data.past)))
            verify(asked.ms > 0, "the frame that asked took time")
            fuzzyCompare(asked.y, sentIn(data.past, asked.ms), 1e-9,
                         "held " + data.past + "px past the belt's line, a frame of " + asked.ms + "ms")
        }

        function test_the_diff_is_sent_up_and_down_from_its_belts_data() { return bothEnds() }

        function test_the_diff_is_sent_up_and_down_from_its_belts(data) {
            const made = built(diffScene)
            const asked = heldAt(made.pick, Qt.point(start(made.pick).x + 40, along(made.pick.height, data.past)))
            verify(asked.ms > 0, "the frame that asked took time")
            fuzzyCompare(asked.y, sentIn(data.past, asked.ms), 1e-9,
                         "held " + data.past + "px past the belt's line, a frame of " + asked.ms + "ms")
            compare(asked.x, 0, "between the side belts nothing goes sideways")
        }

        function test_the_diff_is_sent_sideways_from_its_belts_data() { return bothEnds() }

        /// The text column's own edges: left of the code is still the diff's hand (the gutter is outside it).
        function test_the_diff_is_sent_sideways_from_its_belts(data) {
            const made = built(diffScene)
            const asked = heldAt(made.pick, Qt.point(along(made.pick.width, data.past), start(made.pick).y + root.rowH))
            verify(asked.ms > 0, "the frame that asked took time")
            fuzzyCompare(asked.x, sentIn(data.past, asked.ms), 1e-9,
                         "held " + data.past + "px past the belt's line, a frame of " + asked.ms + "ms")
            compare(asked.y, 0, "between the top and bottom belts nothing goes up or down")
        }

        /// A corner is in two belts, and each axis is sent on its own.
        function test_the_diff_is_sent_on_each_axis_apart() {
            const made = built(diffScene)
            const down = Metrics.selectScrollBelt + 47
            const right = 5
            const asked = heldAt(made.pick, Qt.point(along(made.pick.width, right), along(made.pick.height, down)))
            fuzzyCompare(asked.y, sentIn(down, asked.ms), 1e-9, "down")
            fuzzyCompare(asked.x, sentIn(right, asked.ms), 1e-9, "sideways")
        }

        function test_between_the_belts_nothing_is_sent_data() {
            return [ { tag: "the log", scene: commandsScene }, { tag: "the diff", scene: diffScene } ]
        }

        /// A drag that stays between the belts selects without sending — the belt's line is where it starts.
        function test_between_the_belts_nothing_is_sent(data) {
            const made = built(data.scene)
            const belt = Metrics.selectScrollBelt
            const from = start(made.pick)
            mousePress(made.pick, from.x, from.y, Qt.LeftButton)
            for (const y of [belt + 1, made.pick.height - belt - 1]) {
                mouseMove(made.pick, from.x + 40, y, -1, Qt.LeftButton)
                verify(!tickerOf(made.pick).running, "the ticker runs with the hand at " + y + " of "
                       + made.pick.height)
            }
            mouseRelease(made.pick, from.x + 40, made.pick.height - belt - 1, Qt.LeftButton)
            compare(sent.count, 0)
        }

        function test_a_press_in_the_belt_sends_nothing_until_it_takes_text_data() {
            return [ { tag: "the log", scene: commandsScene }, { tag: "the diff", scene: diffScene } ]
        }

        /// The browser starts its selection's autoscroll only once the selection holds something: a click on the
        /// last row in sight — most of it is the belt — does not send the rows away under the hand.
        function test_a_press_in_the_belt_sends_nothing_until_it_takes_text(data) {
            const made = built(data.scene)
            const x = start(made.pick).x
            const y = made.pick.height - 2
            mousePress(made.pick, x, y, Qt.LeftButton)
            verify(made.pick.dragging, "the press on the text started a drag")
            verify(!tickerOf(made.pick).running, "the ticker runs on a press alone")
            mouseMove(made.pick, x + 40, y, -1, Qt.LeftButton)
            verify(tickerOf(made.pick).running, "the ticker stands still once the drag has taken text in the belt")
            mouseRelease(made.pick, x + 40, y, Qt.LeftButton)
        }
    }
}
