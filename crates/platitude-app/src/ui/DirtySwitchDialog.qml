import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Uncommitted changes come along by default, and this only appears when
// git refuses to carry them: the file changed on both sides, or an
// untracked file stands where the target keeps a tracked one. Two ways
// past it — leave them behind, or take them across merged. Both go
// through a stash, and differ only in whether it is restored on the
// other side (デザイン規約 §未コミット変更がある状態での移動).
AppDialog {
    id: dirtySwitchDialog

    // Where the move goes ("main", a short sha, ...).
    property string moveLabel: ""
    // Where the changes are now ("this commit" when detached).
    property string stayLabel: ""
    // What stands in the way: "changes" (tracked files that differ on both
    // sides) or "untracked" (files git never recorded, which nothing but
    // stashing clears out of the way).
    property string blockKind: "changes"
    readonly property bool mergeable: dirtySwitchDialog.blockKind === "changes"
    /// carry: "stash" = leave them here, "merge" = bring them merged.
    signal resolved(string carry)

    contentItem: ColumnLayout {
        spacing: Theme.spaceLg
        Label {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            text: qsTr("Your changes cannot come along as they are")
            font.pixelSize: Theme.fontLg
            font.weight: Font.DemiBold
        }
        Label {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            color: Theme.textSecondary
            text: dirtySwitchDialog.mergeable
                  ? qsTr("Some of the files you changed look different on %1, "
                         + "so your version cannot simply travel with you.")
                    .arg(dirtySwitchDialog.moveLabel)
                  : qsTr("%1 keeps files of its own where you have new ones "
                         + "here, and git will not write over a file it never "
                         + "recorded.").arg(dirtySwitchDialog.moveLabel)
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceSm
            HoverButton {
                Layout.fillWidth: true
                implicitHeight: Theme.controlHeight
                highlighted: true
                text: qsTr("Leave my changes on %1").arg(dirtySwitchDialog.stayLabel)
                onClicked: {
                    dirtySwitchDialog.close()
                    dirtySwitchDialog.resolved("stash")
                }
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                color: Theme.textMuted
                font.pixelSize: Theme.fontSm
                text: qsTr("Stashes them first; they wait in STASHES until "
                           + "you apply them again.")
            }
            HoverButton {
                Layout.fillWidth: true
                implicitHeight: Theme.controlHeight
                visible: dirtySwitchDialog.mergeable
                text: qsTr("Bring my changes to %1").arg(dirtySwitchDialog.moveLabel)
                onClicked: {
                    dirtySwitchDialog.close()
                    dirtySwitchDialog.resolved("merge")
                }
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                visible: dirtySwitchDialog.mergeable
                color: Theme.textMuted
                font.pixelSize: Theme.fontSm
                text: qsTr("Merges them into the files there, staged and "
                           + "unstaged as they are now. Whatever git cannot "
                           + "combine on its own is left marked up for you to "
                           + "settle, and waits in STASHES as well.")
            }
        }
        HoverButton {
            Layout.alignment: Qt.AlignRight
            implicitHeight: Theme.controlHeight
            text: qsTr("Cancel")
            onClicked: dirtySwitchDialog.close()
        }
    }
}
