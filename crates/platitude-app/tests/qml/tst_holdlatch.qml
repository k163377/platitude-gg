import QtQuick
import QtTest
import platitude.ui

// Whether a press keeps the meaning it was made with (デザイン規約 §長押し「長押しか否かは押した瞬間に確定」).
//
// **The condition moves on its own**: a fetch answering, a reachability walk coming back, git refusing the press
// before it. Read live, a hold begun on a red button comes back as a click and runs the plain command, and a click
// begun on a plain one is dropped when the button turns into a hold under the hand. Neither is something a picture
// or a headless run can be made to show — the flip has to happen *between* a press and its release, which is a
// stretch nothing outside the control can stand in.
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

    /// A tab, as much of one as the band's push button reads. Two of them, both behind their remote — which is what
    /// makes the pair the case the length cannot see: the same 500ms is armed either way.
    component Tab: QtObject {
        id: tab
        property int tab_id: 0
        property string pushTargetLabel: ""
        /// The branch the working tree is on — **what a push actually sends**, which the destination does not say:
        /// two local branches can track one remote one (`PublishFlow.forcePush`).
        property string branch: ""
        property int sent: 0
        /// Which branch went, read back the way the page would send it.
        property string sentBranch: ""
        property string pushState: "behind"
        property bool pushFailed: false
        property bool planShown: false
        property bool canPush: false
        property bool canForcePush: true
        property string pushFailReason: ""
        property QtObject pageTab: QtObject { property string busyOp: "" }
        property QtObject pageWt: QtObject {
            property string branch: tab.branch
            property int behind: 1
            property int ahead: 0
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

        /// The length the fill is drawing out is the one the press was given, and it says so about itself.
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

        /// The other direction, which is the one that can drop a press altogether: a plain press whose row turns
        /// into a held one before the release.
        /// **The premise holds the target as well.** What a press is aimed at can be swapped out while the
        /// length stays exactly where it was, and a fill that ran out over the new target would answer a question
        /// nobody asked.
        function test_a_hold_whose_target_went_raises_nothing() {
            drive.premise = "here"
            drive.begin()
            drive.premise = "there"
            verify(drive.stale, "the press was made against something else")
            tryVerify(() => !drive.fill.running)
            compare(finishes.count, 0, "so the fill ran out and said nothing")
        }

        /// **The band outlives the page it points at.** A hand on `push -f` with a tab switched under it is the case
        /// the length is blind to: both tabs are behind their remotes, so the same 500ms is armed either way, and
        /// before the premise the release overwrote the branch of a repository the hand never pointed at
        /// (repro: qmltestrunner, `first=0 second=1`).
        function test_a_band_hold_does_not_follow_the_tab_that_replaced_it() {
            band.forceActiveFocus()
            keyPress(Qt.Key_Space)
            // **The fill has to be climbing before the tab is swapped** — `gesturing` rises on the press itself, and a
            // swap made in front of the first frame leaves nothing for the rest of this to be about.
            tryVerify(() => band.holdProgress > 0)
            elsewhere.forceActiveFocus()
            band.curPage = root.there
            // Run out, one way or the other: the fill blanks whether it fired or was ignored.
            tryVerify(() => band.holdProgress === 0)
            keyRelease(Qt.Key_Space)
            compare(root.there.sent, 0, "the tab that arrived is not the one the hand was on")
            compare(root.here.sent, 0, "and neither is the one it left, the press having been let go of elsewhere")
        }

        /// **The push sends the branch.** Two local branches can track one remote one, so a switch between
        /// them under the hand leaves `origin/main` on the button either way while the history about to be overwritten
        /// is another branch's (`PublishFlow.forcePush` sends `workTree.branch`).
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

        /// **A key is answered wherever the focus is when it comes up.** Move it and the release lands elsewhere,
        /// leaving the fill to run out on a button nobody is still holding.
        function test_a_band_hold_dies_when_the_focus_leaves_before_the_key_comes_up() {
            band.forceActiveFocus()
            keyPress(Qt.Key_Space)
            tryVerify(() => band.holdProgress > 0)
            elsewhere.forceActiveFocus()
            keyRelease(Qt.Key_Space)
            tryVerify(() => band.holdProgress === 0)
            compare(root.here.sent, 0, "nothing was sent: " + root.here.sentBranch)
        }

        /// **What was lost stays lost.** Read as a comparison with what is true now, a target that went and came back
        /// put the gesture back in business — the tab switched away from and switched back to answered the press as
        /// though nothing had happened.
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

        /// And the ordinary hold on that same button goes through **once**, to the tab it was made on — without which
        /// the test above passes on a button that sends nothing at all.
        function test_a_band_hold_nothing_moved_under_sends_its_own_tab_once() {
            band.forceActiveFocus()
            keyPress(Qt.Key_Space)
            tryVerify(() => root.here.sent === 1)
            keyRelease(Qt.Key_Space)
            compare(root.here.sent, 1, "once, and the release adds nothing")
            compare(root.here.sentBranch, "main", "and it is the branch the press was made on")
            compare(root.there.sent, 0)
        }

        /// **A gesture lasts past the release.** A press that stopped short slides its fill back out, and an owner
        /// latching something other than the length on `gesturing` has to hold through that slide — a row re-wording
        /// itself as the hand lifts is the same disagreement, half a beat later.
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

        /// **Nothing is raised for a premise that has gone.** The fill runs out under a hand that was holding for one
        /// command and would be answered with another, so the gesture ends silently.
        function test_a_hold_whose_answer_went_raises_nothing() {
            drive.holdMs = 20
            drive.begin()
            drive.holdMs = 0
            // The fill running out is the event, waited for by the animation's own end
            // (規約 §非同期・並行テスト).
            tryVerify(() => !drive.fill.running)
            compare(finishes.count, 0, "the hold ran out and said nothing")
        }

        /// And the same fill with its answer intact still fires, so the test above is not passing on a driver that
        /// never fires at all.
        function test_a_hold_that_kept_its_answer_still_fires() {
            drive.holdMs = 20
            drive.begin()
            tryVerify(() => finishes.count === 1)
        }

        /// The button end of it, pressed and released the way a hand does. **A hold begun on it reports no click** —
        /// a release that read the length again would find zero and let the press fall through to the plain command.
        function test_a_button_held_when_the_answer_goes_reports_no_click() {
            mousePress(button, 10, 10)
            button.holdMs = 0
            mouseRelease(button, 10, 10)
            compare(clicks.count, 0, "the press was a hold, and a hold reports no click")
            compare(holds.count, 0, "and the hold it was is not fired either, its answer having gone")
        }

        /// And the other way round. The press is dropped because the answer it was made under has gone, which is the
        /// same thing the hold above is dropped for. A release that read the new length would drop it **silently and
        /// for the wrong reason** — finding a hold and letting the click go — and the button would run the plain
        /// command had the flip landed a frame later.
        function test_a_button_whose_answer_goes_mid_press_runs_nothing() {
            button.holdMs = 0
            mousePress(button, 10, 10)
            button.holdMs = root.hold
            mouseRelease(button, 10, 10)
            compare(holds.count, 0)
            compare(clicks.count, 0,
                    "the premise went, so the plain press is dropped rather than answered with the hold's command")
        }

        /// The ordinary press, which everything above is measured against.
        function test_a_plain_button_nothing_moved_under_answers_its_click() {
            button.holdMs = 0
            mousePress(button, 10, 10)
            mouseRelease(button, 10, 10)
            compare(clicks.count, 1)
            compare(holds.count, 0)
        }

        /// And a hold that runs all the way through is answered **once**. The release still comes, and the same latch
        /// is what keeps it from falling through to the plain command beside the one the hold just ran — the reason
        /// the click is judged from what the press was given (`ActionButton.clickWanted`).
        function test_a_hold_run_to_the_end_is_answered_once() {
            mousePress(button, 10, 10)
            tryVerify(() => holds.count === 1)
            mouseRelease(button, 10, 10)
            compare(holds.count, 1, "the fill fired once")
            compare(clicks.count, 0, "and the release it ended under raised no click beside it")
        }

        /// **The hold's other hand** (デザイン規約 §長押し): focus the control and hold Space. The key is taken at
        /// the press against the live length and answered at the release against the one the press was given — a
        /// release judged by the new length would fall through to the control underneath, which reads it as a plain
        /// press and runs the command the hold stood in for.
        function test_a_key_held_when_the_answer_goes_reports_nothing() {
            button.holdMs = root.hold
            button.forceActiveFocus()
            keyPress(Qt.Key_Space)
            button.holdMs = 0
            keyRelease(Qt.Key_Space)
            compare(holds.count, 0, "the hold's answer went, so it is not fired")
            compare(clicks.count, 0, "and the release is not let through as a click either")
        }

        /// And the ordinary one, so the test above is not passing against a key nothing ever hears.
        function test_a_key_held_to_the_end_fires_the_hold() {
            button.holdMs = 20
            button.forceActiveFocus()
            keyPress(Qt.Key_Space)
            tryVerify(() => holds.count === 1)
            keyRelease(Qt.Key_Space)
            compare(clicks.count, 0)
        }

        /// **A `clicked()` raised with no press behind it still answers.** The band's own doors do exactly that,
        /// so that a run presses what a hand presses (`TopBar.stashNow` / `fetchNow`,
        /// verify-ui §壊れない動詞の実装) — and a latch that only ever reads a press answers none of them, which
        /// took the stash verb's press out of the build entirely.
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
