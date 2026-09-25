import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// What the upstream question asks back: which remote, and which branch on it this one is measured against (デザイン規約
// §ブランチが測られる相手を決める). Built from a `Component`, since it stands inside the question's bar.
ColumnLayout {
    id: upstreamForm

    /// The remotes this repository has — the whole set of answers, so this half only picks.
    required property var remotes
    /// The branches the chosen remote carries here, by the name half alone. Suggestions, not the set of answers.
    required property var branches
    /// Where the question opens: the remote and branch already configured, or the branch's own name where nothing is.
    required property string remote
    required property string branch

    signal remotePicked(int index)
    signal branchEdited(string name)
    /// Enter in the name box (デザイン規約 §立っている質問は 1 か所で聞く). Whether that answers, and what answering
    /// runs, is the bar's (`AskBar`).
    signal answered()

    /// For automation (no injected click opens either list offscreen) and `tst_askfields` (how the boxes share the
    /// row).
    property alias remotePick: remotePick
    property alias branchPick: upstreamBranchField

    spacing: Theme.spaceXs

    RowLayout {
        id: askRow
        Layout.fillWidth: true
        spacing: Theme.spaceXs
        /// Sized as `PublishForm`'s row (デザイン規約 §レイアウト初期値): the typed name asks first, the remote takes
        /// what is left, both down to `askFieldMinW`.
        readonly property real room: upstreamForm.width - slash.width - 2 * askRow.spacing
        readonly property real nameWidth:
            Math.min(Math.max(Metrics.askFieldMinW, upstreamBranchField.wantedWidth),
                     Math.max(Metrics.askFieldMinW, askRow.room - Metrics.askFieldMinW))
        readonly property real remoteCeiling: Math.max(Metrics.askFieldMinW, askRow.room - askRow.nameWidth)
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
        // `wanted:` is only where the question opens: the first key breaks the binding and the box owns the name
        // (`AppCombo.wanted`), so a remote picked later swaps the list and leaves the name standing.
        AppCombo {
            id: upstreamBranchField
            Layout.minimumWidth: Metrics.askFieldMinW
            Layout.maximumWidth: Math.max(Metrics.askFieldMinW, askRow.room - Metrics.askFieldMinW)
            Layout.preferredWidth: askRow.nameWidth
            Layout.fillWidth: true
            model: upstreamForm.branches
            wanted: upstreamForm.branch
            onWantedChanged: upstreamForm.branchEdited(upstreamBranchField.wanted)
            // Enter, with the box's own list not standing in front of it (`AppCombo.submitted`).
            onSubmitted: upstreamForm.answered()
            Component.onCompleted: upstreamBranchField.forceActiveFocus()
        }
    }
}
