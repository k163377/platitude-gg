import QtQuick
import QtTest
import platitude.ui

// Repository names and git's diagnostic strings are data, including when they contain HTML tags: the label every
// input is drawn with (`PlainTextLabel`), the parts built on it, and a box's own placeholder all keep them as written.
Item {
    id: root
    width: 1000
    height: 700

    readonly property string payload: "<b>INJECTED</b>&amp;"

    CodeChip { id: code; word: root.payload }
    CutName { id: cut; y: 60; width: 900; height: 40; text: root.payload }
    RefChip {
        id: ref
        y: 120
        maxWidth: 900
        records: [Object.assign({}, noChip, { name: root.payload })]
    }
    ActionButtonLabel { id: action; y: 180; text: root.payload }
    AppCombo { id: combo; y: 240; width: 900; model: [root.payload] }
    CommandRowDelegate {
        id: command
        y: 300
        width: 900
        index: 0
        clock: "12:00:00"
        args: "clone"
        full: "git clone"
        status: "failed"
        result: "exit 128"
        duration: "1 ms"
        output: root.payload
        sel: ({})
        charW: 8
        commandsModel: null
        ruler: null
    }
    CloneDialog { id: clone }
    PlainTextLabel { id: plainLabel; y: 420; text: root.payload }
    // The message standing behind an empty box (a stopped merge's), drawn by the box's own label, not Qt's.
    SummaryArea { id: summaryBox; y: 460; width: 900; placeholder: root.payload }
    FormField { id: formBox; y: 520; width: 900; placeholder: root.payload }
    DescriptionBox { id: descBox; y: 560; width: 900; height: 80; placeholder: root.payload }
    Text { id: plain; visible: false; textFormat: Text.PlainText }

    TestCase {
        name: "UntrustedText"
        when: windowShown

        function fieldsWithText(item, text) {
            let found = []
            if (item.text === text && item.textFormat !== undefined)
                found.push(item)
            for (const child of item.children || [])
                found = found.concat(fieldsWithText(child, text))
            return found
        }

        function checkSpelling(item, text) {
            const fields = fieldsWithText(item, text)
            verify(fields.length > 0, "the product's text item exists")
            const field = fields[0]
            plain.font = field.font
            plain.text = text
            // The rendered width proves the tags and entity survive; checking the source string alone does not.
            tryCompare(field, "contentWidth", plain.contentWidth)
            for (const drawn of fields)
                compare(drawn.textFormat, Text.PlainText, "both measured and drawn text refuse markup")
        }

        function test_code_name_keeps_tags() {
            checkSpelling(code, root.payload)
        }

        function test_cut_name_keeps_tags() {
            checkSpelling(cut, root.payload)
        }

        function test_ref_name_keeps_tags() {
            checkSpelling(ref, root.payload)
        }

        function test_action_name_keeps_tags() {
            checkSpelling(action, root.payload)
        }

        function test_command_output_keeps_tags() {
            checkSpelling(command, root.payload)
        }

        function test_remote_choices_keep_tags() {
            combo.popup.open()
            tryCompare(combo.popup, "opened", true)
            checkSpelling(combo.popup.contentItem, root.payload)
            combo.popup.close()
        }

        function test_clone_refusal_keeps_tags() {
            const message = "fatal: repository '" + root.payload + "' does not exist"
            clone.said(message)
            checkSpelling(clone.contentItem, message)
        }

        function test_plain_text_label_keeps_tags() {
            checkSpelling(plainLabel, root.payload)
        }

        function test_standing_placeholder_keeps_tags() {
            checkSpelling(summaryBox, root.payload)
        }

        function test_form_placeholder_keeps_tags() {
            checkSpelling(formBox, root.payload)
        }

        function test_description_placeholder_keeps_tags() {
            checkSpelling(descBox, root.payload)
        }

        function test_an_image_tag_stays_text() {
            // An inline image needs no server: the security boundary is refusing markup before any resource load.
            const image = "<img src='data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///ywAAAAAAQABAAACAUwAOw=='>"
            code.word = image
            checkSpelling(code, image)
            code.word = root.payload
        }
    }
}
