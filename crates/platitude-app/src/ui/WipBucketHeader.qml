pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// One bucket heading in the working-tree file list: what the bucket is called, how many files are in it, and the
// affordance that moves the whole bucket at once.
//
// One per bucket list, pinned above its rows (`WipBucketPane`) — so it stands whether or not the bucket has anything
// in it, and stays put when the bucket is scrolled (デザイン規約 §その他の操作).
Rectangle {
    id: bucketHeader

    /// Which bucket this heads: `conflicts` / `unstaged` / `staged`.
    required property string section
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
                  : qsTr("CONFLICTS")
            font.pixelSize: Theme.fontMd
            font.weight: Font.DemiBold
            color: bucketHeader.section === "conflicts" ? Theme.danger : Theme.textSecondary
        }
        // How many, in its own label at the caption's own step — the seat every heading band in the window keeps its
        // count in (NavHeader, CommandsPane). This is the shortest band a count stands in and the step still clears
        // its floor: measured on the 24px band, the brackets' ink runs y 82..96 with 6px of air above and 3px below,
        // and the first row of the list under it is bare. The conflicted bucket has no count — the rows under it are
        // the number.
        Label {
            visible: bucketHeader.section !== "conflicts"
            text: "(" + bucketHeader.count + ")"
            font.pixelSize: Theme.fontMd
            color: Theme.textMuted
        }
        Item { Layout.fillWidth: true }
        // The whole-bucket button's own seat, taken over for as long as the tool holds the queue. The words stay —
        // nothing else in view names the tool being waited on — and the ring says it is still running. No `…`: that is
        // the word for a question standing,
        // and progress is the ring's job (規約 §進行中・長押しの定数).
        //
        // Nothing to press: killing `git mergetool` would leave the editor it started running and its scratch behind.
        Label {
            visible: bucketHeader.waitingForTool
            text: qsTr("Waiting for %1").arg(bucketHeader.workTree.mergeTool)
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
            // The conflicted bucket has this seat too, and gives it up only when the wait for an external tool needs
            // it: the words are the one thing in view naming the tool, and they are longer than the button
            // (規約 §conflict を外部ツールへ渡す). Nothing is lost by standing down — that wait holds the write queue
            // until the tool is closed, so a press taken here would sit behind it with nothing to show.
            visible: !bucketHeader.waitingForTool
            // An empty bucket has nothing to move, and this heading stands even then (§無効 — what cannot be pressed says
            // so where it stands, rather than leaving its seat).
            enabled: bucketHeader.count > 0
            // The same `git add` is a different act on a conflicted file, so the word is the row's own word carried up
            // to the bucket's scope (規約 §diff の中のステージ — the ending names the scope).
            text: bucketHeader.section === "staged"
                  ? qsTr("Unstage all")
                  : bucketHeader.section === "conflicts"
                  ? qsTr("Mark all resolved")
                  : qsTr("Stage all")
            font.pixelSize: Theme.fontMd
            // The seat every button in a `rowHeight` band takes (NavHeader's do the same). A `ToolButton` asks for its
            // word plus its own padding, and at the body step that came to more than the band it stands in — the
            // layout then placed the whole row against a height nobody could see, and the words went down with it
            // until the count's brackets were through the floor (measured, both OSes).
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
                    // Not `stageAll()`: the unstaged bucket beside this one is not part of what this band names.
                    bucketHeader.repoTab.stageConflicted()
                else
                    bucketHeader.repoTab.stageAll()
            }
        }
    }
}
