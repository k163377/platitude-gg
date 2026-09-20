import QtQuick
import QtTest
import platitude.ui

// What the two identity boxes answer about themselves against what git holds (`IdentityFields`): the difference a
// settings Save is lit by, the touch the way out stops for, and the boxes following git while nobody has typed.
//
// **Three questions, read by three buttons.** `differs` is the plain difference, trimmed the way git trims; `dirty`
// is that difference *and* a keystroke; `filled` is both boxes holding something. The settings Saves read the
// first, the way out of the settings screen reads the second, the gate reads the third — so each of them going
// wrong is a different button lit over the wrong boxes, and the rule is fixed here where all three can be read
// off one component with no git behind it.
Item {
    id: root
    width: 600
    height: 400

    readonly property string heldName: "Ada Lovelace"
    readonly property string heldEmail: "ada@example.com"

    /// git's answer as the application delivers it: **one object, one notification, both values**
    /// (`AppBackend.identityName` / `identityEmail` share `identity_changed`). The pair is what the boxes follow, and
    /// a form that followed them one at a time would be tested here in a shape the application never produces.
    QtObject {
        id: answer
        /// `"<name>|<email>"`, so that one write moves both — which is what the backend's single signal does.
        property string pair: "|"
        readonly property string name: answer.pair.split("|")[0]
        readonly property string email: answer.pair.split("|")[1]
    }

    IdentityFields {
        id: fields
        width: root.width
        heldName: root.heldName
        heldEmail: root.heldEmail
        // What the settings screen reads off the backend. There is none here, and none of these is under test.
        nameMarked: false
        emailMarked: false
        errorText: ""
    }

    /// A second form, built after the answer's shape is in place, so the test can watch a pair arrive **under a
    /// form that is already standing** — the case the settings screen is in whenever git answers late.
    Component {
        id: pair
        IdentityFields {
            width: root.width
            heldName: answer.name
            heldEmail: answer.email
            // The three the backend answers for, handed over here too: left to their defaults they reach for a
            // singleton this runner does not register, and the warnings would be this file's.
            nameMarked: false
            emailMarked: false
            errorText: ""
        }
    }

    TestCase {
        name: "IdentityFields"
        when: windowShown

        /// Back to git's answer, the way the screen opening does it.
        function init() {
            fields.heldName = root.heldName
            fields.heldEmail = root.heldEmail
            fields.load()
        }

        /// Types after whatever the name box holds, through the keyboard — the one road that raises `edited`, which
        /// is what the touch is read off (`TextField.textEdited` is not raised by text set in code).
        function typeName(text) {
            fields.focusName()
            keyClick(Qt.Key_End)
            for (const ch of text)
                keyClick(ch)
        }

        function test_boxes_loaded_from_git_differ_in_nothing() {
            compare(fields.nameText, root.heldName)
            compare(fields.emailText, root.heldEmail)
            compare(fields.filled, true)
            compare(fields.differs, false)
            compare(fields.dirty, false)
            compare(fields.touched, false)
        }

        function test_a_keystroke_is_a_difference_and_an_edit() {
            typeName("x")
            compare(fields.nameText, root.heldName + "x")
            compare(fields.touched, true)
            compare(fields.differs, true)
            compare(fields.dirty, true)
        }

        /// git trims before it writes and trims what it reports, so a space after the name is not a difference a
        /// Save could make — lit, it would go out and change nothing.
        function test_a_space_after_the_name_is_no_difference_git_would_keep() {
            typeName(" ")
            compare(fields.nameText, root.heldName + " ")
            compare(fields.touched, true)
            compare(fields.differs, false)
            compare(fields.dirty, false)
        }

        function test_loading_again_takes_the_edit_and_the_touch_back() {
            typeName("x")
            fields.load()
            compare(fields.nameText, root.heldName)
            compare(fields.touched, false)
            compare(fields.differs, false)
            compare(fields.dirty, false)
        }

        /// git answers late and answers again; boxes nobody has typed in follow it, and stay no difference.
        function test_boxes_nobody_typed_in_follow_a_late_answer() {
            fields.heldName = "Grace Hopper"
            compare(fields.nameText, "Grace Hopper")
            compare(fields.differs, false)
            compare(fields.dirty, false)
        }

        function test_boxes_somebody_typed_in_keep_the_edit_over_a_late_answer() {
            typeName("x")
            fields.heldName = "Grace Hopper"
            compare(fields.nameText, root.heldName + "x")
            compare(fields.differs, true)
            compare(fields.dirty, true)
        }

        /// The gate's case: git holds nothing, so the empty pair is no difference and not filled — and typing one
        /// box is a difference the gate still refuses, since its Save wants both.
        function test_an_empty_pair_over_nothing_held_is_no_difference_and_not_filled() {
            fields.heldName = ""
            fields.heldEmail = ""
            fields.load()
            compare(fields.filled, false)
            compare(fields.differs, false)
            compare(fields.dirty, false)
            typeName("Ada")
            compare(fields.filled, false)
            compare(fields.differs, true)
            compare(fields.dirty, true)
        }

        /// **git's answer arriving after the form is standing, both halves at once** — the application's own shape
        /// (`AppBackend` notifies for the pair), and the one a screen opened before git answered lives through.
        /// What must hold afterwards is what holds at rest: the boxes say what git says, and nothing differs — a
        /// Save lit here offers to hand git the answer it just gave.
        function test_an_answer_that_arrives_for_both_halves_at_once_leaves_no_difference() {
            const pairFields = pair.createObject(root, {})
            verify(pairFields !== null, "the pair-fed form is built")
            compare(pairFields.differs, false)
            answer.pair = "Grace Hopper|grace@example.com"
            compare(pairFields.nameText, "Grace Hopper")
            compare(pairFields.emailText, "grace@example.com")
            compare(pairFields.touched, false)
            compare(pairFields.filled, true)
            compare(pairFields.differs, false)
            // What the Saves read. **False because nobody typed** — the half that holds whatever the comparison
            // above makes of two strings arriving a moment apart.
            compare(pairFields.dirty, false)
            pairFields.destroy()
        }

        /// An emptied box is a difference — the repository chapter's Save takes an override out on it — and not
        /// filled, which is what keeps the global chapter's Save dark over the same boxes.
        function test_an_emptied_box_is_a_difference_but_not_filled() {
            fields.focusName()
            keySequence(StandardKey.SelectAll)
            keyClick(Qt.Key_Delete)
            compare(fields.nameText, "")
            compare(fields.touched, true)
            compare(fields.filled, false)
            compare(fields.differs, true)
            compare(fields.dirty, true)
        }
    }
}
