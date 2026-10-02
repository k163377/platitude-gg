import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// Where a repository is cloned from: a URL, a folder to put it in, and the folder's name. A dialog, since there is no
// repository yet for a question bar to stand over; it stays up for the whole call, and git's refusal is read out
// underneath (デザイン規約 §リポジトリを取り寄せる).
AppDialog {
    id: cloneDialog

    /// git is out fetching this clone (`TabsModel.cloning`).
    property bool cloning: false
    /// The folder the clone is made in, as a `file:` URL (the platform dialog's answer). Empty refuses the accept.
    property string parentUrl: ""
    /// What git said about the last try; cleared by the next.
    property string refusal: ""
    /// Set once somebody edits the name themselves, after which it stops following the URL.
    property bool nameTyped: false
    /// Closing because the clone came. `onClosed` otherwise takes every close for a cancel — here a slot call back
    /// into the object whose `drain` emitted the landing: the re-entrant borrow the bridge panics on
    /// (.claude/rules/app-ui.md).
    property bool landing: false

    readonly property string wantedUrl: urlField.text.trim()
    readonly property string wantedName: nameField.text.trim()
    /// The folder as a person reads it.
    readonly property string parentPath: cloneDialog.parentUrl !== ""
                                         ? GitFacts.pickedPath(cloneDialog.parentUrl) : ""
    /// Whether there is a clone to ask for. The accept button reads this, and so does the headless run.
    readonly property bool canSubmit: cloneDialog.wantedUrl !== "" && cloneDialog.wantedName !== ""
                                      && cloneDialog.parentUrl !== "" && !cloneDialog.cloning

    signal submitted(string url, string parentUrl, string name)
    /// The box is going away by any road (button, Escape, …); a clone still out is stopped by it.
    signal cancelled()
    /// Bring the platform's folder dialog up, starting at whatever is chosen now.
    signal chooseFolder(string near)

    function start(nearUrl) {
        cloneDialog.parentUrl = nearUrl
        cloneDialog.refusal = ""
        cloneDialog.nameTyped = false
        urlField.text = ""
        nameField.text = ""
        cloneDialog.open()
    }
    /// The folder dialog answered. Only the folder moves: the name follows the URL, not this.
    function setFolder(url) {
        cloneDialog.parentUrl = url
    }
    /// git's answer to the last try, which the box stays open to show.
    function said(message) {
        cloneDialog.refusal = message
    }

    /// The clone came down: close without it counting as a cancel.
    function landed() {
        cloneDialog.landing = true
        cloneDialog.close()
    }

    // The URL is the one thing here nobody can guess.
    onOpened: urlField.forceActiveFocus()
    onClosed: {
        if (!cloneDialog.landing)
            cloneDialog.cancelled()
        cloneDialog.landing = false
    }

    /// Automation: typing, which no injected key reaches offscreen. An empty half is left as `start` put it; the name
    /// follows the URL as under a hand.
    function setFields(url, name) {
        if (url !== "")
            urlField.text = url
        if (name !== "") {
            nameField.text = name
            cloneDialog.nameTyped = true
        }
    }

    /// Offers a folder name until somebody types one (`platitude_core::remote::folder_name_for`).
    function settleName() {
        if (!cloneDialog.nameTyped)
            nameField.text = GitFacts.cloneFolderName(urlField.text)
    }

    function submit() {
        if (!cloneDialog.canSubmit)
            return
        cloneDialog.refusal = ""
        cloneDialog.submitted(cloneDialog.wantedUrl, cloneDialog.parentUrl, cloneDialog.wantedName)
    }

    contentItem: ColumnLayout {
        spacing: Theme.spaceLg

        Label {
            // No article: it names a kind (§長さ).
            text: qsTr("Clone repository")
            font.pixelSize: Theme.fontXl
            font.weight: Theme.fontWeightStrong
        }

        LabeledField {
            caption: qsTr("URL")
            FormField {
                id: urlField
                Layout.fillWidth: true
                enabled: !cloneDialog.cloning
                placeholderText: qsTr("git@github.com:you/your-repo.git")
                onTextChanged: cloneDialog.settleName()
                onAccepted: cloneDialog.submit()
            }
        }

        // Chosen, not typed, so no sunken ground (規約 §選ぶ欄と打つ欄).
        LabeledField {
            caption: qsTr("Where to put it")
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceSm
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: Theme.controlHeight
                    color: "transparent"
                    radius: Theme.radiusSm
                    border.color: Theme.borderDefault
                    border.width: Theme.borderWidth
                    // Cut in the middle: a path is told apart by both of its ends.
                    CutName {
                        anchors.fill: parent
                        anchors.leftMargin: Theme.spaceXs
                        anchors.rightMargin: Theme.spaceXs
                        text: cloneDialog.parentPath
                        color: cloneDialog.cloning ? Theme.textMuted : Theme.textPrimary
                    }
                }
                ActionButton {
                    implicitHeight: Theme.controlHeight
                    activeFocusOnTab: true
                    enabled: !cloneDialog.cloning
                    kind: "folder"
                    besideWord: true
                    frameColor: Theme.borderDefault
                    // The `…` says a box opens (§長押し の語彙).
                    text: qsTr("Choose…")
                    onActivated: cloneDialog.chooseFolder(cloneDialog.parentUrl)
                }
            }
        }

        LabeledField {
            caption: qsTr("Folder name")
            FormField {
                id: nameField
                Layout.fillWidth: true
                enabled: !cloneDialog.cloning
                // `textEdited`, not `textChanged`: `settleName`'s own write must not count as somebody's choice.
                onTextEdited: cloneDialog.nameTyped = true
                onAccepted: cloneDialog.submit()
            }
        }

        // git's own words, the only red here (規約 §リポジトリを開く「赤は git の文言だけ」). Wrapped: a refusal names
        // the destination or the URL, worth reading whole.
        Label {
            Layout.fillWidth: true
            visible: cloneDialog.refusal !== ""
            wrapMode: Text.Wrap
            color: Theme.danger
            font.pixelSize: Theme.fontSm
            text: cloneDialog.refusal
        }

        DialogActions {
            cancelText: Words.cancel
            // The mark's seat is where the ring turns while git is out (§進行中・長押しの定数「待てるボタンには席が要る」).
            acceptKind: "fetch"
            acceptText: qsTr("Clone")
            busy: cloneDialog.cloning
            acceptEnabled: cloneDialog.canSubmit
            onCancelled: cloneDialog.close()
            onAccepted: cloneDialog.submit()
        }
    }
}
