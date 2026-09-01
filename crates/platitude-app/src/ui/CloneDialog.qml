import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// Where a repository is fetched from a host: a URL, a folder to put it in, and the name that folder takes.
//
// A window of its own, like the two forms beside it (`RemoteDialog` / identity): there is no repository yet, so there
// is no list for a question bar to stand over and no tab for an answer to land in
// (デザイン規約 §可否・警告の出し場所 — popups are for what has nowhere else to go).
//
// **It stays up for the whole call.** git is on the network here, and this box is the only thing on screen that the
// operation is about: the ring turns in the accept button's own seat, the word stays and steps down
// (§進行中・長押しの定数), and whatever git answers is read out underneath. That is also why nothing is checked before
// the call — a destination that is taken and a URL nothing answers are git's to refuse, and unlike the picker's folder
// the refusal has somewhere to be shown when it arrives (§リポジトリを取り寄せる).
AppDialog {
    id: cloneDialog

    /// git is out fetching this clone (`TabsModel.cloning`).
    property bool cloning: false
    /// The folder the clone is made **in** — a `file:` URL, as the platform's dialog answers with one. Empty until
    /// something has named one, which is what leaves the accept button refusing.
    property string parentUrl: ""
    /// What git said about the last try. Cleared by the next one; a box that kept the old answer while a new call was
    /// out would be quoting git about something else.
    property string refusal: ""
    /// Set once somebody edits the name themselves, after which it stops following the URL.
    property bool nameTyped: false
    /// This box is going down because the clone came, rather than because anybody left. Read by `onClosed`, which
    /// otherwise takes every close for a cancel — and a cancel here would be a slot call back into the object whose
    /// `drain` emitted the landing, which is the re-entrant borrow the bridge panics on (.claude/rules/app-ui.md).
    property bool landing: false

    readonly property string wantedUrl: urlField.text.trim()
    readonly property string wantedName: nameField.text.trim()
    /// The folder as a person reads it, rather than as a URL.
    readonly property string parentPath: cloneDialog.parentUrl !== ""
                                         ? GitFacts.folderPath(cloneDialog.parentUrl) : ""
    /// Whether there is a clone to ask for: somewhere to fetch from, somewhere to put it, and a name for it. The
    /// accept button reads this, and so does the headless run — which needs the form's own answer to wait on rather
    /// than a second reading of the three fields (規約 §UI 自動化の因果性).
    readonly property bool canSubmit: cloneDialog.wantedUrl !== "" && cloneDialog.wantedName !== ""
                                      && cloneDialog.parentUrl !== "" && !cloneDialog.cloning

    /// Fetch it. The three the call takes, in the order the form asks them.
    signal submitted(string url, string parentUrl, string name)
    /// The box is going away — whether by the button, by Escape, or by anything else that closes a dialog. A clone
    /// still out is stopped by it: **one road out**, so no way of leaving can strand git on the network with nowhere
    /// left to report.
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
    /// The folder dialog answered. Its own function rather than a write from outside: the name follows the URL and
    /// nothing else, and a caller reaching into the properties is how the two got out of step in the first place.
    function setFolder(url) {
        cloneDialog.parentUrl = url
    }
    /// git's answer to the last try, which the box stays open to show.
    function said(message) {
        cloneDialog.refusal = message
    }

    /// The clone came down: the box goes without the press that takes it away meaning "stop".
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

    /// Automation: typing, which no injected key reaches offscreen. An empty half leaves what `start` put there, and
    /// the name follows the URL exactly as it does under a hand.
    function setFields(url, name) {
        if (url !== "")
            urlField.text = url
        if (name !== "") {
            nameField.text = name
            cloneDialog.nameTyped = true
        }
    }

    /// What the box offers to call the folder, for as long as nobody has said otherwise. The rule is
    /// `platitude_core::remote::folder_name_for` — asked of `GitFacts` rather than written here, like every other rule
    /// QML asks by value (.claude/rules/app-ui.md).
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
            // No article: this names the kind of thing being made, and there is nothing yet to point at (§長さ).
            text: qsTr("Clone repository")
            font.pixelSize: Theme.fontXl
            font.weight: Font.DemiBold
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

        // A folder that is chosen rather than typed, so it wears no sunken ground: what is shown is the whole of it and
        // there is nothing to type into (規約 §選ぶ欄と打つ欄).
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
                    // Cut in the middle: a path is told apart by both of its ends, and the leaf is the half that says
                    // which folder this is (`CutName`).
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
                    // The `…` says a box opens, the way it does on every other row that asks something (§長押し の語彙).
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
                // Only a hand stops the name from following the URL: `textEdited` is the one that is not emitted for a
                // write from here, so `settleName` cannot mark its own suggestion as somebody's choice.
                onTextEdited: cloneDialog.nameTyped = true
                onAccepted: cloneDialog.submit()
            }
        }

        // git's own words and the only red here, the way every other screen quotes them (規約 §リポジトリを開く:
        // 赤は git の文言だけ). Wrapped rather than cut: a refusal names the destination or the URL, and both are worth
        // reading whole.
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
            // The mark a fetch wears: this is the first one, and the seat it stands in is where the ring turns while
            // git is out (§進行中・長押しの定数 — 待てるボタンには席が要る).
            acceptKind: "fetch"
            acceptText: qsTr("Clone")
            busy: cloneDialog.cloning
            acceptEnabled: cloneDialog.canSubmit
            onCancelled: cloneDialog.close()
            onAccepted: cloneDialog.submit()
        }
    }
}
