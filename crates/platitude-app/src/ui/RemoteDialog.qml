import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Where a remote is written down: a name and a URL.
//
// A window of its own rather than the standing question's bar, because this is not a question about what is on screen —
// it is repository configuration, the same kind of thing as identity, which デザイン規約 §可否・警告の出し場所 keeps in a popup.
// Putting it in the bar also put a remote's own name and URL on a second row while the branch name stayed on the first,
// which grouped them backwards.
//
// The same form corrects a URL. Adding one and fixing one differ by which half is filled in already, not by what has to
// be typed.
AppDialog {
    id: remoteDialog

    /// The remote being corrected; empty means one is being made.
    property string editing: ""
    /// Names this repository already has. git refuses a duplicate itself (`remote <name> already exists`, exit 3), but
    /// that refusal would arrive after the dialog had closed, with nothing on screen left for it to be about.
    property var taken: []
    /// Whether this remote is the one pushes go to, and whether that is this repository's own to change. A mark set
    /// for every repository cannot be cleared from here — git has no local spelling for "not set" (measured), and the only
    /// move against it is marking another remote.
    property bool marked: false
    property bool markLocal: true

    readonly property string wantedName: nameField.text.trim()
    readonly property string wantedUrl: urlField.text.trim()
    readonly property bool nameClashes:
        remoteDialog.editing === "" && remoteDialog.wantedName !== ""
        && remoteDialog.taken.indexOf(remoteDialog.wantedName) >= 0

    /// The remote was written down. The URL is not judged here: `git remote add` contacts nothing, so only a push can
    /// find it wrong.
    signal submitted(string name, string url)
    /// The box was left in a different state than it opened in. Its own signal, not part of `submitted`: what it
    /// changes is one config key and not the remote, and a form that reported both would have the caller work out
    /// which of the two it was being told about.
    signal markChanged(string name, bool marked)

    function start(name, url, takenNames, marked, markLocal) {
        remoteDialog.editing = name
        remoteDialog.taken = takenNames
        remoteDialog.marked = marked === true
        remoteDialog.markLocal = markLocal !== false
        markBox.checked = remoteDialog.marked
        // `origin` is only offered while the repository has no remote at all: it is what a clone would have called its
        // first one, and nothing standing there to clash with. Once anything exists the next name is not ours to guess
        // — a prefill could only repeat a name that is taken or invent one. The word carries no standing of its own
        // either way: whatever treats `origin` specially goes by the name a remote actually has, so editing the prefill
        // away simply means no remote is called that.
        nameField.text = name !== "" ? name : takenNames.length === 0 ? "origin" : ""
        urlField.text = url
        remoteDialog.open()
    }
    // The URL is what there is to type once the name is settled — kept on a correction, prefilled on a first remote.
    // Only a name this dialog could not guess puts the caret on the name instead.
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
        // After the remote itself: on the add form the remote being marked does not exist until the line above has
        // run, and the two go through one write queue in the order they are asked for.
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
            // The one thing the fields cannot ask for: where the URL comes from. Not "make one first" — the definite
            // article already puts the repository over there, and nothing here could make it anyway. That the URL is
            // untouched until the push is left unsaid; the push says it, at the moment it can be acted on. The same
            // line has to fit a correction, where the repository plainly exists.
            text: qsTr("Paste the URL of the repository on your host.")
        }

        // A remote being corrected keeps its name: renaming one belongs to the left menu, and doing both here would
        // make this two dialogs.
        LabeledField {
            visible: remoteDialog.editing === ""
            caption: qsTr("Name")
            FormField {
                id: nameField
                Layout.fillWidth: true
                // Only while nothing is called that. The box is prefilled whenever the repository has no remote at all,
                // so this shows in exactly the case a name already exists — and a greyed suggestion the Add button
                // would refuse is worse than no suggestion. No second name is invented in its place: which one fits is
                // the person's to know.
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

        // Where pushes go. The word is the role and `origin` is what git calls it — the name of the remote actually
        // holding it is this form's own heading (デザイン規約 §リモートを書き留める). The same words as the row on the left menu's
        // menu: two ways into one operation say one sentence (§メニュー).
        //
        // Not offered on the first remote of a repository, which is where every push goes anyway — the line is about
        // taking the destination from somewhere else, and there is nowhere else yet.
        ColumnLayout {
            id: markRow
            Layout.fillWidth: true
            spacing: 0
            visible: markBox.offered
            AppCheckBox {
                id: markBox
                /// Whether this form is asking the question at all.
                readonly property bool offered: remoteDialog.editing !== "" || remoteDialog.taken.length > 0
                text: qsTr("Mark as default remote (origin)")
                enabled: remoteDialog.markLocal || !remoteDialog.marked
            }
            // What checking it costs, and what unchecking it gives back (デザイン規約 §長さ: 見出し 1 行 + 失うもの 1 行). The third
            // line is not about this box at all: it says the one move a repository has against a mark it cannot clear.
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
            cancelText: qsTr("Cancel")
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
