pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// One bucket's heading in the worktree file list: its name, its count, and the button that moves the whole
// bucket. Pinned above its rows (`WipBucketPane`).
Rectangle {
    id: bucketHeader

    /// Which bucket this heads: `conflicts` / `unstaged` / `staged`.
    required property string section
    required property var repoTab
    required property var worktree
    /// This bucket's rows.
    required property var bucketModel
    /// Files in this bucket as the list counts them (`NavSectionModel.runFiles`) — untracked ride in unstaged, a rule
    /// kept once on the Rust side (`Bucket::run`).
    readonly property int count: bucketHeader.bucketModel.runFiles
    /// An external merge tool holds the write queue until it is closed.
    readonly property bool waitingForTool: bucketHeader.section === "conflicts"
        && bucketHeader.repoTab.busyOp === "mergetool"

    /// Automation: presses the whole-bucket button. Answers whether it went — a disabled one asks for no write, and a
    /// run counting it as a press would wait for one.
    function moveAll() {
        if (!moveAllButton.visible || !moveAllButton.enabled)
            return false
        moveAllButton.clicked()
        return true
    }

    implicitHeight: Theme.rowHeight
    color: Theme.bgElevated
    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceXs
        anchors.rightMargin: Theme.spaceXs
        spacing: Theme.spaceXs
        Label {
            text: bucketHeader.section === "staged"
                  ? qsTr("STAGED FILES")
                  : bucketHeader.section === "unstaged"
                  ? qsTr("UNSTAGED FILES")
                  : Words.badgeConflicts
            font.pixelSize: Theme.fontMd
            font.weight: Theme.fontWeightStrong
            color: bucketHeader.section === "conflicts" ? Theme.danger : Theme.textSecondary
        }
        // The count in its own label at the caption's step (デザイン規約 §タイポグラフィ). The conflicted bucket has
        // none — its rows are the number.
        Label {
            visible: bucketHeader.section !== "conflicts"
            text: "(" + bucketHeader.count + ")"
            font.pixelSize: Theme.fontMd
            color: Theme.textMuted
        }
        Item { Layout.fillWidth: true }
        // The tool's wait, in the whole-bucket button's seat (規約 §conflict を外部ツールへ渡す).
        Label {
            visible: bucketHeader.waitingForTool
            text: qsTr("Waiting for %1").arg(bucketHeader.worktree.mergeTool)
            font.pixelSize: Theme.fontMd
            color: Theme.textSecondary
        }
        SpinnerIcon {
            spinning: bucketHeader.waitingForTool
            Layout.preferredWidth: Theme.iconSm
            Layout.preferredHeight: Theme.iconSm
        }
        HoverToolButton {
            id: moveAllButton
            // Gives its seat to the tool's wait (規約 §conflict を外部ツールへ渡す); a press then would only queue
            // behind the tool.
            visible: !bucketHeader.waitingForTool
            // The heading stands even when empty, so the button says it cannot move anything (§無効).
            enabled: bucketHeader.count > 0
            // `git add` is a different act on a conflicted file, so the row's word carried up to the bucket
            // (規約 §diff の中のステージ).
            text: bucketHeader.section === "staged"
                  ? qsTr("Unstage all")
                  : bucketHeader.section === "conflicts"
                  ? qsTr("Mark all resolved")
                  : qsTr("Stage all")
            font.pixelSize: Theme.fontMd
            // As in NavHeader: a `ToolButton`'s own height is taller than the `rowHeight` band, and the whole row then
            // sinks through the band's floor.
            implicitHeight: Theme.iconLg
            tip: bucketHeader.section === "staged"
                 ? qsTr("Unstage everything")
                 : bucketHeader.section === "conflicts"
                 ? qsTr("Tell git every conflict has been dealt with")
                 : qsTr("Stage everything, untracked included")
            onClicked: {
                if (bucketHeader.section === "staged")
                    bucketHeader.repoTab.unstageAll()
                else if (bucketHeader.section === "conflicts")
                    // This band names this bucket alone; `stageAll()` would reach the unstaged one beside it.
                    bucketHeader.repoTab.stageConflicted()
                else
                    bucketHeader.repoTab.stageAll()
            }
        }
    }
}
