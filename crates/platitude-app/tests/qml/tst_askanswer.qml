import QtQuick
import QtTest
import platitude.ui

// **Who answers a standing question from the keyboard** (デザイン規約 §立っている質問は 1 か所で聞く). A question the
// pill alone answers hands it the focus as the bar opens; a question with a form leaves the focus in the box being
// typed into, so the form's own Enter has to be the answer — and it has to cost exactly what the pill costs.
//
// **Fixed here because only a real keystroke can say it.** Nothing in the application can press a key: the headless
// runs raise the field's `accepted` themselves, which is the wiring below this line and not the key above it.
Item {
    id: root
    width: 400
    height: 300

    AskBar {
        id: bar
        width: root.width
        label: "push feature/new-thing where?"
        accept: "push"
        neutral: true
        form: publishForm
        open: true
        onConfirmed: tc.confirms++
    }
    Component {
        id: publishForm
        PublishForm {
            choices: ["origin"]
            remote: "origin"
            branch: "feature/new-thing"
        }
    }

    /// The other form the one bar carries, whose name box also offers a list — the half of the rule that belongs to
    /// the box (`AppCombo.submitted`), read here through the bar it answers.
    AskBar {
        id: listBar
        width: root.width
        y: 150
        label: "Upstream of feature/new-thing?"
        accept: "Set"
        neutral: true
        form: upstreamForm
        open: true
        onConfirmed: tc.listConfirms++
    }
    Component {
        id: upstreamForm
        UpstreamForm {
            remotes: ["origin"]
            branches: ["main", "taken", "carried"]
            remote: "origin"
            branch: "main"
        }
    }

    TestCase {
        id: tc
        name: "AskAnswer"
        when: windowShown

        property int confirms: 0
        property int listConfirms: 0

        function init() {
            tc.confirms = 0
            tc.listConfirms = 0
            bar.answerable = true
            bar.hold = false
        }

        /// The box the question is typed into is the box it is answered from.
        function test_return_in_the_name_box_answers_the_question() {
            bar.formItem.branchField.forceActiveFocus()
            keyClick(Qt.Key_Return)
            compare(tc.confirms, 1)
        }

        /// **What the pill cannot do, the key cannot do.** A question with nothing to send yet has a dark pill, and
        /// an Enter that went through anyway would answer a question the screen says is unanswered.
        function test_an_unanswerable_question_is_not_answered_by_the_key() {
            bar.answerable = false
            bar.formItem.branchField.forceActiveFocus()
            keyClick(Qt.Key_Return)
            compare(tc.confirms, 0)
        }

        /// And a question that asks to be held is the pill's alone (§進行中・長押しの定数): the gesture is the
        /// intent, and a keystroke is not one.
        function test_a_held_question_is_not_answered_by_the_key() {
            bar.hold = true
            bar.formItem.branchField.forceActiveFocus()
            keyClick(Qt.Key_Return)
            compare(tc.confirms, 0)
        }

        /// The name box that offers a list answers the same way while the list is not standing in front of it.
        function test_return_in_a_name_box_with_a_list_answers_the_question() {
            listBar.formItem.branchPick.popup.close()
            listBar.formItem.branchPick.contentItem.forceActiveFocus()
            keyClick(Qt.Key_Return)
            compare(tc.listConfirms, 1)
        }

        /// **Under the open list the key is the list's** — the row is picked, and the question stands. Answering
        /// there would answer it with the name the pick is about to replace (`AppCombo.submitted`).
        function test_return_under_the_open_list_picks_instead_of_answering() {
            const box = listBar.formItem.branchPick
            box.contentItem.forceActiveFocus()
            box.popup.open()
            tryCompare(box.popup, "visible", true)
            keyClick(Qt.Key_Down)
            keyClick(Qt.Key_Return)
            compare(tc.listConfirms, 0, "the question still stands")
            compare(box.wanted, "taken", "and the press went to the row the keyboard was on")
        }
    }
}
