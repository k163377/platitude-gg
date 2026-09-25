import QtQuick
import QtTest
import platitude.ui

// Who answers a standing question from the keyboard (デザイン規約 §立っている質問は 1 か所で聞く): with a form, the
// box's Enter, at exactly what the pill costs. Only a real keystroke can fix this — the headless runs raise the
// field's `accepted` themselves.
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

    /// A form whose name box also offers a list (`AppCombo.submitted`).
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

        function test_return_in_the_name_box_answers_the_question() {
            bar.formItem.branchField.forceActiveFocus()
            keyClick(Qt.Key_Return)
            compare(tc.confirms, 1)
        }

        function test_an_unanswerable_question_is_not_answered_by_the_key() {
            bar.answerable = false
            bar.formItem.branchField.forceActiveFocus()
            keyClick(Qt.Key_Return)
            compare(tc.confirms, 0)
        }

        function test_a_held_question_is_not_answered_by_the_key() {
            bar.hold = true
            bar.formItem.branchField.forceActiveFocus()
            keyClick(Qt.Key_Return)
            compare(tc.confirms, 0)
        }

        function test_return_in_a_name_box_with_a_list_answers_the_question() {
            listBar.formItem.branchPick.popup.close()
            listBar.formItem.branchPick.contentItem.forceActiveFocus()
            keyClick(Qt.Key_Return)
            compare(tc.listConfirms, 1)
        }

        /// Under the open list the key picks the row: answering there would carry the name the pick replaces
        /// (`AppCombo.submitted`).
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
