import QtQuick
import QtTest
import platitude.ui

// A question and a report taking turns over the middle of the page, wired as `RepoPage` wires them (`AskBar.held`,
// `givesWay`, `enterFrom`): one is on screen at a time, and one hands the middle to the other in place — the one going
// goes in one frame, and the one coming stands at its height in that same frame (`BarTravel`), so what lies under the
// two never jumps — a question answered and on its way back up, too (`AskBar.handingOver`). A pill being pressed
// through, and its full fill after the last press, keep the middle: a report waits for them (`AskBar.answering`). A
// question waiting behind a report has nothing of it on screen and takes no key — its form is not built, so the box
// cannot take the keyboard from the report's `OK` — and one a report came over comes back with what was typed, the
// keyboard back in its box. Alone in its window: the openings are focus, and another test moving the focus would clear
// them.
Item {
    id: root
    width: 400
    height: 400

    NoticeBar {
        id: notice
        width: root.width
        label: "Nothing was pushed"
        detail: "The remote said no."
        givesWay: bar.open
        onAcknowledged: tc.hideReport()
    }
    AskBar {
        id: bar
        y: 200
        width: root.width
        label: "push feature/new-thing where?"
        accept: "push"
        neutral: true
        held: !notice.shut
        givesWay: tc.bandWaits || tc.reportWaits
        onConfirmed: tc.answer()
    }
    SignalSpy {
        id: handOvers
        target: bar
        signalName: "handingOver"
    }
    Component {
        id: publishForm
        PublishForm {
            choices: ["origin"]
            remote: "origin"
            branch: "feature/new-thing"
        }
    }
    /// Somewhere else for the focus to go between the cases.
    Item { id: elsewhere }

    TestCase {
        id: tc
        name: "AskHeld"
        when: windowShown

        /// The page's doors (`RepoPage.showNotice` / `raiseNotice` / `askHandsOver` / `hideNotice`): each bar coming
        /// takes the place of the one on screen at its height — of the question, whatever of it is still there. A
        /// report waits while an answer is under way on the question's pill (`AskBar.answering`), the question giving
        /// way to it (`givesWay`), and comes as that is over: at the height the question hands over, or over it still
        /// standing.
        property bool reportWaits: false
        /// The discard band waiting behind the question.
        property bool bandWaits: false
        function raiseReport() {
            tc.reportWaits = !notice.open && bar.answering
            if (!tc.reportWaits)
                tc.bringReport()
        }
        function bringReport(from) {
            tc.reportWaits = false
            if (!notice.open)
                notice.enterFrom = from !== undefined ? from : !bar.shut ? bar.height : 0
            notice.open = true
        }
        readonly property Connections answerWatch: Connections {
            target: bar
            function onAnsweringChanged() {
                if (!bar.answering && tc.reportWaits)
                    tc.bringReport()
            }
        }
        /// Where the report stood as the question handed the middle over (-1: not up), read in that turn — the report
        /// travels to its own height from the next frame on.
        property real reportAtHandOver: -1
        /// The band took the middle the question handed over.
        property bool bandCame: false
        readonly property Connections handWatch: Connections {
            target: bar
            // `RepoPage.askHandsOver`: the report waiting for that answer first, else the band.
            function onHandingOver(height) {
                if (tc.reportWaits)
                    tc.bringReport(height)
                else if (tc.bandWaits && notice.shut)
                    tc.bandCame = true
                tc.reportAtHandOver = notice.open ? notice.implicitHeight : -1
            }
        }
        /// What the owner does with the answer (`RepoPage.answerRowAsk`): the question goes down — on its pill's floor,
        /// pressed through — and, where a case asks for it, the write's report arrives then, and another question is
        /// raised on that floor, dressed as `GraphPane.startAsking` dresses it: its form, its count, then `open`.
        property bool reportsOnAnswer: false
        property bool asksAgainOnAnswer: false
        property bool holdAgain: true
        property Component formAgain: null
        property bool onFloorAtAnswer: false
        property int handOversAtAnswer: -1
        property bool reportAtAnswer: false
        function answer() {
            bar.open = false
            tc.onFloorAtAnswer = bar.onFloor
            tc.handOversAtAnswer = handOvers.count
            if (tc.reportsOnAnswer)
                tc.raiseReport()
            tc.reportAtAnswer = notice.open
            if (tc.asksAgainOnAnswer) {
                bar.form = null
                bar.form = tc.formAgain
                bar.hold = tc.holdAgain
                bar.open = true
            }
        }
        /// The pill pressed through from the keyboard, as many times as it asks.
        function pressThrough() {
            tryVerify(() => bar.settled && bar.pill !== null && bar.pill.activeFocus)
            const presses = bar.holdPresses
            for (let i = 0; i < presses; i++)
                keyClick(Qt.Key_Space)
        }
        function hideReport() {
            if (bar.open)
                bar.enterFrom = notice.height
            notice.open = false
        }
        /// The two bars' heights together: where what lies under them stands.
        function stack() {
            return notice.implicitHeight + bar.implicitHeight
        }

        function init() {
            bar.open = false
            notice.open = false
            elsewhere.forceActiveFocus()
            tryVerify(() => bar.shut && notice.shut && !bar.onFloor)
            bar.form = null
            bar.hold = false
            tc.bandWaits = false
            tc.reportsOnAnswer = false
            tc.asksAgainOnAnswer = false
            tc.holdAgain = true
            tc.formAgain = null
            tc.reportWaits = false
            tc.reportAtHandOver = -1
            tc.bandCame = false
            handOvers.clear()
        }

        /// The report standing, and holding the keyboard.
        function standReport() {
            raiseReport()
            tryVerify(() => notice.settled && notice.pill !== null && notice.pill.activeFocus)
        }

        function test_a_question_asked_under_a_report_takes_its_place_once_read() {
            standReport()
            bar.form = publishForm
            bar.open = true
            verify(!bar.shown)
            compare(bar.implicitHeight, 0)
            compare(bar.formItem, null, "no box to take the keyboard from the report")
            verify(notice.pill.activeFocus, "the report keeps the keyboard")
            verify(!bar.escapes)
            verify(notice.escapes, "one Escape alive, the report's")

            const before = tc.stack()
            notice.dismiss()
            verify(notice.shut, "the report goes in one frame")
            verify(bar.shown)
            compare(bar.implicitHeight, before, "the question at the report's height: nothing under them moved")
            tryVerify(() => bar.settled)
            tryVerify(() => bar.formItem !== null && bar.formItem.branchField.activeFocus, undefined,
                      "the keyboard in the box, as on any opening with a form")
            verify(bar.escapes)
            verify(!notice.escapes)
        }

        function test_a_report_takes_a_standing_questions_place_and_gives_it_back_with_what_was_typed() {
            bar.form = publishForm
            bar.open = true
            tryVerify(() => bar.settled && bar.formItem !== null && bar.formItem.branchField.activeFocus)
            keyClick(Qt.Key_X)
            const form = bar.formItem
            const typed = form.branchField.text
            verify(typed !== "feature/new-thing", "something was typed")

            const before = tc.stack()
            raiseReport()
            verify(bar.shut, "the question goes in one frame")
            verify(bar.open, "still standing")
            compare(tc.stack(), before, "the report at the question's height: nothing under them moved")
            tryVerify(() => notice.settled)
            verify(!bar.escapes)
            verify(notice.escapes)
            verify(!form.branchField.enabled, "the box out of sight takes no key — Tab from the report's OK passes it")

            const read = tc.stack()
            notice.dismiss()
            compare(tc.stack(), read, "back at the report's height")
            tryVerify(() => bar.settled)
            compare(bar.formItem, form, "the same form, kept")
            compare(bar.formItem.branchField.text, typed)
            tryVerify(() => bar.formItem.branchField.activeFocus, undefined, "the keyboard back in the box")
        }

        function test_a_question_without_a_form_hands_its_pill_the_opening_once_let_go() {
            standReport()
            bar.open = true
            verify(!bar.shown)
            notice.dismiss()
            tryVerify(() => bar.settled && bar.pill !== null && bar.pill.activeFocus)
            verify(bar.pill.tookTheOpening, "the focus is the opening's, so the pill wears no ring")
        }

        // Answered with nothing behind it, the question goes back up; the report its write brought comes over it on the
        // way, and takes its place from where it is.
        function test_a_report_takes_the_place_of_a_question_on_its_way_up() {
            bar.open = true
            tryVerify(() => bar.settled)
            bar.open = false
            verify(!bar.shut && !bar.standing, "on its way up")
            const at = bar.implicitHeight
            const before = tc.stack()
            tc.raiseReport()
            verify(bar.shut, "gone in the frame the report comes")
            compare(tc.stack(), before, "the report where the question was")
            compare(handOvers.count, 1)
            compare(handOvers.signalArguments[0][0], at, "said where it stood")
        }

        // The report a pressed-through answer brought, arriving while the full fill is still on screen: the fill is
        // seen out, and the report takes the question's place as the floor passes.
        function test_a_report_waits_for_a_pills_floor_and_takes_its_place_as_it_passes() {
            bar.hold = true
            bar.open = true
            tc.reportsOnAnswer = true
            tryVerify(() => bar.settled)
            const at = bar.implicitHeight
            tc.pressThrough()
            verify(tc.onFloorAtAnswer, "the report came on the floor")
            verify(!tc.reportAtAnswer, "and waited")
            tryVerify(() => notice.open, undefined, "it comes as the floor passes")
            verify(bar.shut, "the question gone in that frame")
            compare(handOvers.count, 1)
            compare(handOvers.signalArguments[0][0], at)
            compare(tc.reportAtHandOver, at, "the report where the question was")
        }

        // A report arriving between the presses: the answer under way keeps the middle, through the presses and the
        // floor after them.
        function test_a_report_waits_while_the_pill_is_pressed_through() {
            bar.hold = true
            bar.open = true
            tryVerify(() => bar.settled && bar.pill !== null && bar.pill.activeFocus)
            const at = bar.implicitHeight
            const presses = bar.holdPresses
            keyClick(Qt.Key_Space)
            verify(bar.answering)
            tc.raiseReport()
            verify(!notice.open && bar.shown, "the question keeps the middle")
            for (let i = 1; i < presses; i++)
                keyClick(Qt.Key_Space)
            verify(tc.onFloorAtAnswer)
            tryVerify(() => notice.open, undefined, "the report comes as the floor passes")
            verify(bar.shut)
            compare(tc.reportAtHandOver, at, "the report where the question was")
        }

        // A report waiting for an answer, and the discard band behind the question too: the report comes first, at the
        // question's height, as the floor passes.
        function test_a_report_waiting_for_the_floor_comes_before_the_band() {
            tc.bandWaits = true
            bar.hold = true
            bar.open = true
            tc.reportsOnAnswer = true
            tryVerify(() => bar.settled)
            const at = bar.implicitHeight
            tc.pressThrough()
            verify(!tc.reportAtAnswer)
            tryVerify(() => notice.open, undefined, "it comes as the floor passes")
            verify(bar.shut)
            verify(!tc.bandCame, "the band waits behind the report")
            compare(tc.reportAtHandOver, at, "at the question's height")
        }

        // Another question raised on the last one's floor while a report waits for that answer: the report comes, and
        // the new question waits behind it, nothing of it on screen.
        function test_a_question_raised_on_the_floor_waits_behind_the_report_that_waited() {
            bar.hold = true
            bar.open = true
            tc.reportsOnAnswer = true
            tc.asksAgainOnAnswer = true
            tryVerify(() => bar.settled)
            const at = bar.implicitHeight
            tc.pressThrough()
            verify(notice.open, "the report the answer brought comes")
            verify(bar.open && bar.held && !bar.shown)
            verify(bar.shut && !bar.standing, "the new question stands nowhere")
            compare(tc.reportAtHandOver, at)
        }

        // …and a plain one with a form, dressed on that floor: the count it is dressed with is not the answer's, so
        // the report still waits for the floor to be put down as the question is raised. The form is not built
        // behind the report, so the report's `OK` keeps the keyboard.
        function test_a_plain_question_with_a_form_raised_on_the_floor_waits_behind_the_report() {
            bar.hold = true
            bar.open = true
            tc.reportsOnAnswer = true
            tc.asksAgainOnAnswer = true
            tc.holdAgain = false
            tc.formAgain = publishForm
            tryVerify(() => bar.settled)
            const at = bar.implicitHeight
            tc.pressThrough()
            verify(notice.open, "the report the answer brought comes")
            verify(bar.open && bar.held && !bar.shown)
            verify(bar.shut && !bar.standing, "the new question stands nowhere")
            compare(handOvers.count, 1)
            compare(tc.reportAtHandOver, at)
            compare(bar.formItem, null, "no box to take the keyboard from the report")
            tryVerify(() => notice.settled && notice.pill !== null && notice.pill.activeFocus, undefined,
                      "the report holds the keyboard")
        }

        // A plain pill answers at once: a hand down on it keeps nothing waiting, and its release, the pill gone under
        // the report, answers nothing.
        function test_a_hand_down_on_a_plain_pill_keeps_no_report_waiting() {
            bar.open = true
            tryVerify(() => bar.settled && bar.pill !== null)
            mousePress(bar.pill)
            verify(!bar.answering)
            tc.raiseReport()
            verify(notice.open, "the report comes")
            mouseRelease(bar.pill)
            verify(bar.open, "the release answers nothing")
        }

        // A plain pill turned into a count under a hand already down (`PublishFlow.publishRefused`): that hand's
        // gesture is still the plain one, and keeps nothing waiting.
        function test_a_hand_down_on_a_pill_turned_to_a_count_keeps_no_report_waiting() {
            bar.open = true
            tryVerify(() => bar.settled && bar.pill !== null)
            mousePress(bar.pill)
            bar.hold = true
            verify(!bar.answering, "the gesture under way is the plain one")
            tc.raiseReport()
            verify(notice.open, "the report comes")
            mouseRelease(bar.pill)
            verify(bar.open, "the release answers nothing")
        }

        // Something waiting behind it (`givesWay`): the question stands through its pill's floor — the full fill is the
        // answer — and hands the middle over as the floor passes, at once.
        function test_a_question_giving_way_hands_over_as_its_pills_floor_passes() {
            tc.bandWaits = true
            bar.hold = true
            bar.open = true
            tryVerify(() => bar.settled)
            const at = bar.implicitHeight
            tc.pressThrough()
            verify(tc.onFloorAtAnswer)
            compare(tc.handOversAtAnswer, 0, "standing through the floor")
            tryVerify(() => handOvers.count === 1, undefined, "handed over as the floor passes")
            verify(bar.shut, "gone in that frame")
            verify(tc.bandCame, "the band in its place")
            compare(handOvers.signalArguments[0][0], at)
        }

        // With nothing behind it, the report goes back up rather than at once: only a hand-over is in place.
        function test_a_report_with_nothing_behind_it_goes_back_up() {
            standReport()
            notice.dismiss()
            verify(!notice.shut, "on its way up")
            tryVerify(() => notice.shut)
        }
    }
}
