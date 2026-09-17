import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// The gate that stands on startup when git has no name and email to put on a commit. It asks for that one thing and
// nothing else — the rest of git's configuration is not what the reader was stopped for, and it is a click away in
// the settings screen's git chapter (`SettingsDialog`), which is where an identity that is already set is edited.
//
// A card: this is a question waiting for an answer. The owner decides when to open and close it (the dialog
// reports dismissal); "Not now" leaves the app fully
// usable — reading a repository needs no identity.
AppDialog {
    id: identityDialog

    signal dismissed()

    onClosed: identityDialog.dismissed()

    // A write is in flight that this dialog asked for. Only then does a finished write close it — the notification is
    // shared with the startup check, whose answer leaves the dialog standing.
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

    /// The two boxes, written from outside. An automation-only exposure, the same one `GraphPane.view` is
    /// (app-ui.md): a run has no keyboard, and what it is filling in is the state a person's typing leaves behind.
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
            // Closing on "git raised nothing" would close over a write that only half landed — both keys are set either
            // way, so the answer is what git reports for each of them.
            if (AppBackend.identityNameSaved && AppBackend.identityEmailSaved)
                identityDialog.close()
        }
    }

    contentItem: ColumnLayout {
        spacing: Theme.spaceLg

        Label {
            text: qsTr("Set up your identity")
            font.pixelSize: Theme.fontXl
            font.weight: Font.DemiBold
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
