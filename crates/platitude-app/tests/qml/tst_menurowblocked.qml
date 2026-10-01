import QtQuick
import QtTest
import platitude.ui

// A held menu row that is blocked (`AppMenuItem.blocked` — the menu's held reason arriving, git going busy) while a
// hand is already on it answers as a row blocked from the start: the hold under way blanks and `held` is not raised.
Item {
    id: root
    width: 360
    height: 120

    readonly property int hold: 300

    AppMenuItem {
        id: row
        width: 340
        text: "Delete branch"
        holdMs: root.hold
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
            elsewhere.forceActiveFocus()
            keyRelease(Qt.Key_Space)
            tryVerify(() => row.holdProgress === 0)
            holds.clear()
        }

        /// Longer than the hold, so a fill that was going to land has landed.
        function outlastTheHold() {
            // waits(paced): the subject is a hold that must **not** land, so the wait is the hold it would land after.
            wait(root.hold * 2)
        }

        function test_a_pointer_hold_blocked_midway_never_lands() {
            mousePress(row, 40, 10)
            tryVerify(() => row.holdProgress > 0)
            row.blockedReason = "Git is busy"
            tryVerify(() => row.holdProgress === 0)
            outlastTheHold()
            mouseRelease(row, 40, 10)
            compare(holds.count, 0)
        }

        function test_a_key_hold_blocked_midway_never_lands() {
            row.forceActiveFocus()
            keyPress(Qt.Key_Space)
            tryVerify(() => row.holdProgress > 0)
            row.blockedReason = "Git is busy"
            tryVerify(() => row.holdProgress === 0)
            outlastTheHold()
            keyRelease(Qt.Key_Space)
            compare(holds.count, 0)
        }

        function test_automation_does_not_run_a_blocked_or_disabled_row() {
            row.blockedReason = "Git is busy"
            row.completeHold()
            outlastTheHold()
            row.blockedReason = ""
            row.enabled = false
            row.completeHold()
            outlastTheHold()
            compare(holds.count, 0)
        }

        /// The control: the tests above would pass on a row that never holds.
        function test_a_row_unblocked_again_holds_as_before() {
            row.blockedReason = "Git is busy"
            row.blockedReason = ""
            mousePress(row, 40, 10)
            tryVerify(() => holds.count === 1)
            mouseRelease(row, 40, 10)
            compare(holds.count, 1)
        }
    }
}
