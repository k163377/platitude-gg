pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// One bucket heading in the working-tree file list: what the bucket is called, how many files are in it, and the
// affordance that moves the whole bucket at once.
//
// The list's section delegate builds one per bucket that has rows. The staged one is also built by the list's footer
// when nothing is staged (デザイン規約 §その他の操作) — a section with no rows has no heading of its own, and without this the pane
// never says where a staged file goes.
Rectangle {
    id: bucketHeader

    /// Which bucket this heads: `conflicts` / `unstaged` / `staged`. Named `section` because that is what the list
    /// injects.
    required property string section
    required property real listWidth
    required property var repoTab
    required property var workTree
    /// Files in this bucket. Untracked ones are shown as unstaged (`NavItem.group`), so they are counted there.
    readonly property int count:
        bucketHeader.section === "staged"
        ? bucketHeader.workTree.stagedCount
        : bucketHeader.section === "unstaged"
        ? bucketHeader.workTree.unstagedCount + bucketHeader.workTree.untrackedCount
        : bucketHeader.workTree.conflictCount
    /// An external merge tool holds the write queue until it is closed, which is the longest wait in the app and the
    /// only one with no upper bound.
    readonly property bool waitingForTool: bucketHeader.section === "conflicts"
        && bucketHeader.repoTab.busyOp === "mergetool"

    /// Automation: the whole-bucket button pressed where a hand presses it. Answers whether it went — a heading whose
    /// button is asleep (an empty bucket) has nothing to move, and a run that counted that as a press would wait for a
    /// write that was never asked for.
    function moveAll() {
        if (!moveAllButton.visible || !moveAllButton.enabled)
            return false
        moveAllButton.clicked()
        return true
    }

    width: bucketHeader.listWidth
    // A heading the list is not showing takes no room: the footer keeps its instance alive so the bindings stay live,
    // and a hidden item with a height would leave a band of ground behind the last row.
    height: bucketHeader.visible ? Theme.rowHeight : 0
    color: Theme.bgElevated
    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceSm
        anchors.rightMargin: Theme.spaceXs
        spacing: Theme.spaceXs
        Label {
            text: bucketHeader.section === "staged"
                  ? qsTr("STAGED FILES (%1)").arg(bucketHeader.count)
                  : bucketHeader.section === "unstaged"
                  ? qsTr("UNSTAGED FILES (%1)").arg(bucketHeader.count)
                  : qsTr("CONFLICTS")
            font.pixelSize: Theme.fontSm
            font.weight: Font.DemiBold
            color: bucketHeader.section === "conflicts" ? Theme.danger : Theme.textSecondary
        }
        Item { Layout.fillWidth: true }
        // The seat `Stage all` takes on the other two buckets. The words stay — nothing else in view names the tool
        // being waited on — and the ring says it is still running. No `…`: that is the word for a question standing,
        // and progress is the ring's job (規約 §進行中・長押しの定数).
        //
        // Nothing to press: killing `git mergetool` would leave the editor it started running and its scratch behind.
        Label {
            visible: bucketHeader.waitingForTool
            text: qsTr("Waiting for %1").arg(bucketHeader.workTree.mergeTool)
            font.pixelSize: Theme.fontSm
            color: Theme.textSecondary
        }
        SpinnerIcon {
            spinning: bucketHeader.waitingForTool
            Layout.preferredWidth: Theme.iconSm
            Layout.preferredHeight: Theme.iconSm
        }
        HoverToolButton {
            id: moveAllButton
            visible: bucketHeader.section !== "conflicts"
            // An empty bucket has nothing to move, and this heading stands even then (§無効 — what cannot be pressed says
            // so where it stands, rather than leaving its seat).
            enabled: bucketHeader.count > 0
            text: bucketHeader.section === "staged" ? qsTr("Unstage all") : qsTr("Stage all")
            font.pixelSize: Theme.fontSm
            tip: bucketHeader.section === "staged"
                 ? qsTr("Unstage everything") : qsTr("Stage everything, untracked included")
            onClicked: {
                if (bucketHeader.section === "staged")
                    bucketHeader.repoTab.unstageAll()
                else
                    bucketHeader.repoTab.stageAll()
            }
        }
    }
}
