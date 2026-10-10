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

    AppMenuItem {
        id: row
        width: 340
        text: "Delete branch"
        holdPresses: root.presses
    }
    Item { id: elsewhere }

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
            row.enabled = true
            row.offered = true
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

        /// A row no hand can press takes no presses from automation either, and says so — the caller presses again
        /// later rather than waiting on a write nobody made.
        function test_automation_does_not_run_a_row_out_of_reach() {
            row.blockedReason = "Git is busy"
            compare(row.completeHold(), false, "blocked")
            row.blockedReason = ""
            row.enabled = false
            compare(row.completeHold(), false, "disabled")
            row.enabled = true
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
