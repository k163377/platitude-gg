import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The first push's question: where the branch goes and what it is called there (the flow: `PublishFlow.qml`).
// Built from a `Component`: it stands inside the question's bar, and the bar is always one line — the destination and
// the name over there, nothing else. The destination is a dropdown, not chips: a fork-style repository has several
// remotes, and chips grow sideways one per remote and push the name out of the frame. Adding a remote is the dialog
// (`RemoteDialog`), not fields here: name + URL in the bar would drop the remote's own name and URL to a second line
// and leave the branch name beside the destination — the grouping backwards.
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

    // The `<destination>/<name>` pair (`UpstreamForm` shares the arithmetic). **The two boxes grow to the bar's whole
    // width** (user decision — ask before changing): the bar is as wide as the window, and cutting a name with half
    // the bar empty is a loss. Both stand on `askFieldMinW`, their floor; past it the name scrolls inside its box.
    RowLayout {
        id: askRow
        Layout.fillWidth: true
        spacing: Theme.spaceXs
        readonly property real room: publishForm.width - slash.width - 2 * askRow.spacing
        /// The name being typed claims first (user decision), up to all but the destination's floor — so as the name
        /// grows, the destination comes down to `askFieldMinW`.
        readonly property real nameWidth:
            Math.min(Math.max(Metrics.askFieldMinW, publishBranchField.wantedWidth),
                     Math.max(Metrics.askFieldMinW, askRow.room - Metrics.askFieldMinW))
        /// The destination takes what is left, up to the width its own name needs (`AppCombo.wantedWidth`). **Both
        /// boxes need ceilings**: a layout grants a box its ask and lets the row overflow.
        readonly property real remoteCeiling: Math.max(Metrics.askFieldMinW, askRow.room - askRow.nameWidth)
        AppCombo {
            id: remotePick
            leadMark: "remote"
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
            leadMark: "branch"
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
