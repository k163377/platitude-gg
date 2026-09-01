import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// The name and the email address git puts on a commit, and everything that answers for a write of them: the two
// boxes, the mark beside each, the line saying where they land, and git's own words when one of them did not.
//
// Three screens ask for the same pair — the settings screen's git chapter, its repository chapter, and the gate that
// stands on startup when git has no identity to commit with — so the form is one file and the callers differ only in
// what surrounds it: a chapter with a Save of its own, or a question with `Not now` beside it.
//
// **What the pair is read out of and written back to is the caller's**, which is what lets the same two boxes stand
// for the person's own identity and for one repository's override of it. Everything below defaults to the global pair
// (`AppBackend`), so a caller that says nothing gets the form the first two have always had.
ColumnLayout {
    id: fields

    /// What is typed, for the caller that hands it to git.
    property alias nameText: nameField.text
    property alias emailText: emailField.text
    /// Both boxes hold something. What a Save is enabled by, wherever that button stands — except where empty is an
    /// answer in its own right (the repository chapter, where it means "not set here").
    readonly property bool filled: nameField.text.trim() !== "" && emailField.text.trim() !== ""
    /// **Somebody typed, and what they typed is not what git holds.** The one thing on the settings screen that can
    /// be left half-done — everything else there writes as it is finished with, so this is the only edit a reader
    /// can walk away from and lose (`SettingsDialog.escapeOut`).
    ///
    /// **Both halves are required, and `touched` is the half that matters.** A difference between a box and git's
    /// answer is not by itself somebody's edit: git answers late, answers again on its own, and empties these boxes
    /// on the way to answering about another repository — and a way out that stopped for any of that would be
    /// warning about work nobody did (observed). What the Save is lit by stays the plain difference, since a Save
    /// has to go out when the boxes and git disagree however they came to (`SettingsRepoPane.dirty`).
    readonly property bool dirty: fields.touched
                                  && (nameField.text !== fields.heldName || emailField.text !== fields.heldEmail)

    /// What `load()` fills the boxes from: what git holds now, wherever the caller reads it.
    property string heldName: AppBackend.identityName
    property string heldEmail: AppBackend.identityEmail

    /// What an empty box stands in for. The global pair has nothing to fall back to, so its boxes show an example;
    /// a repository's override falls back to the person's own value, and says so.
    property string namePlaceholder: qsTr("Ada Lovelace")
    property string emailPlaceholder: qsTr("ada@example.com")

    /// The mark beside one box. Only ever seen next to a field whose neighbour has none: a save where both landed
    /// leaves nothing for either to say, which is why both defaults are guarded by the same "did not all land".
    property bool nameMarked: AppBackend.identityUnsaved && AppBackend.identityNameSaved
    property bool emailMarked: AppBackend.identityUnsaved && AppBackend.identityEmailSaved

    /// git's own words, unedited — and where a write took nowhere without git raising anything, a sentence of the
    /// caller's own, because there is no message to pass through and the marks alone do not say why one is missing.
    property string errorText: AppBackend.identityError !== ""
                               ? AppBackend.identityError
                               : AppBackend.identityUnsaved
                                 ? qsTr("git still reports a different identity. A setting in the repository this window was started in can sit over this one.")
                                 : ""

    /// The line over the boxes. The default says how far the write reaches, for the gate, which has nothing above it
    /// that does; the settings screen's warning already says it, so there the line only names the two keys.
    ///
    /// **Over them, not under.** It is about the pair — both boxes and the Save beside them — and a sentence set
    /// after the second box reads as belonging to that box alone (observed). The rule it follows is the one the
    /// group and category sentences already do: **what a thing is goes before it, what a thing currently amounts to
    /// goes after** (`errorText` below, and the settings screen's effective-value lines).
    property string note: qsTr("Saved for every repository on this computer (user.name and user.email).")

    /// Enter was pressed in one of the boxes.
    signal submitted()

    /// A person typed in one of the boxes.
    ///
    /// **Not `nameText`/`emailText` changing.** `TextField.textEdited` is not emitted when the text is set in code, so
    /// this stays quiet while `load()` fills the boxes — which is the whole use for it: a caller that guards a reload
    /// on "has the reader touched this" would otherwise be told yes by its own reload (the same distinction
    /// `AppCombo.wanted` documents, and for the same reason).
    signal edited()

    /// Somebody has typed since the boxes were last filled from git. **What keeps a late answer from being read as
    /// an edit**: git may answer after the screen opened, and boxes still holding the value from before that answer
    /// differ from it without anybody having touched them — which `dirty` would otherwise call unsaved work and the
    /// way out would stop for (measured: the way-out verb wedged on exactly this).
    property bool touched: false
    // **Through `Connections`, not an `onEdited` here.** A handler written in a component's own body is replaced
    // outright by one a caller writes at the instantiation, and one caller does (`SettingsRepoPane`) — so the
    // component's own bookkeeping would quietly stop happening in exactly the chapter that has the most of it.
    Connections {
        target: fields
        function onEdited() { fields.touched = true }
    }
    onHeldNameChanged: if (!fields.touched) fields.load()
    onHeldEmailChanged: if (!fields.touched) fields.load()

    /// Fills the boxes from what git answers with now. Called when the screen around them opens, and again whenever
    /// git's answer moves under boxes nobody has touched.
    function load() {
        nameField.text = fields.heldName
        emailField.text = fields.heldEmail
        fields.touched = false
    }
    function focusName() {
        nameField.forceActiveFocus()
    }
    /// Automation: leaves the name box holding something git has not been given. Both halves of what a keystroke
    /// does, because only one of them happens on its own — setting `text` in code raises no `textEdited`, which is
    /// the whole point of `edited()` above, so a run that only assigned would leave the callers that guard on the
    /// reader's touch thinking nobody had touched anything.
    function autoTypeName(text) {
        nameField.text = text
        fields.edited()
    }

    Layout.fillWidth: true
    spacing: Theme.spaceLg

    HelpText {
        text: fields.note
    }
    LabeledField {
        caption: qsTr("Name")
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceSm
            FormField {
                id: nameField
                Layout.fillWidth: true
                placeholderText: fields.namePlaceholder
                onAccepted: fields.submitted()
                onTextEdited: fields.edited()
                // Not validation — these are the characters git drops when it builds an author line, and keeping
                // them out stops the configuration from disagreeing with what commits show. Everything else is the
                // user's business.
                validator: RegularExpressionValidator {
                    regularExpression: /[^<>\r\n]*/
                }
            }
            // The mark is the caller's to decide (`nameMarked`). `opacity` keeps the field the same width whether or
            // not it is showing.
            NavIcon {
                Layout.alignment: Qt.AlignVCenter
                kind: "check"
                tint: Theme.success
                opacity: fields.nameMarked ? 1 : 0
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
                placeholderText: fields.emailPlaceholder
                onAccepted: fields.submitted()
                onTextEdited: fields.edited()
                validator: RegularExpressionValidator {
                    regularExpression: /[^<>\r\n]*/
                }
            }
            NavIcon {
                Layout.alignment: Qt.AlignVCenter
                kind: "check"
                tint: Theme.success
                opacity: fields.emailMarked ? 1 : 0
            }
        }
    }
    // **The one thing on the settings screen that is not written as it is finished with**, said where it is true
    // rather than in a band at the foot of the screen (規約 §可否・警告の出し場所 — the warning goes where the
    // operation is). The Save that answers it is the next thing under this line, which is the whole reason the way
    // out puts the reader here rather than asking them a question somewhere else (`SettingsDialog.escapeOut`).
    Label {
        Layout.fillWidth: true
        visible: fields.dirty
        wrapMode: Text.Wrap
        color: Theme.warning
        text: qsTr("Not given to git yet.")
    }
    // What went wrong, in the caller's words or git's (`errorText`).
    Label {
        Layout.fillWidth: true
        visible: text !== ""
        wrapMode: Text.Wrap
        color: Theme.danger
        text: fields.errorText
    }
}
