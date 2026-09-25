import QtQuick
import QtQuick.Layouts
import platitude
import platitude.ui

// The identity form (user.name / user.email): the two boxes, their marks, the line over them and git's words on a
// failed write. One form for the gate and both settings chapters
// (rules-refs/app-ui.md「identity の欄は `IdentityFields` に 1 本」); what the pair is read from and written to is the
// caller's, and every default is the global pair (`AppBackend`).
ColumnLayout {
    id: fields

    property alias nameText: nameField.text
    property alias emailText: emailField.text
    /// Both boxes hold something. Not asked where empty is an answer (the repository chapter: "not set here").
    readonly property bool filled: nameField.text.trim() !== "" && emailField.text.trim() !== ""
    /// The boxes differ from what git holds, whoever put it there. Compared trimmed, as git is handed and reports it
    /// (`identity::set_identity` / `identity::read`) — a trailing space is no difference git would keep.
    readonly property bool differs: nameField.text.trim() !== fields.heldName
                                    || emailField.text.trim() !== fields.heldEmail
    /// Somebody typed and it differs from git: the settings screen's one unsaved edit (`SettingsDialog.escapeOut`) and
    /// what both Saves are lit by. `touched` is asked first — git answers late, answers again, and empties the boxes
    /// on the way to another repository, none of which is an edit (rules-refs/app-ui.md の `IdentityFields.dirty` の行).
    readonly property bool dirty: fields.touched && fields.differs

    /// What `load()` fills the boxes from: what git holds now, wherever the caller reads it.
    property string heldName: AppBackend.identityName
    property string heldEmail: AppBackend.identityEmail

    /// What an empty box stands for: an example for the global pair; a repository's override shows the inherited
    /// value.
    property string namePlaceholder: qsTr("Ada Lovelace")
    property string emailPlaceholder: qsTr("ada@example.com")

    /// The check beside one box, shown only when the save half-landed (both landing says nothing).
    property bool nameMarked: AppBackend.identityUnsaved && AppBackend.identityNameSaved
    property bool emailMarked: AppBackend.identityUnsaved && AppBackend.identityEmailSaved

    /// git's own words, unedited; a sentence of our own where a write did not take and git raised nothing.
    property string errorText: AppBackend.identityError !== ""
                               ? AppBackend.identityError
                               : AppBackend.identityUnsaved
                                 ? qsTr("git still reports a different identity. A setting in the repository this window was started in can sit over this one.")
                                 : ""

    /// The line over the boxes (デザイン規約 §設定の画面「説明はその章の前、今の値はその欄の後」). The default, for the
    /// gate, says how far the write reaches; the settings screen's warning says that, so there it only names the keys.
    property string note: qsTr("Saved for every repository on this computer (user.name and user.email).")

    /// Enter was pressed in one of the boxes.
    signal submitted()

    /// A person typed in one of the boxes — from `textEdited`, so `load()` filling them raises nothing (as
    /// `AppCombo.wanted`).
    signal edited()

    /// Somebody typed since the last `load()` (see `dirty`).
    property bool touched: false
    // Through `Connections`: an `onEdited` in this body would be replaced by the caller's (`SettingsRepoPane` writes
    // one).
    Connections {
        target: fields
        function onEdited() { fields.touched = true }
    }
    onHeldNameChanged: if (!fields.touched) fields.load()
    onHeldEmailChanged: if (!fields.touched) fields.load()

    /// Fills the boxes from git's answer: on open, and whenever it moves under untouched boxes.
    function load() {
        nameField.text = fields.heldName
        emailField.text = fields.heldEmail
        fields.touched = false
    }
    function focusName() {
        nameField.forceActiveFocus()
    }
    /// Automation: types into the name box; `edited()` by hand, since code-set text raises no `textEdited`.
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
                // The characters git drops from an author line; keeping them out keeps the config agreeing with
                // commits.
                validator: RegularExpressionValidator {
                    regularExpression: /[^<>\r\n]*/
                }
            }
            // `opacity`, not `visible`, so the field keeps its width.
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
    // The settings screen's one edit waiting for a Save, said beside that Save (デザイン規約 §可否・警告の出し場所); the
    // way out brings the reader here (`SettingsDialog.escapeOut`).
    CardText {
        Layout.fillWidth: true
        visible: fields.dirty
        color: Theme.warning
        text: qsTr("Not given to git yet.")
    }
    // A `CardText` so git's words can be taken away (デザイン規約 §右のペインの字は掴める).
    CardText {
        Layout.fillWidth: true
        visible: text !== ""
        color: Theme.danger
        text: fields.errorText
    }
}
