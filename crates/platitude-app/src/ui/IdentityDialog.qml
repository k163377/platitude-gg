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
            if (AppBackend.identityError === "")
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
            text: qsTr("git records a name and an email address on every commit you "
                       + "make. They are stored in your git configuration — "
                       + "platitude-gg keeps no copy of them.")
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            Label {
                text: qsTr("Name")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
            }
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
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            Label {
                text: qsTr("Email address")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
            }
            FormField {
                id: emailField
                Layout.fillWidth: true
                placeholderText: qsTr("ada@example.com")
                onAccepted: identityDialog.submit()
                validator: RegularExpressionValidator {
                    regularExpression: /[^<>\r\n]*/
                }
            }
        }

        Label {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            color: Theme.textMuted
            font.pixelSize: Theme.fontSm
            text: qsTr("Saved for every repository on this computer "
                       + "(user.name and user.email).")
        }
        // git's own message, unedited.
        Label {
            Layout.fillWidth: true
            visible: AppBackend.identityError !== ""
            wrapMode: Text.Wrap
            color: Theme.danger
            text: AppBackend.identityError
        }

        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: Theme.spaceSm
            HoverButton {
                text: identityDialog.editing ? qsTr("Cancel") : qsTr("Not now")
                onClicked: identityDialog.close()
            }
            HoverButton {
                id: saveButton
                highlighted: true
                text: AppBackend.identityBusy ? qsTr("Saving…") : qsTr("Save")
                enabled: !AppBackend.identityBusy
                         && nameField.text.trim() !== ""
                         && emailField.text.trim() !== ""
                onClicked: identityDialog.submit()
            }
        }
    }
}
