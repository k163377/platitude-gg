import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The middle button's hand, pressed the way a hand presses it (デザイン規約 §中クリックの自動スクロール). The verbs go
// in at `MiddleAutoScroll.press` — a run has no middle button to put down — so **whether a real press reaches the hand,
// and what the hand leaves to what is under it, is Qt's delivery**, and that is what this file holds: a hand over the
// rows that every row, button and word leaves the middle press to, except words being written where the platform
// pastes and a box with a hand of its own; the gesture standing over the rows once it runs; and no hand at all on a
// surface with nowhere to go.
Item {
    id: root
    width: 700
    height: 700

    // A list of the shape every pane's is: rows answering the left and right buttons, words that can be selected, and
    // a box that can be written in (a rename box stands in a row like this).
    Component {
        id: listScene
        AppListView {
            id: list
            property int rowPresses: 0
            property int rows: 60
            width: 300
            height: 240
            model: list.rows
            delegate: Item {
                required property int index
                width: list.width
                height: 30
                MouseArea {
                    width: 140
                    height: parent.height
                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                    onPressed: list.rowPresses++
                }
                TextEdit {
                    x: 140
                    width: 60
                    height: parent.height
                    readOnly: true
                    selectByMouse: true
                    text: "words"
                }
                TextField {
                    x: 210
                    width: 80
                    height: parent.height
                }
            }
        }
    }

    Component {
        id: boxScene
        Item {
            property alias box: box
            width: 320
            height: 150
            DescriptionBox {
                id: box
                anchors.fill: parent
            }
        }
    }

    // A surface that scrolls as one piece with a box of words on it — the right pane's message block, in small.
    Component {
        id: blockScene
        Flickable {
            id: block
            property alias box: box
            property alias hand: hand
            width: 320
            height: 200
            contentWidth: width
            contentHeight: 600
            clip: true
            ScrollBar.vertical: AutoScrollBar {}
            DescriptionBox {
                id: box
                y: 20
                width: 300
                height: 120
            }
            MiddleAutoScroll {
                id: hand
                parent: block
                anchors.fill: parent
                visible: block.ScrollBar.vertical.visible
                onDrifted: dy => block.contentY = Math.max(0, Math.min(block.contentY + dy,
                                                                       block.contentHeight - block.height))
            }
        }
    }

    function longWords() {
        let lines = []
        for (let n = 1; n <= 30; n++)
            lines.push("Line " + n + " of words that run past the box they are written in.")
        return lines.join("\n")
    }
    /// A box holding more words than it shows, standing at its first line: left where the text was assigned, the
    /// caret takes the view to the last one, and a gesture going down from there has nowhere to go.
    function fill(box, readOnly) {
        box.readOnly = readOnly
        box.text = root.longWords()
        box.resetForNewMessage()
    }

    TestCase {
        name: "MiddleHand"
        when: windowShown

        function cleanup() {
            MiddleHand.stop()
        }

        // A middle click anywhere on the rows starts the gesture, and released where it went down it stays latched,
        // standing over the rows — the next click ends it there, and the row under it never hears that click.
        function test_over_the_rows() {
            const list = createTemporaryObject(listScene, root)
            verify(waitForRendering(list))
            mouseClick(list, 70, 45, Qt.MiddleButton)
            verify(list.hand.scrolling, "a middle click on a row did not start the gesture")
            compare(MiddleHand.running, list.hand)
            compare(list.rowPresses, 0)
            mouseClick(list, 70, 75, Qt.LeftButton)
            verify(!list.hand.scrolling, "the click that ends a latched gesture did not end it")
            compare(list.rowPresses, 0, "the click that ended the gesture reached the row under it")
            compare(MiddleHand.running, null)
            mouseClick(list, 70, 75, Qt.LeftButton)
            compare(list.rowPresses, 1, "a click with no gesture standing did not reach the row")
        }

        // Words are the gesture's like everything else, except words being written where the middle button pastes:
        // there the click is the paste, and the box keeps it.
        function test_words_and_the_paste() {
            const list = createTemporaryObject(listScene, root)
            verify(waitForRendering(list))
            mouseClick(list, 170, 45, Qt.MiddleButton)
            verify(list.hand.scrolling, "a middle click on words that are only read did not start the gesture")
            list.hand.stop()
            mouseClick(list, 250, 45, Qt.MiddleButton)
            compare(list.hand.scrolling, !list.hand.middlePastes)
        }

        // A list that holds all of its rows has nowhere to go, and no hand: the press starts nothing.
        function test_no_hand_with_nowhere_to_go() {
            const list = createTemporaryObject(listScene, root, { rows: 3 })
            verify(waitForRendering(list))
            verify(!list.hand.visible, "a list holding all of its rows stood a hand")
            mouseClick(list, 70, 15, Qt.MiddleButton)
            verify(!list.hand.scrolling)
            compare(MiddleHand.running, null)
        }

        // Latched, the gesture sends the list by where the pointer went; the wheel ends it and is still the list's
        // notch.
        function test_the_drift_and_the_wheel() {
            const list = createTemporaryObject(listScene, root)
            verify(waitForRendering(list))
            mouseClick(list, 70, 60, Qt.MiddleButton)
            mouseMove(list, 70, 60 + Metrics.middleScrollDeadZone + 80)
            tryVerify(() => list.contentY > 0, undefined, "the gesture never sent the list")
            const at = list.contentY
            mouseWheel(list, 70, 150, 0, -120)
            verify(!list.hand.scrolling, "the wheel did not end the gesture")
            tryVerify(() => list.contentY > at + 10, undefined, "the notch never reached the list")
        }

        // One gesture in the window: a hand that starts takes the place of whichever was running, and a press that
        // lands anywhere ends the one running — looked at a turn later, so a middle press that starts another gesture
        // is not ended by the press that started it.
        function test_one_gesture_at_a_time() {
            const first = createTemporaryObject(listScene, root)
            const second = createTemporaryObject(listScene, root, { x: 350 })
            verify(waitForRendering(second))
            mouseClick(first, 70, 45, Qt.MiddleButton)
            verify(first.hand.scrolling)
            MiddleHand.pressLanded()
            mouseClick(second, 70, 45, Qt.MiddleButton)
            verify(!first.hand.scrolling, "the first gesture went on beside the second")
            compare(MiddleHand.running, second.hand)
            // The look a turn later has been taken once a later ask of the same kind has: the two are run in the
            // order they were asked.
            let looked = false
            Qt.callLater(() => looked = true)
            tryVerify(() => looked, undefined, "the turn after the press never came")
            verify(second.hand.scrolling, "the press that started the second gesture ended it")
            MiddleHand.pressLanded()
            tryVerify(() => !second.hand.scrolling, undefined, "a press elsewhere left the gesture running")
        }

        // A box's hand stands over its words, and only while they run past the box: a box that holds all of them hands
        // the press back to the words, as it always did.
        function test_the_box_over_its_words() {
            const scene = createTemporaryObject(boxScene, root)
            const box = scene.box
            root.fill(box, true)
            tryVerify(() => box.hand.visible, undefined, "the words ran past the box and its hand never stood")
            tryCompare(box, "textAt", 0)
            mouseClick(box, 150, 70, Qt.MiddleButton)
            verify(box.hand.scrolling, "a middle click on the words did not start the box's gesture")
            verify(!box.focused, "the middle click reached the words under the hand")
            mouseMove(box, 150, 70 + Metrics.middleScrollDeadZone + 60)
            tryVerify(() => box.textAt > 0, undefined, "the gesture never sent the words")
            box.hand.stop()
            box.text = "One line."
            tryVerify(() => !box.hand.visible, undefined, "a box holding all of its words kept its hand")
            mouseClick(box, 150, 20, Qt.MiddleButton)
            verify(box.focused, "a middle click on words that fit did not reach them")
        }

        // Words being written, where the middle button pastes, keep the press: the hand steps aside rather than start.
        function test_the_box_steps_aside_for_a_paste() {
            const scene = createTemporaryObject(boxScene, root)
            const box = scene.box
            root.fill(box, false)
            tryVerify(() => box.hand.visible, undefined, "the words ran past the box and its hand never stood")
            compare(box.hand.claimedAt(150, 70), box.hand.middlePastes)
            compare(box.hand.press(150, 70), !box.hand.middlePastes)
        }

        // A box on a surface that scrolls: the box's words are sent by the box's own hand, which the surface's lets the
        // press go to, and the rest of the surface by the surface's.
        function test_a_box_inside_a_block() {
            const block = createTemporaryObject(blockScene, root)
            root.fill(block.box, true)
            tryVerify(() => block.box.hand.visible && block.hand.visible, undefined, "the two hands never stood")
            mouseClick(block, 150, 80, Qt.MiddleButton)
            verify(block.box.hand.scrolling, "a middle click on the box's words did not start the box's gesture")
            verify(!block.hand.scrolling, "the surface took a press the box's hand was standing for")
            block.box.hand.stop()
            mouseClick(block, 150, 170, Qt.MiddleButton)
            verify(block.hand.scrolling, "a middle click on the surface beside the box did not start its gesture")
        }
    }
}
