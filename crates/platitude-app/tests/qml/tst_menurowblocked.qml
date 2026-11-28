import QtQuick
import QtTest
import platitude.ui

// A confirmed menu row that is blocked (`AppMenuItem.blocked` — the menu's held reason arriving, git going busy) while
// presses are already on it answers as a row blocked from the start: the count blanks and `held` is not raised.
Item {
    id: root
    width: 360
    height: 120

    readonly property int presses: Metrics.holdPressesWarned
    /// A driver to ask which keys stand in for a press (`HoldDriver.holdKey`): `plainRow` builds none of its own.
    readonly property HoldDriver gatekeeper: HoldDriver {}

    AppMenuItem {
        id: row
        width: 340
        text: "Delete branch"
        holdPresses: root.presses
    }
    Item { id: elsewhere }

    // A row with no press count builds no `HoldDriver` to ask, so while blocked it names the keys it drops itself
    // (`AppMenuItem`'s `Keys.onPressed`): held to the driver's own set here. A key it does not drop goes one of two
    // ways — the `MenuItem` under it takes Space, Return and Enter as a press of its own, and anything else passes
    // to the seat (`passed`).
    FocusScope {
        id: plainSeat
        y: 60
        width: 340
        height: plainRow.height

        property var passed: []

        Keys.onPressed: event => plainSeat.passed.push(event.key)
        AppMenuItem {
            id: plainRow
            width: 340
            text: "Open"
            blockedReason: "Git is busy"
        }
    }

    SignalSpy {
        id: holds
        target: row
        signalName: "held"
    }

    TestCase {
        name: "MenuRowBlocked"
        when: windowShown

        function init() {
            row.blockedReason = ""
            row.doable = true
            row.offered = true
            row.note = ""
            row.warning = ""
            elsewhere.forceActiveFocus()
            keyRelease(Qt.Key_Space)
            tryVerify(() => !row.gesturing)
            holds.clear()
        }

        /// Longer than the floor, so a last press that was going to land has landed.
        function outlastTheHold() {
            // waits(paced): the subject is a row that must **not** land, so the wait is the floor it would land after.
            wait(Metrics.holdFloorMs * 2)
        }

        /// A row's tag: its note is part of what it says and stays on a greyed row; its warning is of a press, so a
        /// greyed row drops it, whatever greys it (デザイン規約 §無効).
        function test_a_greyed_row_keeps_its_note_and_warns_of_nothing() {
            const bare = row.besideWords
            row.warning = "already pushed"
            compare(row.tag, "already pushed", "warned while it can be pressed")
            verify(drawn(row, "already pushed"), "after the words on screen")
            verify(row.besideWords > bare, "with its room bid beside the words")
            row.blockedReason = "Git is busy"
            compare(row.tag, "", "a reason greys it")
            verify(!drawn(row, "already pushed"), "and none is on screen")
            compare(row.besideWords, bare, "nor bid for")
            row.blockedReason = ""
            row.doable = false
            compare(row.tag, "", "and so does a press it cannot make now")
            row.doable = true
            compare(row.tag, "already pushed", "back as it can be pressed again")
            row.warning = ""
            row.note = "not merged"
            row.blockedReason = "Git is busy"
            compare(row.tag, "not merged", "the words' own tag stays, grey with them")
            verify(drawn(row, "not merged"), "on screen")
        }

        /// Whether `words` are on screen in `item`: a label under it on show that says them.
        function drawn(item, words) {
            if (!item.visible)
                return false
            if (item.text === words)
                return true
            const kids = item.children === undefined ? [] : item.children
            for (let i = 0; i < kids.length; i++) {
                if (drawn(kids[i], words))
                    return true
            }
            return false
        }

        /// The count goes the moment the row is blocked, and presses after it comes back begin a new one — short of the
        /// last press, so nothing lands.
        function test_a_pointer_count_blocked_midway_never_lands() {
            mousePress(row, 40, 10)
            tryVerify(() => row.holdProgress > 0)
            row.blockedReason = "Git is busy"
            compare(row.holdProgress, 0, "dropped at once, not by the idle reset")
            outlastTheHold()
            mouseRelease(row, 40, 10)
            row.blockedReason = ""
            for (let i = 0; i < root.presses - 1; i++)
                mouseClick(row, 40, 10)
            compare(holds.count, 0, "the presses after it began a new count")
            mouseClick(row, 40, 10)
            compare(holds.count, 1, "which the next press completes: the row is not left deaf")
        }

        function test_a_key_count_blocked_midway_never_lands() {
            row.forceActiveFocus()
            keyPress(Qt.Key_Space)
            tryVerify(() => row.holdProgress > 0)
            row.blockedReason = "Git is busy"
            compare(row.holdProgress, 0, "dropped at once, not by the idle reset")
            outlastTheHold()
            keyRelease(Qt.Key_Space)
            row.blockedReason = ""
            for (let i = 0; i < root.presses - 1; i++)
                keyClick(Qt.Key_Space)
            compare(holds.count, 0, "the presses after it began a new count")
            keyClick(Qt.Key_Space)
            compare(holds.count, 1, "which the next press completes: the row is not left deaf")
        }

        /// The keys a blocked row drops are the keys that stand in for a press, no more and no fewer: one more would
        /// eat a key the menu moves by, one fewer would run the row and close the menu.
        function test_a_blocked_row_with_no_count_drops_the_keys_that_stand_in_for_a_press() {
            compare(plainRow.holdDrive, null, "the row under test has no driver of its own")
            const keys = [Qt.Key_Space, Qt.Key_Return, Qt.Key_Enter, Qt.Key_A, Qt.Key_Escape, Qt.Key_Down, Qt.Key_F2]
            let stoodIn = 0
            for (const key of keys) {
                plainRow.forceActiveFocus()
                plainSeat.passed = []
                keyPress(key)
                const dropped = !plainRow.pressed && plainSeat.passed.indexOf(key) < 0
                keyRelease(key)
                compare(dropped, root.gatekeeper.holdKey(key), "key " + key)
                if (dropped)
                    stoodIn++
            }
            compare(stoodIn, 3, "Space, Return and Enter")
        }

        /// A row no hand can press takes no presses from automation either, and says so — the caller presses again
        /// later rather than waiting on a write nobody made.
        function test_automation_does_not_run_a_row_out_of_reach() {
            row.blockedReason = "Git is busy"
            compare(row.completeHold(), false, "blocked")
            row.blockedReason = ""
            row.doable = false
            compare(row.completeHold(), false, "turned off")
            row.doable = true
            row.offered = false
            compare(row.completeHold(), false, "not on offer")
            outlastTheHold()
            compare(holds.count, 0)
            row.offered = true
            compare(row.completeHold(), true, "and one in reach takes them")
            compare(holds.count, 1)
        }

        /// The control: the tests above would pass on a row that never lands.
        function test_a_row_unblocked_again_holds_as_before() {
            row.blockedReason = "Git is busy"
            row.blockedReason = ""
            for (let i = 0; i < root.presses; i++)
                mouseClick(row, 40, 10)
            compare(holds.count, 1, "the row answers at its last press")
            // Blocked on the floor: the answer has gone, and nothing more comes of it.
            row.blockedReason = "Git is busy"
            outlastTheHold()
            compare(holds.count, 1)
        }
    }
}
