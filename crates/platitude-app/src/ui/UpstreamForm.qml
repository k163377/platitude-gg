import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// What the upstream question asks back: which remote, and which branch on it this one is measured against (デザイン規約
// §ブランチが測られる相手を決める). It stands inside the question's bar, which is why it is built from a `Component` rather than
// placed anywhere itself.
//
// **Both halves are answered rather than guessed**, which is the whole reason the row asks instead of acting: nothing
// local says which of a remote's branches this one belongs with, and a wrong guess is a branch quietly measured
// against somebody else's work.
//
// **The name is typed, never picked from a list**. A list of what the remote already carries
// would put picked names and typed ones in one box, and the two are not the same answer — a picked row is whatever
// happened to be fetched, which is not what the reader came to say.
ColumnLayout {
    id: upstreamForm

    /// The remotes this repository has. The whole set of answers — a branch cannot be measured against a remote that
    /// is not written down — so this half only ever picks.
    required property var remotes
    /// Where the question opens: the remote and branch already configured, or the branch's own name where nothing is.
    required property string remote
    required property string branch
    /// Nothing here answers to that name. The frame says so where the typing is and the bar's line says why
    /// (デザイン規約 §可否・警告の出し場所).
    required property bool refused

    signal remotePicked(int index)
    signal branchEdited(string name)

    spacing: Theme.spaceXs

    RowLayout {
        Layout.fillWidth: true
        spacing: Theme.spaceXs
        AppCombo {
            id: remotePick
            pickOnly: true
            // The fixed-input width every boxed field shares (デザイン規約 §レイアウト初期値 160) — not a width of its own.
            Layout.preferredWidth: 160
            model: upstreamForm.remotes
            wanted: upstreamForm.remote
            onActivated: index => upstreamForm.remotePicked(index)
        }
        Label {
            Layout.alignment: Qt.AlignVCenter
            text: "/"
            color: Theme.textMuted
            font.pixelSize: Theme.fontMd
        }
        // Never a placeholder: the question opens on a name, and an empty box would read as though there were nothing
        // to point at.
        SlimField {
            id: upstreamBranchField
            Layout.preferredWidth: 160
            text: upstreamForm.branch
            refused: upstreamForm.refused
            onTextEdited: upstreamForm.branchEdited(text)
            Component.onCompleted: upstreamBranchField.forceActiveFocus()
        }
        Item {
            Layout.fillWidth: true
        }
    }
}
