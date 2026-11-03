import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// What the upstream question asks back: which remote, and which branch on it this one is measured against (the flow:
// `UpstreamFlow.qml`). Built from a `Component`, since it stands inside the question's bar.
//
// The remote is only picked; the branch name is typed, with a dropdown (user decision — ask before changing): its list
// is the branches the picked remote holds here, a shortcut for typing and nothing more, and the two boxes wear
// different grounds (デザイン規約 §選ぶ欄と打つ欄). Unlike the publish chooser, the remotes are the configured ones only,
// with no `Add remote…` row. A remote just written down holds no tracking ref yet, so the name's list has nothing to
// open (its mark drops) and typing answers.
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
    /// The least width the pair takes, as `PublishForm.leastWidth`.
    readonly property real leastWidth: 2 * Metrics.askFieldMinW + slash.width + 2 * askRow.spacing

    spacing: Theme.spaceXs

    RowLayout {
        id: askRow
        /// Sized as `PublishForm`'s row (`askRow` there says why): the typed name asks first, the remote takes what is
        /// left, both down to `askFieldMinW` — and on a row narrower than the two floors, the remote down to what its
        /// name needs (never past half the row) and the name to the rest; past the floors a short remote comes back up
        /// to its floor a pixel at a time.
        readonly property real room: upstreamForm.width - slash.width - 2 * askRow.spacing
        readonly property bool narrow: askRow.room < 2 * Metrics.askFieldMinW
        readonly property real remoteFloor:
            Math.min(Metrics.askFieldMinW,
                     Math.max(Math.min(remotePick.wantedWidth, askRow.room / 2),
                              askRow.room - 2 * Metrics.askFieldMinW
                              + Math.min(remotePick.wantedWidth, Metrics.askFieldMinW)))
        readonly property real nameFloor: askRow.narrow ? askRow.room - askRow.remoteFloor : Metrics.askFieldMinW
        readonly property real nameWidth:
            Math.min(Math.max(askRow.nameFloor, upstreamBranchField.wantedWidth),
                     Math.max(askRow.nameFloor, askRow.room - askRow.remoteFloor))
        readonly property real remoteCeiling: Math.max(askRow.remoteFloor, askRow.room - askRow.nameWidth)
        Layout.fillWidth: true
        spacing: Theme.spaceXs
        AppCombo {
            id: remotePick
            leadMark: "remote"
            pickOnly: true
            // Both lists spread across the form to show each name whole, whatever the boxes gave way to.
            spreadWithin: upstreamForm
            Layout.minimumWidth: askRow.remoteFloor
            Layout.maximumWidth: askRow.remoteCeiling
            Layout.preferredWidth: Math.max(askRow.remoteFloor, remotePick.wantedWidth)
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
            leadMark: "branch"
            spreadWithin: upstreamForm
            Layout.minimumWidth: askRow.nameFloor
            Layout.maximumWidth: Math.max(askRow.nameFloor, askRow.room - askRow.remoteFloor)
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
