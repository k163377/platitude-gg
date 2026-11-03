import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The first push's question: where the branch goes and what it is called there (the flow: `PublishFlow.qml`).
// Built from a `Component`: it stands inside the question's bar, and the form is always one line — the destination
// and the name over there, nothing else. The destination is a dropdown, not chips: a fork-style repository has several
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
    /// The least width the pair takes: both boxes on their floor, the slash and the gaps. `AskBar` stands its pill
    /// under the form where this and the pill do not fit on one line.
    readonly property real leastWidth: 2 * Metrics.askFieldMinW + slash.width + 2 * askRow.spacing

    spacing: Theme.spaceXs

    // The `<destination>/<name>` pair (`UpstreamForm` shares the arithmetic). **The two boxes grow to the bar's whole
    // width** (user decision — ask before changing): the bar is as wide as the window, and cutting a name with half
    // the bar empty is a loss. Both stand on `askFieldMinW`, their floor, where the row holds the two (`remoteFloor`
    // says how a short destination comes back to it); past it the name scrolls inside its box. A row narrower than the
    // two floors (the graph at its floor) lets them give way, so nothing runs out past it.
    RowLayout {
        id: askRow
        readonly property real room: publishForm.width - slash.width - 2 * askRow.spacing
        /// Narrower than the two floors: the floors give way (user decision — ask before changing). The destination
        /// comes down to what its own name needs, never past half the row, and the name takes the rest. The arithmetic
        /// is `UpstreamForm`'s too.
        readonly property bool narrow: askRow.room < 2 * Metrics.askFieldMinW
        /// Past the two floors a short destination comes back up to its floor a pixel for each the row gains, the name
        /// keeping the width it had where the floors met, so neither box jumps as the row widens. Where the bar's pill
        /// moves up beside the words the row loses the pill's width, and the destination gives it, back down to its
        /// own name's width: the name being typed keeps its box (user decision — ask before changing).
        readonly property real remoteFloor:
            Math.min(Metrics.askFieldMinW,
                     Math.max(Math.min(remotePick.wantedWidth, askRow.room / 2),
                              askRow.room - 2 * Metrics.askFieldMinW
                              + Math.min(remotePick.wantedWidth, Metrics.askFieldMinW)))
        readonly property real nameFloor: askRow.narrow ? askRow.room - askRow.remoteFloor : Metrics.askFieldMinW
        /// The name being typed claims first (user decision), up to all but the destination's floor — so as the name
        /// grows, the destination comes down to its floor.
        readonly property real nameWidth:
            Math.min(Math.max(askRow.nameFloor, publishBranchField.wantedWidth),
                     Math.max(askRow.nameFloor, askRow.room - askRow.remoteFloor))
        /// The destination takes what is left, up to the width its own name needs (`AppCombo.wantedWidth`). **Both
        /// boxes need ceilings**: a layout grants a box its ask and lets the row overflow.
        readonly property real remoteCeiling: Math.max(askRow.remoteFloor, askRow.room - askRow.nameWidth)
        Layout.fillWidth: true
        spacing: Theme.spaceXs
        AppCombo {
            id: remotePick
            leadMark: "remote"
            pickOnly: true
            lastRowActs: true
            // The list spreads across the form to show each remote whole, whatever the box gave way to.
            spreadWithin: publishForm
            Layout.minimumWidth: askRow.remoteFloor
            Layout.maximumWidth: askRow.remoteCeiling
            Layout.preferredWidth: Math.max(askRow.remoteFloor, remotePick.wantedWidth)
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
            // As tall as the destination's box and the bar's pill, which shares the form's line (`AskBar.askColumns`).
            implicitHeight: Theme.controlHeight
            Layout.minimumWidth: askRow.nameFloor
            Layout.maximumWidth: Math.max(askRow.nameFloor, askRow.room - askRow.remoteFloor)
            Layout.preferredWidth: askRow.nameWidth
            Layout.fillWidth: true
            text: publishForm.branch
            onTextEdited: publishForm.branchEdited(text)
            onAccepted: publishForm.answered()
            Component.onCompleted: publishBranchField.forceActiveFocus()
        }
    }
}
