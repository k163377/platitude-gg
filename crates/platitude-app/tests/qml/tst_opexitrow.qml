import QtQuick
import QtTest
import platitude.ui

// A row of the stopped operation's card turned off (`enabled`, its own or an ancestor's) answers nothing: no key, no
// press, and no hold already under way lands after it. Turned back on, it answers as before.
Item {
    id: root
    width: 360
    height: 200

    readonly property int hold: 300

    /// Takes the tab before the rows.
    Item {
        id: before
        width: 10
        height: 10
        activeFocusOnTab: true
    }

    Item {
        id: ancestor
        y: 20
        width: 340
        height: 120

        OpExitRow {
            id: plain
            width: 340
            code: "--continue"
            text: "Carry on with what is staged"
        }
        OpExitRow {
            id: held
            y: 60
            width: 340
            code: "--abort"
            text: "Undo it all and go back"
            holdMs: root.hold
        }
    }
    /// Somewhere else for the focus to go.
    Item { id: elsewhere }

    SignalSpy {
        id: plainPicks
        target: plain
        signalName: "picked"
    }
    SignalSpy {
        id: heldPicks
        target: held
        signalName: "picked"
    }

    TestCase {
        name: "OpExitRow"
        when: windowShown

        function init() {
            ancestor.enabled = true
            plain.enabled = true
            held.enabled = true
            elsewhere.forceActiveFocus()
            keyRelease(Qt.Key_Space)
            tryVerify(() => held.holdProgress === 0 && plain.holdProgress === 0)
            plainPicks.clear()
            heldPicks.clear()
        }

        /// Longer than the hold, so a fill that was going to land has landed.
        function outlastTheHold() {
            // waits(paced): the subject is a pick that must **not** come, so the wait is the hold it would come after.
            wait(root.hold * 2)
        }

        function test_a_row_turned_off_under_the_focus_takes_no_key_data() {
            return [{ tag: "Return", key: Qt.Key_Return }, { tag: "Enter", key: Qt.Key_Enter },
                    { tag: "Space", key: Qt.Key_Space }]
        }
        function test_a_row_turned_off_under_the_focus_takes_no_key(data) {
            plain.forceActiveFocus()
            verify(plain.activeFocus)
            plain.enabled = false
            verify(!plain.activeFocus, "the focus goes with it")
            keyClick(data.key)
            compare(plainPicks.count, 0)
        }

        function test_a_key_hold_turned_off_midway_never_lands() {
            held.forceActiveFocus()
            keyPress(Qt.Key_Space)
            tryVerify(() => held.holdProgress > 0)
            held.enabled = false
            tryVerify(() => held.holdProgress === 0)
            outlastTheHold()
            keyRelease(Qt.Key_Space)
            compare(heldPicks.count, 0)
        }

        function test_a_pointer_hold_turned_off_midway_never_lands() {
            mousePress(held, 40, 10)
            tryVerify(() => held.holdProgress > 0)
            held.enabled = false
            tryVerify(() => held.holdProgress === 0)
            outlastTheHold()
            mouseRelease(held, 40, 10)
            compare(heldPicks.count, 0)
        }

        function test_a_press_turned_off_before_its_release_picks_nothing() {
            mousePress(plain, 40, 10)
            plain.enabled = false
            mouseRelease(plain, 40, 10)
            compare(plainPicks.count, 0)
        }

        function test_an_ancestor_turned_off_reaches_the_row() {
            held.forceActiveFocus()
            keyPress(Qt.Key_Space)
            tryVerify(() => held.holdProgress > 0)
            ancestor.enabled = false
            verify(!plain.enabled && !held.enabled, "the rows read their ancestor's")
            compare(plain.wordColor, Theme.textMuted)
            compare(held.wordColor, Theme.textMuted)
            tryVerify(() => held.holdProgress === 0)
            outlastTheHold()
            keyRelease(Qt.Key_Space)
            mouseClick(plain, 40, 10)
            plain.forceActiveFocus()
            keyClick(Qt.Key_Return)
            compare(plainPicks.count, 0)
            compare(heldPicks.count, 0)
        }

        function test_a_row_turned_off_is_not_in_the_tab_order() {
            plain.enabled = false
            before.forceActiveFocus()
            keyClick(Qt.Key_Tab)
            verify(held.activeFocus, "the tab passes the row that is off")
        }

        function test_automation_does_not_run_a_row_turned_off() {
            plain.enabled = false
            held.enabled = false
            plain.completeHold()
            held.completeHold()
            outlastTheHold()
            compare(plainPicks.count, 0)
            compare(heldPicks.count, 0)
        }

        function test_a_row_turned_back_on_answers_again() {
            plain.enabled = false
            held.enabled = false
            plain.enabled = true
            held.enabled = true
            mouseClick(plain, 40, 10)
            compare(plainPicks.count, 1)
            held.forceActiveFocus()
            keyPress(Qt.Key_Space)
            tryVerify(() => heldPicks.count === 1)
            keyRelease(Qt.Key_Space)
        }

        /// The control: every test above would pass on a row that answers nothing.
        function test_a_row_that_is_on_answers_as_it_always_did() {
            compare(plain.wordColor, Theme.textPrimary)
            compare(held.wordColor, held.holdTone, "a held row wears its cost before it is touched")
            mouseClick(plain, 40, 10)
            compare(plainPicks.count, 1, "a plain row runs on the release")
            plain.forceActiveFocus()
            keyClick(Qt.Key_Return)
            compare(plainPicks.count, 2, "and on its key")
            mouseClick(held, 40, 10)
            outlastTheHold()
            compare(heldPicks.count, 0, "a held row ignores a click")
            mousePress(held, 40, 10)
            tryVerify(() => heldPicks.count === 1)
            mouseRelease(held, 40, 10)
            compare(heldPicks.count, 1, "and runs once the hold is through")
        }
    }
}
