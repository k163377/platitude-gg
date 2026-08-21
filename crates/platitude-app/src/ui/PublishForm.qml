import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// What the first push's question asks back: where the branch goes, and what it is called when it gets there (デザイン規約
// §はじめてリモートへ送る). It stands inside the question's bar, which is why it is built from a `Component` rather than placed
// anywhere itself.
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

    /// Automation only: the list cannot be opened by an injected click on the offscreen platform.
    property alias remotePick: remotePick

    spacing: Theme.spaceXs

    RowLayout {
        Layout.fillWidth: true
        spacing: Theme.spaceXs
        AppCombo {
            id: remotePick
            pickOnly: true
            lastRowActs: true
            // The fixed-input width every boxed field shares (デザイン規約 §レイアウト初期値 160) — not a width of its own.
            Layout.preferredWidth: 160
            model: publishForm.choices
            markedRow: publishForm.markedRemote
            wanted: publishForm.remote
            onActivated: index => publishForm.remotePicked(index)
        }
        Label {
            Layout.alignment: Qt.AlignVCenter
            text: "/"
            color: Theme.textMuted
            font.pixelSize: Theme.fontMd
        }
        // Never a placeholder: an empty box would read as though there were nothing to send.
        SlimField {
            id: publishBranchField
            Layout.preferredWidth: 160
            text: publishForm.branch
            onTextEdited: publishForm.branchEdited(text)
            Component.onCompleted: publishBranchField.forceActiveFocus()
        }
        Item {
            Layout.fillWidth: true
        }
    }
}
