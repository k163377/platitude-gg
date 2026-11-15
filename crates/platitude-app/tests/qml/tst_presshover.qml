import QtQuick
import QtTest
import platitude.ui
import platitude.ui.input

// What hover does while a press holds the window, and after it (デザイン規約 §hover のツールチップ「押している間は何も
// 開かない」): nothing it asks for opens from the press until the hand's own next move after the release, a drag takes
// down what is out, and a click answered in the box's words opens it at once.
//
// Only a real pointer answers this: Qt delivers no hover while a press holds its grab, and hands it at the release to
// whatever stands under the hand (rules-refs/app-ui.md「押下が掴んでいる間、窓に hover は届かない」). The window's
// parts as `Main` lays them: the press watcher telling `Hand` (`FocusRelease.tellsHand`), and the window's hand, whose
// moves let the hold go (`Hand.heard`).
Item {
    id: root
    width: 400
    height: 400

    HandWatch { id: hand }
    Connections {
        target: hand
        function onMoved(at) {
            Hand.heard(at)
        }
    }

    TipSeat {
        id: shared
        host: root
        hand: hand
    }

    // Two rows that put a box out on a rest, under one hand that takes their presses — the log's rows under the hand
    // that picks their text (`CommandRowDelegate`, `CommandsTextSelect`).
    Column {
        Repeater {
            id: rows
            model: 2
            delegate: Item {
                id: row
                required property int index
                readonly property alias area: rowHover
                width: 200
                height: 40
                MouseArea {
                    id: rowHover
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.NoButton
                    readonly property TipAsk rowTip: TipAsk {
                        target: rowHover
                        asked: rowHover.containsMouse
                        words: "row " + row.index
                    }
                }
            }
        }
    }
    MouseArea {
        id: pick
        width: 200
        height: 80
        acceptedButtons: Qt.LeftButton
    }

    // A target that answers its click in its box's words (`HashPlate`'s copy).
    property bool copied: false
    MouseArea {
        id: copier
        y: 200
        width: 200
        height: 40
        hoverEnabled: true
        onContainsMouseChanged: root.copied = false
        onClicked: {
            root.copied = true
            copyTip.answer()
        }
        readonly property TipAsk copyTip: TipAsk {
            id: copyTip
            target: copier
            asked: copier.containsMouse
            words: root.copied ? "Copied!" : "Copy"
        }
    }

    // A box that answers typing, not a hand (`NavNameBox`'s refusal).
    property bool refusedNow: false
    Item {
        id: typedBox
        y: 300
        width: 200
        height: 40
        readonly property TipAsk typedTip: TipAsk {
            target: typedBox
            asked: root.refusedNow
            typed: true
            words: "refused"
        }
    }

    // A seat that opens the moment it is asked (`expands`, the author cards): the hover Qt hands at the release may ask
    // before the press is told let go.
    readonly property HoverSeat atOnce: HoverSeat {
        id: atOnce
        expands: true
    }

    FocusRelease {
        window: root.Window.window
        tellsHand: true
    }

    TestCase {
        name: "PressHover"
        when: windowShown

        /// A press a case took and did not get to let go of — a failed check leaves the button down for the next case.
        property bool holding: false

        function init() {
            // Two places in a corner nothing reaches: the second is a move whatever the case before let go on.
            mouseMove(root, root.width - 2, root.height - 2)
            mouseMove(root, root.width - 1, root.height - 1)
            root.refusedNow = false
            atOnce.takeDown("init", false)
            tryVerify(() => !shared.box.visible && !shared.settling, undefined, "each case starts with nothing out")
            verify(!Hand.holdsHover, "and with hover not held back")
        }
        function cleanup() {
            if (holding)
                mouseRelease(root, root.width - 1, root.height - 1)
            holding = false
        }

        function row(i) {
            return rows.itemAt(i).area
        }
        /// The box is out, standing on `target`.
        function tipOn(target) {
            return shared.box.visible && shared.shownNow && shared.current !== null && shared.current.target === target
        }

        // The case this was found by, held still: the rest the row was counting goes with the press.
        function test_a_a_press_counts_no_rest() {
            mouseMove(pick, 100, 20)
            mousePress(pick, 100, 20)
            holding = true
            // waits(paced): the subject is a box that must **not** come out, so the wait is the rest it was counting.
            wait(Metrics.tipDelayMs * 2)
            verify(!shared.box.visible, "the rest counting at the press opens nothing while the press holds")

            mouseRelease(pick, 100, 20)
            holding = false
            // waits(paced): the subject is a box that must **not** come out, so the wait is the rest it would count.
            wait(Metrics.tipDelayMs * 2)
            verify(!shared.box.visible, "nor once let go, under the hand that has not moved")

            mouseMove(pick, 102, 20)
            tryVerify(() => tipOn(row(0)), undefined, "the next move asks again, and the box comes out on its rest")
        }

        // The case this was found by: text taken across the rows, the hand stopped on another row.
        function test_b_a_drag_takes_down_what_is_out_and_its_release_opens_nothing() {
            mouseMove(pick, 100, 20)
            tryVerify(() => tipOn(row(0)), undefined, "the row under the resting hand puts its box out")
            mousePress(pick, 100, 20)
            holding = true
            verify(tipOn(row(0)), "a press alone leaves the box that is out")

            mouseMove(pick, 100, 60, -1, Qt.LeftButton)
            verify(!shared.box.visible, "the box goes the moment the press becomes a drag")
            // waits(paced): the subject is a box that must **not** come out, so the wait is the rest a row would count.
            wait(Metrics.tipDelayMs * 2)
            verify(!shared.box.visible, "nothing comes out while the drag holds")

            mouseRelease(pick, 100, 60)
            holding = false
            // waits(paced): the subject is a box that must **not** come out, so the wait is the rest a row would count.
            wait(Metrics.tipDelayMs * 2)
            verify(!shared.box.visible, "the release opens nothing on the row it let go on")

            mouseMove(pick, 102, 60)
            tryVerify(() => tipOn(row(1)), undefined, "the hand's next move asks for the row it is on")
        }

        // A click before the box's rest is over: the answer is to the click, so it comes out with it.
        function test_c_a_click_answered_in_the_box_opens_it_at_once() {
            mouseMove(copier, 100, 20)
            mouseClick(copier, 100, 20)
            verify(shared.shownNow && shared.current === copyTip, "the answer comes out with the click")
            tryVerify(() => tipOn(copier), undefined, "and is drawn")
            compare(shared.box.text, "Copied!")
        }

        // The box out before the click stays out, and takes the answer's words.
        function test_d_a_click_on_a_box_that_is_out_keeps_it() {
            mouseMove(copier, 100, 20)
            tryVerify(() => tipOn(copier) && shared.box.text === "Copy", undefined, "the box offers the copy")
            mouseClick(copier, 100, 20)
            verify(tipOn(copier), "the box stays out through the click")
            compare(shared.box.text, "Copied!")
        }

        // What no hover opened is not the drag's to take down.
        function test_e_a_box_that_answers_typing_stands_through_a_drag() {
            root.refusedNow = true
            tryVerify(() => tipOn(typedBox), undefined, "the refusal is out")
            mousePress(pick, 100, 20)
            holding = true
            mouseMove(pick, 100, 60, -1, Qt.LeftButton)
            verify(tipOn(typedBox), "the drag leaves the refusal standing")
            mouseRelease(pick, 100, 60)
            holding = false
            verify(tipOn(typedBox), "and so does the release")
        }

        // The release is one step: the hold goes from the press to the still hand without a gap an ask kept meanwhile
        // could open through.
        function test_f_an_ask_kept_through_the_release_waits_for_the_move() {
            mousePress(pick, 100, 20)
            holding = true
            atOnce.point({ "key": "card" }, true)
            verify(!atOnce.shown, "an ask made while the press holds is kept")
            mouseRelease(pick, 100, 20)
            holding = false
            verify(!atOnce.shown, "and kept once let go, under the hand that has not moved")
            mouseMove(pick, 102, 20)
            verify(atOnce.shown, "the hand's next move opens it")
        }
    }
}
