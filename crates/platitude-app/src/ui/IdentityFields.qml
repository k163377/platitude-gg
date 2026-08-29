import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// The name and the email address git puts on a commit, and everything that answers for a write of them: the two
// boxes, the mark beside each, the line saying where they land, and git's own words when one of them did not.
//
// Two screens ask for the same pair — the settings screen's git chapter, and the gate that stands on startup when git
// has no identity to commit with — so the form is one file and the two callers differ only in what surrounds it: a
// chapter with a Save of its own, or a question with `Not now` beside it.
ColumnLayout {
    id: fields

    /// What is typed, for the caller that hands it to git.
    property alias nameText: nameField.text
    property alias emailText: emailField.text
    /// Both boxes hold something. What a Save is enabled by, wherever that button stands.
    readonly property bool filled: nameField.text.trim() !== "" && emailField.text.trim() !== ""

    /// The line under the boxes. The default says how far the write reaches, for the gate, which has nothing above it
    /// that does; the settings screen's warning already says it, so there the line only names the two keys.
    property string note: qsTr("Saved for every repository on this computer (user.name and user.email).")

    /// Enter was pressed in one of the boxes.
    signal submitted()

    /// Fills the boxes from what git answers with now. Called when the screen around them opens.
    function load() {
        nameField.text = AppBackend.identityName
        emailField.text = AppBackend.identityEmail
    }
    function focusName() {
        nameField.forceActiveFocus()
    }

    Layout.fillWidth: true
    spacing: Theme.spaceLg

    LabeledField {
        caption: qsTr("Name")
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceSm
            FormField {
                id: nameField
                Layout.fillWidth: true
                placeholderText: qsTr("Ada Lovelace")
                onAccepted: fields.submitted()
                // Not validation — these are the characters git drops when it builds an author line, and keeping
                // them out stops the configuration from disagreeing with what commits show. Everything else is the
                // user's business.
                validator: RegularExpressionValidator {
                    regularExpression: /[^<>\r\n]*/
                }
            }
            // The mark is only ever seen next to a field whose neighbour has none: a save where both landed leaves
            // nothing for either to say. `opacity` keeps the field the same width whether or not it is showing.
            NavIcon {
                Layout.alignment: Qt.AlignVCenter
                kind: "check"
                tint: Theme.success
                opacity: AppBackend.identityUnsaved && AppBackend.identityNameSaved ? 1 : 0
            }
        }
    }
    LabeledField {
        caption: qsTr("Email address")
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceSm
            FormField {
                id: emailField
                Layout.fillWidth: true
                placeholderText: qsTr("ada@example.com")
                onAccepted: fields.submitted()
                validator: RegularExpressionValidator {
                    regularExpression: /[^<>\r\n]*/
                }
            }
            NavIcon {
                Layout.alignment: Qt.AlignVCenter
                kind: "check"
                tint: Theme.success
                opacity: AppBackend.identityUnsaved && AppBackend.identityEmailSaved ? 1 : 0
            }
        }
    }
    Label {
        Layout.fillWidth: true
        wrapMode: Text.Wrap
        color: Theme.textMuted
        font.pixelSize: Theme.fontSm
        text: fields.note
    }
    // git's own message, unedited — and where a write took nowhere without git raising anything, a sentence of our
    // own, because there is no message to pass through and the marks alone do not say why one of them is missing.
    Label {
        Layout.fillWidth: true
        visible: text !== ""
        wrapMode: Text.Wrap
        color: Theme.danger
        text: AppBackend.identityError !== ""
              ? AppBackend.identityError
              : AppBackend.identityUnsaved
                ? qsTr("git still reports a different identity. A setting in the repository this window was started in can sit over this one.")
                : ""
    }
}
