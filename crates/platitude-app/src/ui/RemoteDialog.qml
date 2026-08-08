import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Where a remote is written down: a name and a URL.
//
// A window of its own rather than the standing question's bar, because
// this is not a question about what is on screen — it is repository
// configuration, the same kind of thing as identity, which
// デザイン規約 §可否・警告の出し場所 keeps in a popup. Putting it in the
// bar also put a remote's own name and URL on a second row while the
// branch name stayed on the first, which grouped them backwards.
//
// The same form corrects a URL. Adding one and fixing one differ by which
// half is filled in already, not by what has to be typed.
AppDialog {
    id: remoteDialog

    /// The remote being corrected; empty means one is being made.
    property string editing: ""
    /// Names this repository already has. git refuses a duplicate itself
    /// (`remote <name> already exists`, exit 3), but that refusal would
    /// arrive after the dialog had closed, with nothing on screen left for
    /// it to be about.
    property var taken: []

    readonly property string wantedName: nameField.text.trim()
    readonly property string wantedUrl: urlField.text.trim()
    readonly property bool nameClashes:
        remoteDialog.editing === "" && remoteDialog.wantedName !== ""
        && remoteDialog.taken.indexOf(remoteDialog.wantedName) >= 0

    /// The remote was written down. The URL is not judged here: `git
    /// remote add` contacts nothing, so only a push can find it wrong.
    signal submitted(string name, string url)

    function start(name, url, takenNames) {
        remoteDialog.editing = name
        remoteDialog.taken = takenNames
        nameField.text = name === "" ? "origin" : name
        urlField.text = url
        remoteDialog.open()
    }
    // The URL is what there is to type in both cases — a new remote is
    // called `origin` until somebody says otherwise, and one being
    // corrected keeps the name it has.
    onOpened: urlField.forceActiveFocus()

    /// Automation: typing, which no injected key reaches offscreen. An
    /// empty half leaves what `start` put there.
    function setFields(name, url) {
        if (name !== "")
            nameField.text = name
        if (url !== "")
            urlField.text = url
    }

    function submit() {
        if (!saveButton.enabled)
            return
        const name = remoteDialog.editing !== "" ? remoteDialog.editing
                                                 : remoteDialog.wantedName
        const url = remoteDialog.wantedUrl
        remoteDialog.close()
        remoteDialog.submitted(name, url)
    }

    contentItem: ColumnLayout {
        spacing: Theme.spaceLg

        Label {
            text: remoteDialog.editing === ""
                  ? qsTr("Add a remote")
                  : qsTr("Where %1 is").arg(remoteDialog.editing)
            font.pixelSize: Theme.fontXl
            font.weight: Font.DemiBold
        }
        Label {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            color: Theme.textSecondary
            text: qsTr("Create the repository on your host first, then paste "
                       + "the URL it gives you. Nothing is contacted now — "
                       + "the first push is what tries it.")
        }

        // A remote being corrected keeps its name: renaming one belongs to
        // the left menu, and doing both here would make this two dialogs.
        ColumnLayout {
            Layout.fillWidth: true
            visible: remoteDialog.editing === ""
            spacing: Theme.spaceXs
            Label {
                text: qsTr("Name")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
            }
            FormField {
                id: nameField
                Layout.fillWidth: true
                placeholderText: "origin"
                onAccepted: remoteDialog.submit()
            }
            Label {
                visible: remoteDialog.nameClashes
                text: qsTr("This repository already has a remote called that")
                color: Theme.warning
                font.pixelSize: Theme.fontSm
            }
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            Label {
                text: qsTr("URL")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
            }
            FormField {
                id: urlField
                Layout.fillWidth: true
                placeholderText: "git@github.com:you/your-repo.git"
                onAccepted: remoteDialog.submit()
            }
        }

        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: Theme.spaceSm
            HoverButton {
                implicitHeight: Theme.controlHeight
                text: qsTr("Cancel")
                onClicked: remoteDialog.close()
            }
            HoverButton {
                id: saveButton
                implicitHeight: Theme.controlHeight
                highlighted: true
                text: remoteDialog.editing === "" ? qsTr("Add") : qsTr("Save")
                enabled: remoteDialog.wantedUrl !== ""
                         && (remoteDialog.editing !== ""
                             || (remoteDialog.wantedName !== ""
                                 && !remoteDialog.nameClashes))
                onClicked: remoteDialog.submit()
            }
        }
    }
}
