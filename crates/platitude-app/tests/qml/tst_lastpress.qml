import QtQuick
import QtTest
import platitude.ui

// What a press count answers, and when (デザイン規約 §回数押し): the answer is decided and sent at the last press, while
// the presses' latches still hold; only going off the screen waits out the floor. Each case is something that can land
// inside those `holdFloorMs` — git's unmerged answer, the card aimed at another branch, git's answer to the delete
// itself — and must not change what was pressed for.
Item {
    id: root
    width: 600
    height: 600

    /// What the card asked for, every request in order: the card runs nothing itself.
    property var asked: []

    RefBranchMenu {
        id: card
        onDeleteRequested: (kind, id, name, oidHex) => root.asked.push("delete " + id)
        onForceDeleteRequested: branch => root.asked.push("force " + branch)
        onDeleteRemoteRequested: (remoteRef, expect) => root.asked.push("delete-remote " + remoteRef)
        onDeleteEverywhereRequested: (branch, remoteRef, forced, expect) =>
            root.asked.push("delete-both " + branch + " forced=" + forced)
    }

    /// A branch with a reading on the remote, whose merged state only git can answer (`merged: ""`): the answer arrives
    /// later, as `checkedBranch` / `checkedMerged`.
    function standOn(name) {
        card.standOn("branch", name, name, "abc123", {
            "heldByWorktree": "", "holderLeaf": "", "remoteCounterpart": "origin/" + name, "upstream": "origin/" + name,
            "upstreamWhere": "remote",
            "remoteCounterpartOid": "def456", "remoteDrifted": false, "open": true, "merged": "",
            "offers": ["switch", "branch-here", "integrate", "delete", "delete-remote", "set-upstream"]
        })
    }

    AskBar {
        id: bar
        y: 300
        width: 500
        label: "Undo it and go?"
        code: "rebase --abort"
        danger: true
        hold: true
        onConfirmed: {
            tc.confirms++
            // As `RepoPage.answerRowAsk`: the owner lowers the bar as it answers.
            bar.open = false
        }
    }

    /// The plan as the run bar sees it, answering the way `RebasePlanModel` does (`answer.rs`): the todo is taken at
    /// `acceptRun` and given as taken; `steps` stands for the rows a hand can still reach.
    QtObject {
        id: plan
        property bool dirty: true
        property int dropCount: 1
        property int stepCount: 2
        property bool locked: false
        property var steps: ["drop A", "pick B"]
        property var taken: null
        property int accepts: 0
        /// What reached git, every run in order.
        property var ran: []
        function acceptRun() {
            if (plan.locked)
                return 0
            plan.accepts++
            plan.taken = plan.steps.slice()
            plan.locked = true
            return 7
        }
        function giveAccepted(ticket) {
            if (ticket !== 7 || plan.taken === null)
                return
            plan.ran = plan.ran.concat([plan.taken])
            plan.taken = null
            plan.locked = false
        }
        function runPlan() {
            plan.ran = plan.ran.concat([plan.steps.slice()])
        }
    }
    RebasePlanRunBar {
        id: runBar
        y: 450
        width: 300
        plan: plan
        pushedCount: 0
        tipHeldElsewhere: false
        busy: false
    }
    /// The plan as the plan pane sees it, with no rows: the header and the `Discard` beside it, answering the way
    /// `RebasePlanModel` does (`answer.rs`).
    ListModel {
        id: draft
        property string ontoRef: "main"
        property bool root: false
        property string ontoOid: "abc123"
        property string ontoSubject: "the base"
        property int ontoAvatar: 0
        property string ontoAvatarUrl: ""
        property bool locked: false
        property bool taken: false
        property int accepts: 0
        /// Discards that reached the plan, and the pane's other way to put it away.
        property int discarded: 0
        property int cancels: 0
        function acceptDiscard() {
            if (draft.locked)
                return 0
            draft.accepts++
            draft.taken = true
            draft.locked = true
            return 9
        }
        function giveAccepted(ticket) {
            if (ticket !== 9 || !draft.taken)
                return
            draft.discarded++
            draft.taken = false
            draft.locked = false
        }
        function cancelPlan() {
            draft.cancels++
        }
        function selectRow(row) {}
        function canFold(row) {
            return false
        }
    }
    RebasePlanPane {
        id: planPane
        y: 500
        width: 400
        height: 100
        planModel: draft
        waiting: false
        discards: true
    }

    /// A settings list's removals, over a list of two people as filed.
    readonly property var twoFiled: [{ email: "a@example.com", url: "file:///a.png" },
                                     { email: "b@example.com", url: "file:///b.png" }]
    AvatarRemovals {
        id: removals
        filed: root.twoFiled
        onRemove: email => tc.removed = tc.removed.concat([email])
    }
    Item { id: elsewhere }

    TestCase {
        id: tc
        name: "LastPress"
        when: windowShown

        property int confirms: 0
        /// Who `removals` took the picture off, in order.
        property var removed: []

        function init() {
            root.asked = []
            tc.confirms = 0
            card.checkedBranch = ""
            card.checkedMerged = ""
            card.forceDeleteBranch = ""
            card.landedDelete = ""
            plan.steps = ["drop A", "pick B"]
            plan.taken = null
            plan.locked = false
            plan.accepts = 0
            plan.ran = []
            draft.taken = false
            draft.locked = false
            draft.accepts = 0
            draft.discarded = 0
            draft.cancels = 0
            removals.filed = root.twoFiled
            removals.waiting = []
            tc.removed = []
        }

        function cleanup() {
            card.dismiss()
            bar.open = false
            elsewhere.forceActiveFocus()
            tryVerify(() => !card.visible && bar.shut)
            tryVerify(() => !card.deleteItem.gesturing && !card.deleteRemoteItem.gesturing
                      && !card.deleteBothItem.gesturing && bar.holdProgress === 0 && plan.taken === null
                      && !draft.taken && removals.waiting.length === 0
                      && runBar.runTicket === 0 && planPane.discardTicket === 0)
        }

        function openOn(name) {
            root.standOn(name)
            card.offer()
            tryVerify(() => card.opened)
            root.asked = []
        }

        /// Presses as a hand makes them from the keyboard, on whatever holds the focus.
        function pressKeys(times) {
            for (let i = 0; i < times; i++) {
                keyPress(Qt.Key_Space)
                keyRelease(Qt.Key_Space)
            }
        }

        function pressRow(row, times) {
            row.forceActiveFocus()
            tc.pressKeys(times)
        }

        /// Past the floor: whatever was going to happen after it has happened.
        function outlastTheFloor() {
            // waits(paced): the subject is what must **not** happen once the floor has passed.
            wait(Metrics.holdFloorMs * 2)
        }

        function test_a_plain_delete_stays_plain_when_the_unmerged_answer_lands_on_its_floor() {
            tc.openOn("topic")
            compare(card.deleteItem.holdPresses, Metrics.holdPressesPlain)
            tc.pressRow(card.deleteItem, Metrics.holdPressesPlain)
            compare(root.asked, ["delete topic"], "sent at the last press")
            card.checkedBranch = "topic"
            card.checkedMerged = "no"
            tc.outlastTheFloor()
            compare(root.asked, ["delete topic"],
                    "and nothing after it: consent to the plain delete is not taken for one to -D")
            verify(card.opened, "the card stays up for git's answer")
            compare(card.deleteItem.code, "branch -D", "which the row wears from the next press on")
        }

        function test_delete_both_keeps_the_local_half_its_first_press_saw() {
            tc.openOn("topic")
            tc.pressRow(card.deleteBothItem, 1)
            card.checkedBranch = "topic"
            card.checkedMerged = "no"
            compare(card.deleteBothItem.forces, false, "latched at the first press")
            tc.pressKeys(Metrics.holdPressesWarned - 1)
            compare(root.asked, ["delete-both topic forced=false"])
        }

        function test_a_row_that_closes_its_card_waits_out_the_floor() {
            tc.openOn("topic")
            tc.pressRow(card.deleteRemoteItem, Metrics.holdPressesWarned)
            compare(root.asked, ["delete-remote origin/topic"], "sent at the last press")
            verify(card.opened, "the card stays up under the full fill")
            verify(card.deleteRemoteItem.gesturing, "its row still completing")
            tryVerify(() => !card.opened, undefined, "and goes once the floor has passed")
        }

        /// The card closed and aimed at another branch inside the floor: the first branch's answer has already gone,
        /// and its row's floor neither asks for anything more nor closes the card now standing.
        function test_a_confirmation_never_reaches_the_card_aimed_elsewhere_on_its_floor() {
            tc.openOn("topic")
            tc.pressRow(card.deleteRemoteItem, Metrics.holdPressesWarned)
            compare(root.asked, ["delete-remote origin/topic"])
            card.dismiss()
            tryVerify(() => !card.visible)
            tc.openOn("other")
            tc.outlastTheFloor()
            compare(root.asked, [], "the card aimed at the other branch is asked nothing")
            verify(card.opened, "and is not closed by the first row's floor")
        }

        function test_git_answering_inside_the_floor_leaves_the_card_its_floor() {
            tc.openOn("topic")
            tc.pressRow(card.deleteItem, Metrics.holdPressesPlain)
            card.landedDelete = "topic"
            verify(card.opened, "the full fill is still seen")
            tryVerify(() => !card.opened, undefined, "and the card goes with the floor")
        }

        function focusInside(item) {
            for (let at = item.Window.activeFocusItem; at !== null; at = at.parent) {
                if (at === item)
                    return true
            }
            return false
        }

        function raise() {
            bar.open = true
            tryVerify(() => bar.settled && tc.focusInside(bar))
            compare(bar.armedPresses, Metrics.holdPressesWarned)
        }

        function test_a_question_is_answered_at_the_last_press_and_stays_down_for_the_floor() {
            tc.raise()
            tc.pressKeys(Metrics.holdPressesWarned)
            compare(tc.confirms, 1, "answered at the last press")
            verify(!bar.open, "the owner has lowered it")
            verify(bar.openHeight > 0, "but it stays down under the full fill")
            tryVerify(() => bar.shut, undefined, "and goes up once the floor has passed")
            compare(tc.confirms, 1, "answered once")
        }

        function test_the_next_question_on_the_floor_gets_a_pill_of_its_own() {
            tc.raise()
            tc.pressKeys(Metrics.holdPressesWarned)
            compare(tc.confirms, 1)
            verify(!bar.open && bar.openHeight > 0, "the first answer's floor is running")
            bar.open = true
            compare(bar.holdProgress, 0, "the last answer's fill is not this question's")
            verify(!bar.travelling,
                   "the bar never went up, so nothing comes down: words arriving later grow it unanimated")
            // The pill takes the focus a tick after the opening (`AskBar.onOpenChanged`).
            tryVerify(() => tc.focusInside(bar), undefined, "the pill has the keys")
            tc.pressKeys(Metrics.holdPressesWarned)
            compare(tc.confirms, 2, "and its presses count at once, inside the first answer's floor or not")
        }

        /// Where a hand presses the run bar's button: just inside its bottom edge (`RebasePlanRunBar`'s anchors).
        function pressRun(times) {
            for (let i = 0; i < times; i++)
                mouseClick(runBar, runBar.width / 2, runBar.height - Theme.spaceSm - Theme.spaceXs)
        }

        /// `A` dropped and `B` kept at the last press, the rows turned round on the floor: what runs is what was
        /// pressed for, given once — the bar hands over its ticket and never reads the plan again.
        function test_a_held_run_is_taken_at_the_last_press_and_given_once_after_the_floor() {
            tc.pressRun(Metrics.holdPressesWarned)
            compare(plan.accepts, 1, "taken at the last press")
            verify(plan.locked, "the plan locked behind it — which greys the button, and calls nothing off")
            compare(plan.ran, [], "and not run before the full fill is seen")
            plan.steps = ["pick A", "drop B"]
            tryVerify(() => plan.ran.length === 1, undefined, "run once the floor has passed")
            compare(plan.ran, [["drop A", "pick B"]], "as it stood at the press")
            tc.outlastTheFloor()
            compare(plan.ran.length, 1, "and once")
        }

        /// The plan put away on the floor (the tip moved, the tab left its worktree) takes the answer with it; the
        /// bar's ticket then gives nothing, and the bar never falls back on the plan's own run.
        function test_a_plan_put_away_on_the_floor_runs_nothing() {
            tc.pressRun(Metrics.holdPressesWarned)
            plan.taken = null
            plan.locked = false
            tc.outlastTheFloor()
            compare(plan.ran, [])
        }

        /// The plan pane's `Discard`, found as a hand finds it: the button saying so.
        function discardButton() {
            const walk = item => {
                if (item.text === "Discard" && item.holdPresses !== undefined)
                    return item
                for (let i = 0; i < item.children.length; i++) {
                    const found = walk(item.children[i])
                    if (found !== null)
                        return found
                }
                return null
            }
            return walk(planPane)
        }

        function pressDiscard(times) {
            const button = tc.discardButton()
            verify(button !== null, "the pane has its Discard")
            for (let i = 0; i < times; i++)
                mouseClick(button, button.width / 2, button.height / 2)
        }

        /// A composed plan's `Discard`: taken at the last press, the plan locked behind it, and given once the full
        /// fill has been seen — never through the pane's plain way out.
        function test_a_held_discard_is_taken_at_the_last_press_and_given_once_after_the_floor() {
            tc.pressDiscard(Metrics.holdPressesWarned)
            compare(draft.accepts, 1, "taken at the last press")
            verify(draft.locked)
            compare(draft.discarded, 0, "and not given before the full fill is seen")
            tryVerify(() => draft.discarded === 1, undefined, "given once the floor has passed")
            tc.outlastTheFloor()
            compare(draft.discarded, 1, "and once")
            compare(draft.cancels, 0)
        }

        function test_a_plan_put_away_on_the_discards_floor_gives_nothing() {
            tc.pressDiscard(Metrics.holdPressesWarned)
            draft.taken = false
            draft.locked = false
            tc.outlastTheFloor()
            compare(draft.discarded, 0)
            compare(draft.cancels, 0)
        }

        /// A removal given by its row's floor goes once: the list's own floor finds nothing left to give.
        function test_a_removal_is_given_once() {
            removals.accept("a@example.com", "file:///a.png")
            compare(tc.removed, [], "not at the last press")
            removals.give("a@example.com")
            removals.give("a@example.com")
            tc.outlastTheFloor()
            compare(tc.removed, ["a@example.com"])
        }

        /// The row rebuilt away before its floor passed: the list gives the removal itself.
        function test_a_removal_whose_row_went_is_given_by_the_list() {
            removals.accept("a@example.com", "file:///a.png")
            tryVerify(() => tc.removed.length === 1, undefined, "given on the list's floor")
            compare(tc.removed, ["a@example.com"])
        }

        /// The person given another picture under the floor: the removal is struck off for good — the picture coming
        /// back does not revive it, and the new one is not taken off in its place.
        function test_a_removal_whose_picture_changed_is_struck_off_for_good() {
            removals.accept("a@example.com", "file:///a.png")
            removals.filed = [{ email: "a@example.com", url: "file:///other.png" }, root.twoFiled[1]]
            removals.filed = root.twoFiled
            removals.give("a@example.com")
            tc.outlastTheFloor()
            compare(tc.removed, [])
        }
    }
}
