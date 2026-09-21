import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// What the upstream question asks back: which remote, and which branch on it this one is measured against (デザイン規約
// §ブランチが測られる相手を決める). It stands inside the question's bar, which is why it is built from a
// `Component`.
//
// **Both halves are answered**, which is the whole reason the row asks: nothing
// local says which of a remote's branches this one belongs with, and a wrong guess is a branch quietly measured
// against somebody else's work.
//
// **The name is typed, and the list is a shortcut past typing it.** What the remote already carries is a set of
// suggestions — a name outside it is a branch the next push makes, which is an answer this question takes
// (デザイン規約 §ブランチが測られる相手を決める).
ColumnLayout {
    id: upstreamForm

    /// The remotes this repository has. The whole set of answers — a branch cannot be measured against a remote that
    /// is not written down — so this half only ever picks.
    required property var remotes
    /// The branches the chosen remote carries here, by the name half alone. Suggestions, not the set of answers.
    required property var branches
    /// Where the question opens: the remote and branch already configured, or the branch's own name where nothing is.
    required property string remote
    required property string branch

    signal remotePicked(int index)
    signal branchEdited(string name)

    /// Automation only: neither list can be opened by an injected click on the offscreen platform, and how the two
    /// boxes share the row is a thing only a laid-out row can answer (`tst_askfields`).
    property alias remotePick: remotePick
    property alias branchPick: upstreamBranchField

    spacing: Theme.spaceXs

    RowLayout {
        id: askRow
        Layout.fillWidth: true
        spacing: Theme.spaceXs
        /// The same two claims the publish question's row settles (`PublishForm`): what is being typed asks first,
        /// and the destination takes what is left, down to the floor the two boxes share.
        readonly property real room: upstreamForm.width - slash.width - 2 * askRow.spacing
        readonly property real nameWidth:
            Math.min(Math.max(Metrics.askFieldMinW, upstreamBranchField.wantedWidth),
                     Math.max(Metrics.askFieldMinW, askRow.room - Metrics.askFieldMinW))
        readonly property real remoteCeiling: Math.max(Metrics.askFieldMinW, askRow.room - askRow.nameWidth)
        // The same two boxes the publish question asks in, sized the same way (デザイン規約 §レイアウト初期値).
        AppCombo {
            id: remotePick
            pickOnly: true
            Layout.minimumWidth: Metrics.askFieldMinW
            Layout.maximumWidth: askRow.remoteCeiling
            Layout.preferredWidth: Math.max(Metrics.askFieldMinW, remotePick.wantedWidth)
            model: upstreamForm.remotes
            wanted: upstreamForm.remote
            onActivated: index => upstreamForm.remotePicked(index)
        }
        Label {
            id: slash
            Layout.alignment: Qt.AlignVCenter
            text: "/"
            color: Theme.textMuted
            font.pixelSize: Theme.fontMd
        }
        // The question opens on a name: an empty box would read as though there were nothing
        // to point at.
        //
        // **`wanted` is the box's own once a key lands in it** — the binding below is where the question opens, and
        // the field takes the value over from there (`AppCombo.wanted`). A remote picked after that swaps the list
        // and leaves the name standing, which is what the two halves being one answer means.
        AppCombo {
            id: upstreamBranchField
            Layout.minimumWidth: Metrics.askFieldMinW
            Layout.maximumWidth: Math.max(Metrics.askFieldMinW, askRow.room - Metrics.askFieldMinW)
            Layout.preferredWidth: askRow.nameWidth
            Layout.fillWidth: true
            model: upstreamForm.branches
            wanted: upstreamForm.branch
            onWantedChanged: upstreamForm.branchEdited(upstreamBranchField.wanted)
            Component.onCompleted: upstreamBranchField.forceActiveFocus()
        }
    }
}
