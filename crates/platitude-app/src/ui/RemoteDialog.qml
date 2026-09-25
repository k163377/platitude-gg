import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Where a remote is written down — a name and a URL — or its URL corrected (デザイン規約 §リモートを書き留める). A
// dialog, because this is repository configuration (デザイン規約 §可否・警告の出し場所).
AppDialog {
    id: remoteDialog

    /// The remote being corrected; empty means one is being made.
    property string editing: ""
    /// Names this repository already has — git's own refusal of a duplicate would land after the dialog closed.
    property var taken: []
    /// Whether this remote is origin (`RepoTab.markedOrigin`), and whether that mark is this repository's own: one
    /// set for every repository cannot be cleared here, git having no local spelling for "not set".
    property bool marked: false
    property bool markLocal: true

    readonly property string wantedName: nameField.text.trim()
    readonly property string wantedUrl: urlField.text.trim()
    readonly property bool nameClashes:
        remoteDialog.editing === "" && remoteDialog.wantedName !== ""
        && remoteDialog.taken.indexOf(remoteDialog.wantedName) >= 0

    /// The remote was written down, the URL as typed: `git remote add` contacts nothing.
    signal submitted(string name, string url)
    /// The box was left changed. Its own signal: it writes config keys, not the remote.
    signal markChanged(string name, bool marked)

    function start(name, url, takenNames, marked, markLocal) {
        remoteDialog.editing = name
        remoteDialog.taken = takenNames
        remoteDialog.marked = marked === true
        remoteDialog.markLocal = markLocal !== false
        markBox.checked = remoteDialog.marked
        // `origin` only while there is no remote at all (デザイン規約 §リモートを書き留める).
        nameField.text = name !== "" ? name : takenNames.length === 0 ? "origin" : ""
        urlField.text = url
        remoteDialog.open()
    }
    // The caret goes to the URL, unless the name is left to type.
    onOpened: (remoteDialog.editing === "" && nameField.text === "" ? nameField : urlField).forceActiveFocus()

    /// Automation: typing, which no injected key reaches offscreen. An empty half leaves what `start` put there.
    function setFields(name, url) {
        if (name !== "")
            nameField.text = name
        if (url !== "")
            urlField.text = url
    }

    function submit() {
        if (!actions.acceptEnabled)
            return
        const name = remoteDialog.editing !== "" ? remoteDialog.editing : remoteDialog.wantedName
        const url = remoteDialog.wantedUrl
        const marked = markBox.checked
        remoteDialog.close()
        remoteDialog.submitted(name, url)
        // After `submitted`: on the add form the remote exists only once that has run, and the write queue keeps
        // the order.
        if (markBox.offered && marked !== remoteDialog.marked)
            remoteDialog.markChanged(name, marked)
    }

    contentItem: ColumnLayout {
        spacing: Theme.spaceLg

        Label {
            text: remoteDialog.editing === "" ? qsTr("Add remote") : qsTr("Where %1 is").arg(remoteDialog.editing)
            font.pixelSize: Theme.fontXl
            font.weight: Font.DemiBold
        }
        Label {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            color: Theme.textSecondary
            // Shared by the add and correction forms (デザイン規約 §リモートを書き留める).
            text: qsTr("Paste the URL of the repository on your host.")
        }

        // A corrected remote keeps its name: renaming belongs to the left menu.
        LabeledField {
            visible: remoteDialog.editing === ""
            caption: qsTr("Name")
            FormField {
                id: nameField
                Layout.fillWidth: true
                // Never a name Add would refuse (デザイン規約 §リモートを書き留める).
                placeholderText: remoteDialog.taken.indexOf("origin") >= 0 ? "" : "origin"
                onAccepted: remoteDialog.submit()
            }
            Label {
                visible: remoteDialog.nameClashes
                text: qsTr("This repository already has a remote called that")
                color: Theme.warning
                font.pixelSize: Theme.fontSm
            }
        }
        LabeledField {
            caption: qsTr("URL")
            FormField {
                id: urlField
                Layout.fillWidth: true
                placeholderText: qsTr("git@github.com:you/your-repo.git")
                onAccepted: remoteDialog.submit()
            }
        }

        // Which remote is origin (デザイン規約 §リモートを書き留める). Not on the first remote's add form: every push
        // goes there anyway.
        ColumnLayout {
            id: markRow
            Layout.fillWidth: true
            spacing: 0
            visible: markBox.offered
            AppCheckBox {
                id: markBox
                readonly property bool offered: remoteDialog.editing !== "" || remoteDialog.taken.length > 0
                text: qsTr("Mark as origin (default remote)")
                enabled: remoteDialog.markLocal || !remoteDialog.marked
            }
            Label {
                Layout.fillWidth: true
                leftPadding: Theme.spaceXl
                wrapMode: Text.Wrap
                font.pixelSize: Theme.fontSm
                color: markBox.enabled ? Theme.textSecondary : Theme.warning
                text: !markBox.enabled
                      ? qsTr("Set for every repository — marking another remote is what moves it.")
                      : markBox.checked
                        ? qsTr("Each branch goes back to pushing where it tracks.")
                        : qsTr("Every branch pushes here, whatever it tracks.")
            }
        }

        DialogActions {
            id: actions
            cancelText: Words.cancel
            acceptKind: remoteDialog.editing === "" ? "plus" : "check"
            acceptText: remoteDialog.editing === "" ? qsTr("Add") : qsTr("Save")
            acceptEnabled: remoteDialog.wantedUrl !== ""
                           && (remoteDialog.editing !== ""
                               || (remoteDialog.wantedName !== ""
                                   && !remoteDialog.nameClashes))
            onCancelled: remoteDialog.close()
            onAccepted: remoteDialog.submit()
        }
    }
}
