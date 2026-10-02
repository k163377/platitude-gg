import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// The startup gate when git has no identity to commit with. It asks for that pair only — the rest of git's
// configuration is in the settings screen (`SettingsDialog`). The owner opens and closes it; "Not now" leaves the app
// usable, since reading a repository needs no identity.
AppDialog {
    id: identityDialog

    signal dismissed()

    onClosed: identityDialog.dismissed()

    // A write this dialog asked for is in flight. Only its finish closes the dialog — `identityChanged` is shared with
    // the startup check.
    property bool saving: false

    function submit() {
        if (!actions.acceptEnabled)
            return
        identityDialog.saving = true
        AppBackend.saveIdentity(fields.nameText, fields.emailText)
    }

    onOpened: {
        fields.load()
        fields.focusName()
    }

    /// Automation-only: fills the two boxes (a run has no keyboard).
    function fill(name, email) {
        fields.nameText = name
        fields.emailText = email
    }

    Connections {
        target: AppBackend
        function onIdentityChanged() {
            if (!identityDialog.saving || AppBackend.identityBusy)
                return
            identityDialog.saving = false
            // Not on "git raised nothing": a half-landed write raises nothing either. Close only when git reports both.
            if (AppBackend.identityNameSaved && AppBackend.identityEmailSaved)
                identityDialog.close()
        }
    }

    contentItem: ColumnLayout {
        spacing: Theme.spaceLg

        Label {
            text: qsTr("Set up your identity")
            font.pixelSize: Theme.fontXl
            font.weight: Theme.fontWeightStrong
        }
        Label {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            color: Theme.textSecondary
            text: qsTr("git records a name and an email address on every commit you make. They are stored in your git configuration — Platitude GG keeps no copy of them.")
        }

        IdentityFields {
            id: fields
            onSubmitted: identityDialog.submit()
        }

        DialogActions {
            id: actions
            cancelText: qsTr("Not now")
            acceptKind: "check"
            acceptText: AppBackend.identityBusy ? qsTr("Saving…") : qsTr("Save")
            acceptEnabled: !AppBackend.identityBusy && fields.filled
            onCancelled: identityDialog.close()
            onAccepted: identityDialog.submit()
        }
    }
}
