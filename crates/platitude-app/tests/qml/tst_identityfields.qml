import QtQuick
import QtTest
import platitude.ui

// What the two identity boxes answer about themselves against what git holds (`IdentityFields`), read off one
// component with no git behind it: `differs` is the plain difference, trimmed as git trims; `dirty` is that
// difference and a keystroke (the settings Saves and the way out read it); `filled` is both boxes holding something
// (the gate reads it).
Item {
    id: root
    width: 600
    height: 400

    readonly property string heldName: "Ada Lovelace"
    readonly property string heldEmail: "ada@example.com"

    /// git's answer as the application delivers it: one notification for both values (`AppBackend.identityName` /
    /// `identityEmail` share `identity_changed`).
    QtObject {
        id: answer
        /// `"<name>|<email>"`, so one write moves both.
        property string pair: "|"
        readonly property string name: answer.pair.split("|")[0]
        readonly property string email: answer.pair.split("|")[1]
    }

    IdentityFields {
        id: fields
        width: root.width
        heldName: root.heldName
        heldEmail: root.heldEmail
        // Left to their defaults these reach for a singleton this runner does not register; none is under test.
        nameMarked: false
        emailMarked: false
        errorText: ""
    }

    /// A form already standing when the pair arrives — the settings screen whenever git answers late.
    Component {
        id: pair
        IdentityFields {
            width: root.width
            heldName: answer.name
            heldEmail: answer.email
            // As in `fields`.
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

        /// Through the keyboard: `textEdited` (what the touch is read off) is not raised by text set in code.
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

        /// git trims what it writes and reports: a Save lit here would change nothing.
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

        /// The gate's case: git holds nothing, and one typed box is a difference the gate still refuses (its Save
        /// wants both).
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

        /// Afterwards the boxes say what git says and nothing differs — a Save lit here would hand git the answer it
        /// just gave.
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
            // What the Saves read: false because nobody typed, whatever the comparison makes of two strings
            // arriving a moment apart.
            compare(pairFields.dirty, false)
            pairFields.destroy()
        }

        /// The repository chapter's Save takes an override out on the difference; not filled keeps the global
        /// chapter's Save dark.
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
