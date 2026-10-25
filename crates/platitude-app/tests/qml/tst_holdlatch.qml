import QtQuick
import QtTest
import platitude.ui

// Whether presses keep the meaning their gesture began with (rules-refs/app-ui.md「回数押しか否かは最初の押下で確定する」),
// and what the last press does. The state deciding it flips on its own, and the flip has to land between two presses —
// a stretch no picture or headless run can stand in.
Item {
    id: root
    width: 200
    height: 200

    readonly property int presses: Metrics.holdPressesWarned

    readonly property HoldDriver drive: HoldDriver {
        id: drive
        holdPresses: root.presses
    }

    SignalSpy {
        id: finishes
        target: drive
        signalName: "finished"
    }
    SignalSpy {
        id: landings
        target: drive
        signalName: "landed"
    }

    ActionButton {
        id: button
        width: 120
        height: 32
        text: "push"
        holdPresses: root.presses
    }
    SignalSpy {
        id: clicks
        target: button
        signalName: "activated"
    }
    SignalSpy {
        id: holds
        target: button
        signalName: "held"
    }

    /// As much of a tab as the band's push button reads. Both are behind their remote, so the count cannot tell them
    /// apart.
    component Tab: QtObject {
        id: tab
        property int tab_id: 0
        property string pushTargetLabel: ""
        /// The branch the worktree is on — what a push actually sends.
        property string branch: ""
        property int sent: 0
        /// Which branch went, read back the way the page would send it.
        property string sentBranch: ""
        property string pushState: "behind"
        property int pushBehind: 1
        property int pushAhead: 0
        property bool pushFailed: false
        property bool planShown: false
        property bool canPush: false
        property bool canForcePush: true
        property string pushFailReason: ""
        property QtObject pageTab: QtObject { property string busyOp: "" }
        property QtObject pageWorktree: QtObject {
            property string branch: tab.branch
        }
        function forcePush() { tab.sent++; tab.sentBranch = tab.branch }
        function pushNow() { tab.sent++; tab.sentBranch = tab.branch }
    }
    readonly property Tab here: Tab { tab_id: 1; branch: "main"; pushTargetLabel: "origin/main" }
    readonly property Tab there: Tab { tab_id: 2; branch: "release"; pushTargetLabel: "origin/release" }

    BandPushButton {
        id: band
        // Clear of the plain button above: the mouse presses below are delivered by position.
        y: 48
        width: 160
        height: 32
        curPage: root.here
    }
    /// Somewhere else for the focus to go, the way opening another tab takes it.
    Item { id: elsewhere }

    TestCase {
        name: "HoldLatch"
        when: windowShown

        /// One press as a hand makes it: down, then up.
        function click() {
            drive.begin()
            drive.letUp()
        }
        function keyClickSpace(times) {
            for (let i = 0; i < times; i++) {
                keyPress(Qt.Key_Space)
                keyRelease(Qt.Key_Space)
            }
        }

        function init() {
            drive.settle()
            drive.letUp()
            drive.holdPresses = root.presses
            drive.premise = ""
            drive.waiting = false
            button.holdPresses = root.presses
            finishes.clear()
            landings.clear()
            clicks.clear()
            holds.clear()
            band.curPage = root.here
            root.here.branch = "main"
            root.here.sent = 0
            root.here.sentBranch = ""
            root.there.sent = 0
            root.there.sentBranch = ""
            // Keys counted on the band in the test before this one go with the focus, so nothing carries over.
            band.forceActiveFocus()
            keyRelease(Qt.Key_Space)
            elsewhere.forceActiveFocus()
            tryVerify(() => !band.gesturing)
            tryVerify(() => !drive.gesturing)
            // A button pressed through in the test before is locked for its floor.
            tryVerify(() => !button.gesturing)
        }

        function test_the_count_is_taken_at_the_first_press_and_kept() {
            compare(drive.armedPresses, root.presses)
            drive.begin()
            drive.holdPresses = 0
            compare(drive.armedPresses, root.presses, "the hand still down keeps the count it was made with")
            verify(drive.stale, "and says the answer it was made under has gone")
            compare(drive.count, 0, "the presses made under it are dropped")
            drive.letUp()
            compare(drive.armedPresses, 0, "the latch opens once the hand is up")
            verify(drive.stale, "and what was lost stays lost until something is pressed again")
            drive.begin()
            verify(!drive.stale, "which is the press")
        }

        /// The premise holds the target as well: it can be swapped while the count stays put, and presses finished on
        /// the new target would answer a question nobody asked.
        function test_presses_whose_target_went_count_for_nothing() {
            drive.premise = "here"
            click()
            drive.premise = "there"
            verify(drive.stale, "the presses were made against something else")
            compare(drive.count, 0)
            for (let i = 1; i < root.presses; i++)
                click()
            compare(finishes.count, 0, "so the presses left over start a new count, short of the last press")
        }

        /// The band outlives the page it points at, and the count is the same for both tabs: only the premise keeps
        /// the last press from overwriting a repository the hand never pointed at.
        function test_a_band_count_does_not_follow_the_tab_that_replaced_it() {
            band.forceActiveFocus()
            keyClickSpace(1)
            tryVerify(() => band.holdProgress > 0)
            band.curPage = root.there
            compare(band.holdProgress, 0, "dropped at once, not by the idle reset")
            keyClickSpace(root.presses - 1)
            compare(root.there.sent, 0, "the tab that arrived is not the one the presses began on")
            compare(root.here.sent, 0, "and neither is the one it left")
        }

        /// Two local branches can track one remote one: a switch between them leaves `origin/main` on the button while
        /// `PublishFlow.forcePush` would send the other branch (`worktree.branch`).
        function test_a_band_count_does_not_follow_a_branch_that_replaced_it() {
            band.forceActiveFocus()
            keyClickSpace(1)
            tryVerify(() => band.holdProgress > 0)
            root.here.branch = "main-rework"
            compare(band.holdProgress, 0, "dropped at once, not by the idle reset")
            keyClickSpace(root.presses - 1)
            compare(root.here.sent, 0, "the branch under the presses went, so nothing is sent: " + root.here.sentBranch)
        }

        /// Keys counted on one control are its own gesture: move the focus and they go.
        function test_a_band_count_dies_when_the_focus_leaves() {
            band.forceActiveFocus()
            keyClickSpace(root.presses - 1)
            tryVerify(() => band.holdProgress > 0)
            elsewhere.forceActiveFocus()
            compare(band.holdProgress, 0, "dropped at once, not by the idle reset")
            band.forceActiveFocus()
            keyClickSpace(1)
            compare(root.here.sent, 0, "the last press alone sends nothing: " + root.here.sentBranch)
        }

        /// `stale` is a latch: compared with what is true now, a target that went and came back would revive the
        /// count.
        function test_a_target_that_comes_back_does_not_revive_the_count() {
            band.forceActiveFocus()
            keyClickSpace(root.presses - 1)
            tryVerify(() => band.holdProgress > 0)
            band.curPage = root.there
            band.curPage = root.here
            compare(band.holdProgress, 0, "dropped at once, not by the idle reset")
            keyClickSpace(1)
            compare(root.here.sent, 0, "nothing was sent: " + root.here.sentBranch)
            compare(root.there.sent, 0)
        }

        /// The control: the band tests above would also pass on a button that sends nothing.
        function test_a_band_count_nothing_moved_under_sends_its_own_tab_once() {
            band.forceActiveFocus()
            keyClickSpace(root.presses)
            tryVerify(() => root.here.sent === 1)
            keyClickSpace(1)
            compare(root.here.sent, 1, "once: the last press locks it")
            compare(root.here.sentBranch, "main", "and it is the branch the presses were made on")
            compare(root.there.sent, 0)
        }

        /// Presses are counted as they land, and kept between them: rapid ones are not folded into one
        /// (デザイン規約 §回数押し).
        function test_presses_count_at_once_and_hold_between() {
            click()
            verify(drive.gesturing, "a press counted holds the latch with the hand up")
            compare(drive.count, 1)
            click()
            compare(drive.count, 2, "the next press, however soon, is the next one")
            drive.leave()
            compare(drive.count, 0, "the hand leaving the control takes the count back to zero")
            tryVerify(() => !drive.gesturing, undefined, "and the fill slides back out")
        }

        function test_a_plain_press_stays_plain() {
            drive.holdPresses = 0
            compare(drive.armedPresses, 0)
            drive.begin()
            drive.holdPresses = root.presses
            compare(drive.armedPresses, 0, "the press is still the plain one it was made as")
            verify(drive.stale)
        }

        /// The hand was pressing for one command and would be answered with another, so the gesture ends silently.
        function test_presses_whose_answer_went_raise_nothing() {
            click()
            drive.holdPresses = 0
            for (let i = 1; i < root.presses; i++)
                click()
            compare(finishes.count, 0, "the presses after it press a plain control")
        }

        /// The control: the test above would also pass on a driver that never fires.
        function test_the_last_press_fires_once_and_locks() {
            for (let i = 0; i < root.presses; i++)
                click()
            compare(finishes.count, 1, "the last press fires at once")
            verify(drive.done)
            click()
            compare(finishes.count, 1, "and a press after it is not counted")
            tryVerify(() => landings.count === 1, undefined, "the floor passes")
            tryVerify(() => !drive.gesturing, undefined, "and with no work out the control settles")
        }

        /// Work still out past the floor keeps the full fill, its ring turning, until it is in.
        function test_work_still_out_holds_the_full_fill() {
            drive.waiting = true
            for (let i = 0; i < root.presses; i++)
                click()
            tryVerify(() => drive.loading, undefined, "past the floor the ring turns")
            verify(drive.dimmed, "over the fill's step down")
            drive.blank()
            verify(drive.done, "a control going deaf under its own work keeps it")
            drive.waiting = false
            verify(!drive.gesturing, "the work in, it settles at once")
        }

        /// Automation presses with no hand: no floor is waited out.
        function test_running_out_answers_at_once() {
            drive.runOut()
            compare(finishes.count, 1)
            compare(landings.count, 1)
            verify(!drive.done, "and it is settled before the call returns")
        }

        /// A release that read the count again would find zero and let the gesture fall through to the plain command.
        function test_a_button_pressed_when_the_answer_goes_reports_no_click() {
            mousePress(button, 10, 10)
            button.holdPresses = 0
            mouseRelease(button, 10, 10)
            compare(clicks.count, 0, "the press was a counted one, which reports no click")
            compare(holds.count, 0, "and its gesture is not fired either, its answer having gone")
        }

        /// The other way round: the plain press is dropped because the answer it was made under went — not, as a
        /// release reading the new count would, for finding a counted gesture.
        function test_a_button_whose_answer_goes_mid_press_runs_nothing() {
            button.holdPresses = 0
            mousePress(button, 10, 10)
            button.holdPresses = root.presses
            mouseRelease(button, 10, 10)
            compare(holds.count, 0)
            compare(clicks.count, 0,
                    "the premise went, so the plain press is dropped rather than answered with the presses' command")
        }

        /// The control the button tests above are measured against.
        function test_a_plain_button_nothing_moved_under_answers_its_click() {
            button.holdPresses = 0
            mousePress(button, 10, 10)
            mouseRelease(button, 10, 10)
            compare(clicks.count, 1)
            compare(holds.count, 0)
        }

        /// The releases still come; the latch keeps them from also running the plain command
        /// (`ActionButton.clickWanted`).
        function test_presses_run_to_the_end_are_answered_once() {
            for (let i = 0; i < root.presses; i++)
                mouseClick(button, 10, 10)
            compare(holds.count, 1, "the last press fired once")
            compare(clicks.count, 0, "and no release raised a click beside it")
        }

        /// The press's other hand, Space on the focused control (デザイン規約 §回数押し): a release judged by the new
        /// count would fall through to the control underneath as a plain press.
        function test_a_key_pressed_when_the_answer_goes_reports_nothing() {
            button.forceActiveFocus()
            keyPress(Qt.Key_Space)
            button.holdPresses = 0
            keyRelease(Qt.Key_Space)
            compare(holds.count, 0, "the presses' answer went, so it is not fired")
            compare(clicks.count, 0, "and the release is not let through as a click either")
        }

        /// The control: the test above would also pass against a key nothing hears.
        function test_keys_pressed_to_the_end_fire_the_button() {
            button.forceActiveFocus()
            keyClickSpace(root.presses)
            compare(holds.count, 1)
            compare(clicks.count, 0)
        }

        /// The band's doors raise `clicked()` with no press so a run presses what a hand does (`TopBar.stashNow` /
        /// `fetchNow`); a latch that only reads a press would answer none of them.
        function test_a_click_raised_with_no_press_behind_it_still_answers() {
            button.holdPresses = 0
            button.clicked()
            compare(clicks.count, 1)
            // …and a counted button still reports no click, however the click was raised.
            button.holdPresses = root.presses
            button.clicked()
            compare(clicks.count, 1)
            compare(holds.count, 0)
        }
    }
}
