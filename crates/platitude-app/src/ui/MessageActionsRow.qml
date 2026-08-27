import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The details editor's action row: the one button that writes the message back, and the lines that say what writing it
// costs. It stands from the moment a caret enters either box rather than from the moment the text differs — a button
// that appears under the hand as the first character lands moves everything below it at the worst moment, and the way
// out of the editor should be on screen for as long as the editor is open.
//
// **There is no Cancel.** Throwing away a draft has two ways out that the reader takes on purpose — Escape, and
// reading another commit — and neither of them asks (デザイン規約 §コミットメッセージの 2 つの枠). A button whose
// whole job is to undo typing would be the one place in the app where a single click throws text away.
ColumnLayout {
    id: actions

    /// The boxes differ from the commit's own message.
    property bool dirty: false
    /// A remote already has this commit.
    property bool published: false
    /// A write is already running, so nothing new starts.
    property bool busy: false
    /// The summary box holds something to save.
    property bool canSave: false
    /// A caret is in one of the boxes.
    property bool editing: false
    /// Who will be recorded as having made this commit once it is written back — **not** who wrote it. git keeps the
    /// author and replaces the committer with whoever runs the rewrite (measured: Alice's commit amended by Bob comes
    /// back `A=Alice C=Bob`), which is the one thing about this button a reader cannot see anywhere else on the pane —
    /// the row above names the author.
    property int committerFace: -1
    property string committerFaceUrl: ""
    /// Whether that commit will be signed, and with what (`SignatureMark` / `AvatarButton`).
    property string signature: ""
    property string signatureTip: ""

    signal saveRequested()

    visible: actions.dirty || actions.editing
    spacing: Theme.spaceXs

    // Said, not asked, like the amend editor's tag: the save still goes ahead, and this line is the warning it gets.
    Label {
        Layout.fillWidth: true
        visible: actions.published
        wrapMode: Text.Wrap
        text: qsTr("This commit is on a remote. Rewriting it leaves anyone who already has it out of step.")
        color: Theme.warning
        font.pixelSize: Theme.fontSm
    }
    // The same button the commit editor ends with, and for the same reason: it names the command it runs and whom the
    // result will be attributed to. One command only — the boxes open on HEAD's own commit and nothing else
    // (`offers::message_edit`), which is exactly what `commit --amend` reaches (デザイン規約 §git 用語のコード表記).
    ActionButton {
        id: saveButton
        Layout.fillWidth: true
        font.pixelSize: Theme.fontLg
        implicitHeight: Theme.toolbarHeight
        centred: true
        tone: Theme.textPrimary
        frameColor: saveButton.enabled ? Theme.accent : Theme.borderDefault
        activeFocusOnTab: true
        phraseHead: "commit --amend"
        text: qsTr("the message")
        phraseFace: actions.committerFace
        phraseFaceUrl: actions.committerFaceUrl
        phraseSignature: actions.signature
        phraseSignatureTip: actions.signatureTip
        // Live whenever there is a message to save. **Not `dirty`** — a button that greys out the moment the text
        // matches again answers "did I change anything" with its own state, which is a question nobody asked; pressing
        // it with nothing changed simply does nothing (`DetailsPane.submitMessage`).
        enabled: !actions.busy && actions.canSave
        onActivated: actions.saveRequested()
    }
}
