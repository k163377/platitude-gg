import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The keys a form box answers with, in both shapes (`FormField`): a typing box takes Space into its text and raises
// `accepted` on Return, as the plain `TextField` beside it (the control) does; a choosing box answers the same keys
// with `picking` and puts nothing into its text.
// Both shapes share one attached `Keys`, and a handler for a named key accepts the event whatever its body does — a
// box that only asked whether it was choosing would swallow every Space and Return typed into a name.
Item {
    id: root
    width: 400
    height: 200

    Column {
        TextField {
            id: plain
            width: root.width
        }
        FormField {
            id: typing
            width: root.width
        }
        FormField {
            id: choosing
            width: root.width
            choosing: true
        }
    }

    TestCase {
        id: tc
        name: "FormField"
        when: windowShown

        property int plainAccepted: 0
        property int typingAccepted: 0
        property int picks: 0

        // Named through the id: a `Connections` function sees the target's scope, not the test's.
        Connections {
            target: plain
            function onAccepted() { tc.plainAccepted++ }
        }
        Connections {
            target: typing
            function onAccepted() { tc.typingAccepted++ }
        }
        Connections {
            target: choosing
            function onPicking() { tc.picks++ }
        }

        function init() {
            plain.text = ""
            typing.text = ""
            choosing.text = ""
            tc.plainAccepted = 0
            tc.typingAccepted = 0
            tc.picks = 0
        }

        function test_a_plain_box_takes_a_space() {
            plain.forceActiveFocus()
            keyClick("a")
            keyClick(" ")
            keyClick("b")
            compare(plain.text, "a b")
        }

        function test_a_typing_box_takes_a_space_the_same_way() {
            typing.forceActiveFocus()
            keyClick("a")
            keyClick(" ")
            keyClick("b")
            compare(typing.text, "a b")
        }

        function test_return_in_a_plain_box_is_accepted() {
            plain.forceActiveFocus()
            keyClick("a")
            keyClick(Qt.Key_Return)
            compare(plainAccepted, 1)
        }

        function test_return_in_a_typing_box_is_accepted_the_same_way() {
            typing.forceActiveFocus()
            keyClick("a")
            keyClick(Qt.Key_Return)
            compare(typingAccepted, 1)
            keyClick(Qt.Key_Enter)
            compare(typingAccepted, 2)
        }

        function test_a_choosing_box_answers_space_and_return_with_picking() {
            choosing.forceActiveFocus()
            keyClick(" ")
            keyClick(Qt.Key_Return)
            keyClick(Qt.Key_Enter)
            compare(picks, 3)
            compare(choosing.text, "")
        }
    }
}
