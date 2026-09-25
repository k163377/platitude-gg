import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The details editor's action row: the button that writes the message back, and the warning of what that costs. It
// stands from the moment a caret enters either box — appearing as the first character lands would move everything
// below under the hand. No discard button on purpose: a draft is thrown away by Escape or by reading another commit
// (デザイン規約 §コミットメッセージの 2 つの枠), and a one-click discard would be the only one in the app.
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
    /// The boxes are feeding a rebase plan's `reword` row: the press stores the words in the
    /// plan and runs nothing (`RebasePlanModel.setMessage`).
    property bool intoPlan: false
    /// The committer once this is written back: git keeps the author and records whoever runs the rewrite, which
    /// nothing else on the pane shows.
    property int committerFace: -1
    property string committerFaceUrl: ""
    /// Whether that commit will be signed, and with what (`SignatureMark` / `AvatarButton`).
    property string signature: ""
    property string signatureTip: ""

    signal saveRequested()

    visible: actions.dirty || actions.editing
    spacing: Theme.spaceXs

    // A warning only; the save still goes ahead.
    Label {
        Layout.fillWidth: true
        visible: actions.published
        wrapMode: Text.Wrap
        text: qsTr("This commit is on a remote. Rewriting it leaves anyone who already has it out of step.")
        color: Theme.warning
        font.pixelSize: Theme.fontSm
    }
    // Outside a plan, always `commit --amend`: the boxes open only on HEAD's commit (`offers::message_edit`).
    ActionButton {
        id: saveButton
        Layout.fillWidth: true
        font.pixelSize: Theme.fontLg
        implicitHeight: Theme.toolbarHeight
        centred: true
        tone: Theme.textPrimary
        frameColor: saveButton.enabled ? Theme.accent : Theme.borderDefault
        activeFocusOnTab: true
        // Feeding a plan's `reword` row, the chip is that row's verb and nothing runs on this press
        // (デザイン規約 §git 用語のコード表記「interactive rebase の todo 動詞は、コマンドと同じに扱う」).
        phraseHead: actions.intoPlan ? "reword" : "commit --amend"
        text: actions.intoPlan ? qsTr("in the plan") : qsTr("the message")
        phraseFace: actions.committerFace
        phraseFaceUrl: actions.committerFaceUrl
        phraseSignature: actions.signature
        phraseSignatureTip: actions.signatureTip
        // Not on `dirty`: it would grey out whenever the text matches again. Pressed with nothing changed, it does
        // nothing (`DetailsPane.submitMessage`).
        enabled: !actions.busy && actions.canSave
        onActivated: actions.saveRequested()
    }
}
