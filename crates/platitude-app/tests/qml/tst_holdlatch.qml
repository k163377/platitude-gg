import QtQuick
import QtTest
import platitude.ui

// Whether a press keeps the meaning it was made with (rules-refs/app-ui.md「長押しか否かは押した瞬間に確定する」).
// The state deciding it flips on its own, and the flip has to land between a press and its release — a stretch no
// picture or headless run can stand in.
Item {
    id: root
    width: 200
    height: 200

    readonly property int hold: 400

    HoldDriver {
        id: drive
        holdMs: root.hold
    }

    SignalSpy {
        id: finishes
        target: drive
        signalName: "finished"
    }

    ActionButton {
        id: button
        width: 120
        height: 32
        text: "push"
        holdMs: root.hold
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

    /// As much of a tab as the band's push button reads. Both are behind their remote, so the length cannot tell them
    /// apart.
    component Tab: QtObject {
        id: tab
        property int tab_id: 0
        property string pushTargetLabel: ""
        /// The branch the working tree is on — what a push actually sends.
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
        property QtObject pageWt: QtObject {
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

        function init() {
            drive.blank()
            drive.holdMs = root.hold
            drive.premise = ""
            button.holdMs = root.hold
            finishes.clear()
            clicks.clear()
            holds.clear()
            band.curPage = root.here
            root.here.branch = "main"
            root.here.sent = 0
            root.here.sentBranch = ""
            root.there.sent = 0
            root.there.sentBranch = ""
            // The key from the test before this one is let go of on the button that took it, so nothing carries over.
            band.forceActiveFocus()
            keyRelease(Qt.Key_Space)
            tryVerify(() => !band.gesturing)
        }

        function test_the_length_is_taken_at_the_press_and_kept() {
            compare(drive.armedMs, root.hold)
            drive.begin()
            drive.holdMs = 0
            compare(drive.armedMs, root.hold, "the press keeps the length it was made with")
            verify(drive.stale, "and says the answer it was made under has gone")
            drive.letUp()
            drive.blank()
            compare(drive.armedMs, 0, "the latch opens once the gesture is over")
            verify(drive.stale, "and what was lost stays lost until something is pressed again")
            drive.begin()
            verify(!drive.stale, "which is the press")
        }

        /// The premise holds the target as well: it can be swapped while the length stays put, and a fill run out over
        /// the new target would answer a question nobody asked.
        function test_a_hold_whose_target_went_raises_nothing() {
            drive.premise = "here"
            drive.begin()
            drive.premise = "there"
            verify(drive.stale, "the press was made against something else")
            tryVerify(() => !drive.fill.running)
            compare(finishes.count, 0, "so the fill ran out and said nothing")
        }

        /// The band outlives the page it points at, and the length is the same for both tabs: only the premise keeps
        /// the release from overwriting a repository the hand never pointed at.
        function test_a_band_hold_does_not_follow_the_tab_that_replaced_it() {
            band.forceActiveFocus()
            keyPress(Qt.Key_Space)
            // Wait for the fill, not `gesturing` (which rises on the press itself): a swap before the first frame
            // tests nothing.
            tryVerify(() => band.holdProgress > 0)
            elsewhere.forceActiveFocus()
            band.curPage = root.there
            // Run out, one way or the other: the fill blanks whether it fired or was ignored.
            tryVerify(() => band.holdProgress === 0)
            keyRelease(Qt.Key_Space)
            compare(root.there.sent, 0, "the tab that arrived is not the one the hand was on")
            compare(root.here.sent, 0, "and neither is the one it left, the press having been let go of elsewhere")
        }

        /// Two local branches can track one remote one: a switch between them leaves `origin/main` on the button while
        /// `PublishFlow.forcePush` would send the other branch (`workTree.branch`).
        function test_a_band_hold_does_not_follow_a_branch_that_replaced_it() {
            band.forceActiveFocus()
            keyPress(Qt.Key_Space)
            tryVerify(() => band.holdProgress > 0)
            root.here.branch = "main-rework"
            tryVerify(() => band.holdProgress === 0)
            keyRelease(Qt.Key_Space)
            compare(root.here.sent, 0,
                    "the branch under the hand went, so nothing is sent: " + root.here.sentBranch)
        }

        /// A key is answered wherever the focus is when it comes up: move it and the fill runs out on a button nobody
        /// is holding.
        function test_a_band_hold_dies_when_the_focus_leaves_before_the_key_comes_up() {
            band.forceActiveFocus()
            keyPress(Qt.Key_Space)
            tryVerify(() => band.holdProgress > 0)
            elsewhere.forceActiveFocus()
            keyRelease(Qt.Key_Space)
            tryVerify(() => band.holdProgress === 0)
            compare(root.here.sent, 0, "nothing was sent: " + root.here.sentBranch)
        }

        /// `stale` is a latch: compared with what is true now, a target that went and came back would revive the
        /// gesture.
        function test_a_target_that_comes_back_does_not_revive_the_gesture() {
            band.forceActiveFocus()
            keyPress(Qt.Key_Space)
            tryVerify(() => band.holdProgress > 0)
            band.curPage = root.there
            band.curPage = root.here
            tryVerify(() => band.holdProgress === 0)
            keyRelease(Qt.Key_Space)
            compare(root.here.sent, 0, "nothing was sent: " + root.here.sentBranch)
            compare(root.there.sent, 0)
        }

        /// The control: the band tests above would also pass on a button that sends nothing.
        function test_a_band_hold_nothing_moved_under_sends_its_own_tab_once() {
            band.forceActiveFocus()
            keyPress(Qt.Key_Space)
            tryVerify(() => root.here.sent === 1)
            keyRelease(Qt.Key_Space)
            compare(root.here.sent, 1, "once, and the release adds nothing")
            compare(root.here.sentBranch, "main", "and it is the branch the press was made on")
            compare(root.there.sent, 0)
        }

        /// An owner latching on `gesturing` has to hold through the fill sliding back, or a row re-words itself as the
        /// hand lifts.
        function test_a_gesture_lasts_past_the_release_while_the_fill_slides_back() {
            drive.begin()
            verify(drive.gesturing, "the press itself")
            tryVerify(() => drive.progress > 0)
            drive.letUp()
            verify(drive.gesturing, "and the fill sliding back out after it")
            tryVerify(() => !drive.gesturing)
        }

        function test_a_plain_press_stays_plain() {
            drive.holdMs = 0
            compare(drive.armedMs, 0)
            drive.begin()
            drive.holdMs = root.hold
            compare(drive.armedMs, 0, "the press is still the plain one it was made as")
            verify(drive.stale)
        }

        /// The hand was holding for one command and would be answered with another, so the gesture ends silently.
        function test_a_hold_whose_answer_went_raises_nothing() {
            drive.holdMs = 20
            drive.begin()
            drive.holdMs = 0
            // Waited for by the animation's own end (core.md §非同期・並行テスト).
            tryVerify(() => !drive.fill.running)
            compare(finishes.count, 0, "the hold ran out and said nothing")
        }

        /// The control: the test above would also pass on a driver that never fires.
        function test_a_hold_that_kept_its_answer_still_fires() {
            drive.holdMs = 20
            drive.begin()
            tryVerify(() => finishes.count === 1)
        }

        /// A release that read the length again would find zero and let the hold fall through to the plain command.
        function test_a_button_held_when_the_answer_goes_reports_no_click() {
            mousePress(button, 10, 10)
            button.holdMs = 0
            mouseRelease(button, 10, 10)
            compare(clicks.count, 0, "the press was a hold, and a hold reports no click")
            compare(holds.count, 0, "and the hold it was is not fired either, its answer having gone")
        }

        /// The other way round: the plain press is dropped because the answer it was made under went — not, as a
        /// release reading the new length would, for finding a hold.
        function test_a_button_whose_answer_goes_mid_press_runs_nothing() {
            button.holdMs = 0
            mousePress(button, 10, 10)
            button.holdMs = root.hold
            mouseRelease(button, 10, 10)
            compare(holds.count, 0)
            compare(clicks.count, 0,
                    "the premise went, so the plain press is dropped rather than answered with the hold's command")
        }

        /// The control the button tests above are measured against.
        function test_a_plain_button_nothing_moved_under_answers_its_click() {
            button.holdMs = 0
            mousePress(button, 10, 10)
            mouseRelease(button, 10, 10)
            compare(clicks.count, 1)
            compare(holds.count, 0)
        }

        /// The release still comes; the latch keeps it from also running the plain command
        /// (`ActionButton.clickWanted`).
        function test_a_hold_run_to_the_end_is_answered_once() {
            mousePress(button, 10, 10)
            tryVerify(() => holds.count === 1)
            mouseRelease(button, 10, 10)
            compare(holds.count, 1, "the fill fired once")
            compare(clicks.count, 0, "and the release it ended under raised no click beside it")
        }

        /// The hold's other hand, Space on the focused control (デザイン規約 §長押し): a release judged by the new length
        /// would fall through to the control underneath as a plain press.
        function test_a_key_held_when_the_answer_goes_reports_nothing() {
            button.holdMs = root.hold
            button.forceActiveFocus()
            keyPress(Qt.Key_Space)
            button.holdMs = 0
            keyRelease(Qt.Key_Space)
            compare(holds.count, 0, "the hold's answer went, so it is not fired")
            compare(clicks.count, 0, "and the release is not let through as a click either")
        }

        /// The control: the test above would also pass against a key nothing hears.
        function test_a_key_held_to_the_end_fires_the_hold() {
            button.holdMs = 20
            button.forceActiveFocus()
            keyPress(Qt.Key_Space)
            tryVerify(() => holds.count === 1)
            keyRelease(Qt.Key_Space)
            compare(clicks.count, 0)
        }

        /// The band's doors raise `clicked()` with no press so a run presses what a hand does (`TopBar.stashNow` /
        /// `fetchNow`); a latch that only reads a press would answer none of them.
        function test_a_click_raised_with_no_press_behind_it_still_answers() {
            button.holdMs = 0
            button.clicked()
            compare(clicks.count, 1)
            // …and a held button still reports no click, however the click was raised.
            button.holdMs = root.hold
            button.clicked()
            compare(clicks.count, 1)
            compare(holds.count, 0)
        }
    }
}
