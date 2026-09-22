import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// What the first push's question asks back: where the branch goes, and what it is called when it gets there (デザイン規約
// §はじめてリモートへ送る). It stands inside the question's bar, which is why it is built from a
// `Component`.
ColumnLayout {
    id: publishForm

    /// The remotes to choose from, with the add-remote row last.
    required property var choices
    /// The one this repository sends pushes to, empty where none is marked — where the question opens, and what the
    /// list says about it.
    property string markedRemote: ""
    /// What is chosen now, and what the branch is to be called over there.
    required property string remote
    required property string branch

    signal remotePicked(int index)
    signal branchEdited(string name)
    /// The name is finished: the question is answered from the box the keyboard is already in (デザイン規約
    /// §立っている質問は 1 か所で聞く). Nothing stands in front of this box — it has no list of its own, so the key
    /// is never somebody else's — and what answering costs is the bar's to weigh (`AskBar`): a first push the far
    /// side has not answered for yet is unanswerable, and one that turns out to replace their work is held.
    signal answered()

    /// Automation only: the list cannot be opened by an injected click on the offscreen platform, and how the two
    /// boxes share the row is a thing only a laid-out row can answer (`tst_askfields`).
    property alias remotePick: remotePick
    property alias branchField: publishBranchField

    spacing: Theme.spaceXs

    RowLayout {
        id: askRow
        Layout.fillWidth: true
        spacing: Theme.spaceXs
        /// What the two boxes have to share: the bar less the `/` and the gaps beside it.
        readonly property real room: publishForm.width - slash.width - 2 * askRow.spacing
        /// **What is being typed has the first claim on it** (デザイン規約 §レイアウト初期値): it is the half a reader
        /// is writing and reading back at the caret, so the room it needs comes off the destination — up to
        /// everything but the destination's floor, and at least its own.
        readonly property real nameWidth:
            Math.min(Math.max(Metrics.askFieldMinW, publishBranchField.wantedWidth),
                     Math.max(Metrics.askFieldMinW, askRow.room - Metrics.askFieldMinW))
        /// And the destination takes what is left. **Ceilings on both**, because a layout hands a
        /// box the width it asked for and lets the row overflow —
        /// so without them one long box pushed the other off the end of the bar (observed).
        readonly property real remoteCeiling: Math.max(Metrics.askFieldMinW, askRow.room - askRow.nameWidth)
        // **The destination takes the room its name needs out of what the name left** (デザイン規約 §レイアウト初期値):
        // the bar is as wide as the window, so a name cut while half of it stands empty is a name cut for nothing —
        // but the box being typed into asks first, and this one comes down to the floor the two share to let it.
        AppCombo {
            id: remotePick
            pickOnly: true
            lastRowActs: true
            Layout.minimumWidth: Metrics.askFieldMinW
            Layout.maximumWidth: askRow.remoteCeiling
            Layout.preferredWidth: Math.max(Metrics.askFieldMinW, remotePick.wantedWidth)
            model: publishForm.choices
            markedRow: publishForm.markedRemote
            wanted: publishForm.remote
            onActivated: index => publishForm.remotePicked(index)
        }
        Label {
            id: slash
            Layout.alignment: Qt.AlignVCenter
            text: "/"
            color: Theme.textMuted
            font.pixelSize: Theme.fontMd
        }
        // The box holds the name: empty, it would read as though there were nothing to send.
        //
        // **This is the box that asks first, and the box the slack goes to** — the one a caret is in. What it needs
        // is taken off the destination down to the floor they share (`remoteCeiling`); past that the text scrolls
        // inside it, the way any field being typed into does.
        SlimField {
            id: publishBranchField
            Layout.minimumWidth: Metrics.askFieldMinW
            Layout.maximumWidth: Math.max(Metrics.askFieldMinW, askRow.room - Metrics.askFieldMinW)
            Layout.preferredWidth: askRow.nameWidth
            Layout.fillWidth: true
            text: publishForm.branch
            onTextEdited: publishForm.branchEdited(text)
            onAccepted: publishForm.answered()
            Component.onCompleted: publishBranchField.forceActiveFocus()
        }
    }
}
