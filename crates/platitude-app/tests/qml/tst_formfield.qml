import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The keys a form box answers with, in both of its shapes (`FormField`): a typing box has to take a Space into its
// text and raise `accepted` on Return, exactly as a plain `TextField` does; a choosing box answers the same keys
// with `picking` and puts nothing into its text.
//
// **Fixed here because the two shapes share one component and one attached `Keys`.** A handler written for a named
// key accepts the event the moment it is connected, whatever its body does — so a box that only *asked* whether it
// was choosing would swallow every Space and Return typed into a name (observed: a space typed after a name left
// the text as it was). The plain `TextField` beside it is the control: what Qt does with the same keystrokes when
// nothing stands in front of the box.
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

        /// The control: a plain box takes the space between two words.
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

        /// The control: Return in a plain box is `accepted`.
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

        /// A choosing box answers Space and Return with the door, and its text stays what it was handed.
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
