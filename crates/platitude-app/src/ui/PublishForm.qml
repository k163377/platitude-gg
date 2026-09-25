import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The first push's question: where the branch goes and what it is called there (デザイン規約 §はじめてリモートへ送る).
// Built from a `Component`: it stands inside the question's bar.
ColumnLayout {
    id: publishForm

    /// The remotes to choose from, with the add-remote row last.
    required property var choices
    /// The remote this repository pushes to (empty if none is marked); the list marks it.
    property string markedRemote: ""
    /// What is chosen now, and what the branch is to be called over there.
    required property string remote
    required property string branch

    signal remotePicked(int index)
    signal branchEdited(string name)
    /// Enter in the name box answers the question (デザイン規約 §立っている質問は 1 か所で聞く); whether it may is the
    /// bar's to weigh (`AskBar`).
    signal answered()

    /// For automation and `tst_askfields`: no injected click opens the list offscreen.
    property alias remotePick: remotePick
    property alias branchField: publishBranchField

    spacing: Theme.spaceXs

    RowLayout {
        id: askRow
        Layout.fillWidth: true
        spacing: Theme.spaceXs
        readonly property real room: publishForm.width - slash.width - 2 * askRow.spacing
        /// The name being typed claims first, up to all but the destination's floor
        /// (デザイン規約 §レイアウト初期値 の `askFieldMinW`).
        readonly property real nameWidth:
            Math.min(Math.max(Metrics.askFieldMinW, publishBranchField.wantedWidth),
                     Math.max(Metrics.askFieldMinW, askRow.room - Metrics.askFieldMinW))
        /// The destination takes what is left. **Both boxes need ceilings**: a layout grants a box its ask and lets
        /// the row overflow.
        readonly property real remoteCeiling: Math.max(Metrics.askFieldMinW, askRow.room - askRow.nameWidth)
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
        // Takes the slack; past its ceiling the text scrolls inside it.
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
