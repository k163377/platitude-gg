import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// Author-identity form: opens on startup when git has no name and
// email to put on a commit, and on demand from the app menu or the
// toolbar badge. The owner decides when to open and close it (the
// dialog reports dismissal); "Not now" leaves the app fully usable —
// reading a repository needs no identity.
AppDialog {
    id: identityDialog

    // Changes the cancel wording: editing an identity that exists vs
    // being asked for one on first run.
    property bool editing: false
    signal dismissed()

    onClosed: identityDialog.dismissed()

    // A write is in flight that this dialog asked for. Only then does
    // a finished write close it — the notification is shared with the
    // startup check, which must not close the dialog under the user.
    property bool saving: false

    function submit() {
        if (!saveButton.enabled)
            return
        identityDialog.saving = true
        AppBackend.saveIdentity(nameField.text, emailField.text)
    }

    onOpened: {
        nameField.text = AppBackend.identityName
        emailField.text = AppBackend.identityEmail
        nameField.forceActiveFocus()
        if (AppBackend.autoIdentity !== "")
            Qt.callLater(identityDialog.applyAutoIdentity)
    }

    // Screenshot hook: PG_AUTO_IDENTITY="<name>|<email>" fills the
    // fields, PG_AUTO_IDENTITY_SAVE=1 submits them, "skip" answers
    // "Not now" to show the state behind the dialog, and "edit" leaves
    // an identity that is already set as it is.
    function applyAutoIdentity() {
        if (AppBackend.autoIdentity === "skip") {
            identityDialog.close()
            return
        }
        if (AppBackend.autoIdentity === "edit")
            return
        const parts = AppBackend.autoIdentity.split("|")
        nameField.text = parts[0]
        emailField.text = parts.length > 1 ? parts[1] : ""
        if (AppBackend.autoIdentitySave)
            identityDialog.submit()
    }

    Connections {
        target: AppBackend
        function onIdentityChanged() {
            if (!identityDialog.saving || AppBackend.identityBusy)
                return
            identityDialog.saving = false
            // Closing on "git raised nothing" would close over a write
            // that only half landed — both keys are set either way, so
            // the answer is what git reports for each of them.
            if (AppBackend.identityNameSaved && AppBackend.identityEmailSaved)
                identityDialog.close()
        }
    }

    contentItem: ColumnLayout {
        spacing: Theme.spaceLg

        Label {
            text: AppBackend.identityState === "ready" ? qsTr("Your identity")
                                                       : qsTr("Set up your identity")
            font.pixelSize: Theme.fontXl
            font.weight: Font.DemiBold
        }
        Label {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            color: Theme.textSecondary
            text: qsTr("git records a name and an email address on every commit you make. They are stored in your git configuration — Platitude GG keeps no copy of them.")
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            Label {
                text: qsTr("Name")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
            }
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceSm
                FormField {
                    id: nameField
                    Layout.fillWidth: true
                    placeholderText: qsTr("Ada Lovelace")
                    onAccepted: identityDialog.submit()
                    // Not validation — these are the characters git drops
                    // when it builds an author line, and keeping them out
                    // stops the configuration from disagreeing with what
                    // commits show. Everything else is the user's business.
                    validator: RegularExpressionValidator {
                        regularExpression: /[^<>\r\n]*/
                    }
                }
                // The mark is only ever seen next to a field whose
                // neighbour has none: a save where both landed closes
                // this dialog. `opacity` keeps the field the same width
                // whether or not it is showing.
                NavIcon {
                    Layout.alignment: Qt.AlignVCenter
                    kind: "check"
                    tint: Theme.success
                    opacity: AppBackend.identityUnsaved
                             && AppBackend.identityNameSaved ? 1 : 0
                }
            }
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            Label {
                text: qsTr("Email address")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
            }
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceSm
                FormField {
                    id: emailField
                    Layout.fillWidth: true
                    placeholderText: qsTr("ada@example.com")
                    onAccepted: identityDialog.submit()
                    validator: RegularExpressionValidator {
                        regularExpression: /[^<>\r\n]*/
                    }
                }
                NavIcon {
                    Layout.alignment: Qt.AlignVCenter
                    kind: "check"
                    tint: Theme.success
                    opacity: AppBackend.identityUnsaved
                             && AppBackend.identityEmailSaved ? 1 : 0
                }
            }
        }

        Label {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            color: Theme.textMuted
            font.pixelSize: Theme.fontSm
            text: qsTr("Saved for every repository on this computer (user.name and user.email).")
        }
        // git's own message, unedited — and where a write took nowhere
        // without git raising anything, a sentence of our own, because
        // there is no message to pass through and the marks alone do not
        // say why one of them is missing.
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

        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: Theme.spaceSm
            ActionButton {
                implicitHeight: Theme.controlHeight
                activeFocusOnTab: true
                text: identityDialog.editing ? qsTr("Cancel") : qsTr("Not now")
                onActivated: identityDialog.close()
            }
            ActionButton {
                id: saveButton
                implicitHeight: Theme.controlHeight
                kind: "check"
                besideWord: true
                frameColor: enabled ? Theme.accent : Theme.borderDefault
                activeFocusOnTab: true
                text: AppBackend.identityBusy ? qsTr("Saving…") : qsTr("Save")
                enabled: !AppBackend.identityBusy
                         && nameField.text.trim() !== ""
                         && emailField.text.trim() !== ""
                onActivated: identityDialog.submit()
            }
        }
    }
}
